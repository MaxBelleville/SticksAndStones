mod allocators;
mod brick;
mod clipmap;
mod device;
mod index;
mod map;
mod material;
use bevy_math::*;

use std::collections::{HashMap, hash_map::Entry};

/*
    What is a brick hub?
    Brickhub is a spatial hashed container of brickmaps + material buckets, where each brickmap contains 8x8x8 bricks.
    And also each brick contains 8x8x8 voxels as well as lod masks.
    Work based losely on: https://nva.sikt.no/registration/0198ea9104d9-15642a8d-7b03-45bc-9463-aaa920c8dd4b
    Code for paper: https://github.com/Avokadoen/zig_vulkan
    and super cell code: https://github.com/stijnherfst/BrickMap/tree/master
*/

// --- Constants (Adjust these to match your headers) ---
pub const BRICK_SIZE: usize = 8;
pub const VOXELS_PER_BRICK: usize = BRICK_SIZE * BRICK_SIZE * BRICK_SIZE; // 512
pub const MATERIAL_BUCKET_SIZE: usize = VOXELS_PER_BRICK;

pub const BRICKHUB_CELL_SIZE: usize = 8; // bricks per axis
pub const BRICKS_PER_CELL: usize = BRICKHUB_CELL_SIZE * BRICKHUB_CELL_SIZE * BRICKHUB_CELL_SIZE;
pub const INVALID_BRICK: u32 = u32::MAX;
use sns_core::prelude::GpuDevice;
// --- Core Structures ---
#[derive(Default)]
pub struct BrickHubCell {
    coord: Vec3,
    map: map::BrickMap,
    device_state: device::CellDeviceState,
    last_touched_frame: u64,
    dirty: bool,
}
impl BrickHubCell {
    pub fn touch(&mut self, frame: u64) {
        self.last_touched_frame = frame;
    }

    pub fn insert(&mut self, voxel: UVec3, material: u16, frame: u64) {
        self.touch(frame);

        let result = self.map.insert(voxel, material);
        if result.is_brick_new() {
            self.device_state
                .get_index_delta_mut()
                .register(result.get_brick_index());
        }
        self.device_state
            .get_brick_delta_mut()
            .register(result.get_brick_index());

        if let Some(base) = result.get_mat_base_allocated() {
            self.device_state
                .get_material_delta_mut()
                .register_range(base as usize, base as usize + VOXELS_PER_BRICK);
        }

        self.device_state
            .get_material_delta_mut()
            .register(result.get_mat_index());

        self.dirty = true;
    }
    pub fn remove(&mut self, voxel: UVec3, frame: u64) {
        self.touch(frame);

        let Some(result) = self.map.remove(voxel) else {
            return;
        };

        self.device_state
            .get_brick_delta_mut()
            .register(result.get_brick_index());

        self.device_state
            .get_material_delta_mut()
            .register(result.get_mat_index());

        if result.is_brick_empty() {
            self.device_state
                .get_index_delta_mut()
                .register(result.get_brick_index());
        }

        self.dirty = true;
    }

    pub fn clear(&mut self, frame: u64) {
        self.touch(frame);

        self.map.clear();
        self.device_state.get_brick_delta_mut().reset();
        self.device_state.get_material_delta_mut().reset();
        self.device_state.get_index_delta_mut().reset();
        self.dirty = true;
    }
}

#[derive(Default)]
pub struct BrickHub {
    cells: HashMap<IVec3, BrickHubCell>,
    device: device::HubDevice,
    device_queue: allocators::HubDeviceQueue,
    allocator: allocators::HubGpuAllocator,
}

impl BrickHub {
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
            device: device::HubDevice::default(),
            device_queue: allocators::HubDeviceQueue::default(),
            allocator: allocators::HubGpuAllocator::default(),
        }
    }
    /// Get clipmap rings, evict unnecessary cells, admit new cells if not already found.
    pub fn get_or_admit_cell(
        &mut self,
        coord: IVec3,
        view_cell: IVec3,
        frame: u64,
        gpu: &dyn GpuDevice,
    ) -> Option<&mut BrickHubCell> {
        // classify ring

        let ring = clipmap::classify_ring(coord, view_cell)?;
        // check residency
        let already_resident = self.cells.contains_key(&coord);

        // admission pressure handling

        if !already_resident {
            // Try compaction first
            if self.ring_full(ring) {
                let compacted = self.compact_ring_if_needed(gpu, ring);
                if compacted {
                    debug_assert!(!self.ring_full(ring));
                }
            }

            // Evict if still full
            while self.ring_full(ring) {
                if let Some(evict) = self.select_eviction_candidate(frame, ring) {
                    self.unload_cell(evict);
                } else {
                    return None; // hard-full ring
                }
            }

            // Admit CPU-side cell
            self.cells.insert(
                coord,
                BrickHubCell {
                    coord: coord.as_vec3(),
                    last_touched_frame: frame,
                    dirty: true,
                    ..Default::default()
                },
            );

            // Admit GPU-side allocation
            self.allocator.admit_cell(coord, ring);
        }

        let cell = self.cells.get_mut(&coord).expect("cell must exist");
        cell.touch(frame);
        Some(cell)
    }
    /// Unload a cell and free its GPU ranges.
    pub fn unload_cell(&mut self, coord: IVec3) {
        if let Some(_cell) = self.cells.remove(&coord) {
            if let Some(alloc) = self.allocator.remove(&coord) {
                self.allocator.free_cell_allocation(alloc);
            }
            self.device_queue.remove_cell_updates(coord);
        }
    }

    /// Checks if a ring is full (based on current live cells in allocator)
    pub fn ring_full(&self, ring: u8) -> bool {
        let ring_cfg = &clipmap::CLIPMAP_RINGS[ring as usize];
        let live_in_ring = self
            .allocator
            .values()
            .filter(|a| a.get_ring() == ring)
            .count();
        live_in_ring >= ring_cfg.max_cells
    }

    /// Select eviction candidate in a ring based on LRU
    pub fn select_eviction_candidate(&self, frame: u64, ring: u8) -> Option<IVec3> {
        self.cells
            .iter()
            .filter(|(coord, _)| {
                self.allocator
                    .get(**coord)
                    .map_or(false, |a| a.get_ring() == ring)
            })
            .filter(|(_, cell)| !cell.dirty) // only evict clean cells
            .min_by_key(|(_, cell)| frame - cell.last_touched_frame)
            .map(|(coord, _)| *coord)
    }
    /// Aggregate all dirty cells for both rings and whole device
    fn aggregate_device_queue_internal(&mut self, ring_filter: Option<u8>) {
        self.device_queue.clear();

        for (coord, cell) in self.cells.iter_mut() {
            if !cell.dirty {
                continue;
            }

            if let Some(ring) = ring_filter {
                if self
                    .allocator
                    .get(*coord)
                    .map_or(true, |a| a.get_ring() != ring)
                {
                    continue;
                }
            }

            self.device_queue
                .aggregate_cell(*coord, &mut cell.device_state);

            cell.dirty = false;
        }
    }

    /// Aggregate all dirty cells
    pub fn aggregate_device_queue(&mut self) {
        self.aggregate_device_queue_internal(None);
    }

    /// Aggregate only dirty cells in a specific ring
    pub fn aggregate_device_queue_by_ring(&mut self, ring: u8) {
        self.aggregate_device_queue_internal(Some(ring));
    }

    // Generic flush helper, accepts optional ring filter
    fn flush_queue_internal(&mut self, gpu: &dyn GpuDevice, ring_filter: Option<u8>) {
        // Aggregate first
        if let Some(ring) = ring_filter {
            self.aggregate_device_queue_by_ring(ring);
        } else {
            self.aggregate_device_queue();
        }

        // Ensure GPU buffers are large enough
        self.device.ensure_capacity(
            gpu,
            self.allocator.brick_capacity(),
            self.allocator.material_capacity(),
            self.allocator.index_capacity(),
        );

        // Upload only relevant updates (the device queue is already filtered)
        self.upload_bricks(gpu);
        self.upload_materials(gpu);
        self.upload_indices(gpu);
    }

    /// Flush the entire device queue
    pub fn flush_queue_to_device(&mut self, gpu: &dyn GpuDevice) {
        self.flush_queue_internal(gpu, None);
    }

    /// Flush only cells belonging to a specific ring
    pub fn flush_ring_to_device(&mut self, gpu: &dyn GpuDevice, ring: u8) {
        self.flush_queue_internal(gpu, Some(ring));
    }

    pub fn compact_ring_if_needed(&mut self, gpu: &dyn GpuDevice, ring: u8) -> bool {
        // 1. Detect fragmentation
        if !self.allocator.is_fragmented(ring) {
            return false;
        }

        // 2. Select candidates (LRU or lowest-cost first)
        let mut candidates = self.allocator.select_compaction_candidates(ring);

        if candidates.is_empty() {
            return false;
        }

        // 3. Allocate new ranges
        self.allocator.allocate_new_ranges(&mut candidates);

        // 4. Move GPU data (CPU → GPU reupload for now)
        self.move_gpu_data(gpu, &candidates);

        // 5. Finalize allocator state
        self.allocator.finalize_compaction(&candidates);

        // 6. Invalidate device queue + mark dirty
        for cand in &candidates {
            self.device_queue.remove_cell_updates(cand.get_coord());
            if let Some(cell) = self.cells.get_mut(&cand.get_coord()) {
                cell.dirty = true;
            }
        }

        true
    }

    pub fn move_gpu_data(
        &mut self,
        gpu: &dyn GpuDevice,
        candidates: &[allocators::CompactionCandidate],
    ) {
        for cand in candidates {
            if let Some(new_alloc) = cand.get_new_alloc() {
                let old = cand.get_old_alloc();
                // Upload bricks
                let bricks = &self.cells[&cand.get_coord()]
                    .map
                    .get_brick_allocator()
                    .slice(0, BRICKS_PER_CELL);

                self.device
                    .bricks
                    .upload_range(gpu, new_alloc.get_brick_base(), bricks);

                // Upload materials
                let mats = &self.cells[&cand.get_coord()]
                    .map
                    .get_material_allocator()
                    .slice(0, BRICKS_PER_CELL * VOXELS_PER_BRICK);
                self.device
                    .materials
                    .upload_range(gpu, new_alloc.get_material_base(), mats);

                // Upload indices
                let indices = &self.cells[&cand.get_coord()]
                    .map
                    .get_index_grid()
                    .slice(0, BRICKS_PER_CELL);
                self.device
                    .indices
                    .upload_range(gpu, new_alloc.get_index_base(), indices);
            }
        }
    }

    pub fn upload_bricks(&mut self, gpu: &dyn GpuDevice) {
        for upload in self.device_queue.get_brick_updates() {
            let Some(alloc) = self.allocator.get(upload.get_cell_coord()) else {
                continue;
            };

            let global_index = alloc.get_brick_base() + upload.get_brick_index() as usize;

            let brick = self.cells[&upload.get_cell_coord()]
                .map
                .get_brick_allocator()
                .get(upload.get_brick_index() as usize);

            self.device
                .bricks
                .upload_range(gpu, global_index, std::slice::from_ref(brick));
        }
    }

    pub fn upload_materials(&mut self, gpu: &dyn GpuDevice) {
        for upload in self.device_queue.get_material_updates() {
            let Some(alloc) = self.allocator.get(upload.get_cell_coord()) else {
                continue;
            };

            let global_base = alloc.get_material_base() + upload.get_base() as usize;

            let data = self.cells[&upload.get_cell_coord()]
                .map
                .get_material_allocator()
                .slice(upload.get_base() as usize, upload.get_count() as usize);

            self.device.materials.upload_range(gpu, global_base, data);
        }
    }

    pub fn upload_indices(&mut self, gpu: &dyn GpuDevice) {
        for upload in self.device_queue.get_index_updates().iter() {
            let Some(alloc) = self.allocator.get(upload.get_cell_coord()) else {
                continue;
            };

            let brick_index = self.cells[&upload.get_cell_coord()]
                .map
                .get_index_grid()
                .raw_index(upload.get_index());

            let global_index = alloc.get_index_base() + upload.get_index() as usize;

            self.device
                .indices
                .upload_range(gpu, global_index, std::slice::from_ref(&brick_index));
        }
    }
}

#[inline]
pub fn voxel_in_brick(voxel: UVec3) -> u32 {
    let bx = voxel.x & (BRICK_SIZE as u32 - 1);
    let by = voxel.y & (BRICK_SIZE as u32 - 1);
    let bz = voxel.z & (BRICK_SIZE as u32 - 1);
    bx + BRICK_SIZE as u32 * (bz + BRICK_SIZE as u32 * by)
}
#[inline]
pub fn index_to_brick_coord(index: u32) -> UVec3 {
    let x = index % BRICKHUB_CELL_SIZE as u32;
    let y = (index / BRICKHUB_CELL_SIZE as u32) % BRICKHUB_CELL_SIZE as u32;
    let z = index / (BRICKHUB_CELL_SIZE * BRICKHUB_CELL_SIZE) as u32;
    UVec3::new(x, y, z)
}
