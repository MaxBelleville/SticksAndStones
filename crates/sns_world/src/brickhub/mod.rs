mod allocators;
mod brick;
mod clipmap;
mod device;
mod index;
mod map;
mod material;
mod scheduler;

use std::collections::HashMap;

use bevy_math::*;
use sns_core::prelude::{GpuDevice, Range};

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

pub const BRICKMAP_SIZE: usize = 8; // bricks per axis
pub const BRICKS_PER_BRICKMAP: usize = BRICKMAP_SIZE * BRICKMAP_SIZE * BRICKMAP_SIZE;
pub const INVALID_BRICK: u32 = u32::MAX;

pub type CellID = IVec3;
pub type MapID = IVec3;
pub type BrickID = UVec3;
pub type MaterialID = u16;

pub type MaterialIndex = u16;
pub type BrickIndex = u32;

// --- Core Structures ---
#[derive(Default)]
pub struct BrickHubCell {
    map: map::BrickMap,
    delta: allocators::CellDelta,
}
impl BrickHubCell {
    pub fn insert(&mut self, voxel: BrickID, material: MaterialID) {
        let events = self.map.insert(voxel, material);
        for event in events {
            self.apply_event(event);
        }
    }
    pub fn remove(&mut self, voxel: BrickID) {
        let events = self.map.remove(voxel);

        for event in events {
            self.apply_event(event);
        }
    }
    fn apply_event(&mut self, event: map::BrickEditEvent) {
        match event {
            map::BrickEditEvent::BrickAllocated { brick_index } => {
                self.delta
                    .register(device::StorageType::Indices, brick_index);
                self.delta.register(device::StorageType::Brick, brick_index);
            }

            map::BrickEditEvent::MaterialBlockAllocated { base } => {
                self.delta.register_range(
                    device::StorageType::Material,
                    base as usize,
                    base as usize + VOXELS_PER_BRICK,
                );
            }

            map::BrickEditEvent::VoxelMaterialChanged { index } => {
                self.delta.register(device::StorageType::Material, index);
            }

            map::BrickEditEvent::BrickBecameEmpty { brick_index } => {
                self.delta.register(device::StorageType::Brick, brick_index);
                self.delta
                    .register(device::StorageType::Indices, brick_index);
            }
        }
    }
    pub fn clear_delta(&mut self) {
        self.delta.clear();
    }

    pub fn get_delta_mut(&mut self) -> &mut allocators::CellDelta {
        &mut self.delta
    }
    pub fn get_brickmap(&self) -> &map::BrickMap {
        &self.map
    }
}

#[derive(Default)]
pub struct BrickHub {
    cells: HashMap<IVec3, BrickHubCell>,
    buffers: device::Buffers,
    allocator: allocators::DeviceAllocator,
    scheduler: scheduler::DeviceScheduler,
    residency: clipmap::ResidencyManager,
}

impl BrickHub {
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
            buffers: device::Buffers::default(),
            allocator: allocators::DeviceAllocator::default(),
            scheduler: scheduler::DeviceScheduler::default(),
            residency: clipmap::ResidencyManager::default(),
        }
    }

    pub fn get_or_admit_cell(
        &mut self,
        id: CellID,
        view_cell: CellID,
        frame: u64,
    ) -> Option<&mut BrickHubCell> {
        let ring = clipmap::classify_ring(id, view_cell)?;

        if !self.cells.contains_key(&id) {
            match self.residency.admit(id, ring, frame) {
                clipmap::ResidencyDecision::Admit => {
                    self.cells.insert(id, BrickHubCell::default());
                    self.allocator.admit_cell(id, ring);
                }

                clipmap::ResidencyDecision::EvictAndAdmit(evict_id) => {
                    self.unload_cell(evict_id);
                    self.cells.insert(id, BrickHubCell::default());
                    self.allocator.admit_cell(id, ring);
                }

                clipmap::ResidencyDecision::Reject => {
                    return None;
                }
            }
        } else {
            self.residency.touch(id, ring, frame);
        }

        self.cells.get_mut(&id)
    }

    /// Unload a cell and free its GPU ranges.
    pub fn unload_cell(&mut self, id: CellID) {
        let alloc = self
            .allocator
            .remove(id)
            .expect("allocator and cell map out of sync");

        let ring = alloc.get_ring();

        self.allocator.free_cell_allocation(alloc);

        self.residency.remove(id, ring);

        self.cells
            .remove(&id)
            .expect("allocator and cell map out of sync");
    }

    
    pub fn flush(&mut self, gpu: &dyn GpuDevice) {
        // ---- Phase 1: Collect CPU deltas
        let batch = self.collect_device_batch();

        // ---- Phase 2: Compaction
        if let Some(candidates) = self.reserve_compaction_if_needed() {
            self.schedule_gpu_migrations(&candidates);

            self.scheduler.resolve_migrations(&self.cells);

            self.scheduler.execute_uploads(gpu, &mut self.buffers);

            self.allocator.finalize_compaction(&candidates);
        }

        // ---- Phase 3: Upload regular deltas
        self.scheduler
            .flush_deltas(batch, &self.cells, &self.allocator, &mut self.buffers, gpu);
    }


    fn collect_device_batch(&mut self) -> device::DeltaBatch {
        let mut batch = device::DeltaBatch::default();

        for  (&id,cell) in self.cells.iter_mut() {
            batch.take_and_push(id, cell.get_delta_mut());
        }

        batch
    }

    fn schedule_gpu_migrations(&mut self, candidates: &[allocators::CompactionCandidate]) {
        for cand in candidates {
            if let Some(new_alloc) = cand.get_new_alloc() {
                let id = cand.get_id();

                self.scheduler.enqueue(scheduler::GpuMigrationTask {
                    id,
                    old_alloc: cand.get_old_alloc().clone(),
                    new_alloc: new_alloc.clone(),
                });
            }
        }
    }

    fn reserve_compaction_if_needed(&mut self) -> Option<Vec<allocators::CompactionCandidate>> {
        // Choose a ring to compact (policy decision)
        let ring = 0; // example policy

        if !self.allocator.is_fragmented_for(BRICKS_PER_BRICKMAP) {
            return None;
        }

        let candidates = self.allocator.select_compaction_candidates(ring);

        if candidates.is_empty() {
            return None;
        }

        let mut candidates = candidates;

        // Reserve new ranges
        self.allocator.allocate_new_ranges(&mut candidates);

        Some(candidates)
    }
}

#[inline]
pub fn voxel_in_brick(voxel: BrickID) -> BrickIndex {
    let bx = voxel.x & (BRICK_SIZE as u32 - 1);
    let by = voxel.y & (BRICK_SIZE as u32 - 1);
    let bz = voxel.z & (BRICK_SIZE as u32 - 1);
    bx + BRICK_SIZE as u32 * (bz + BRICK_SIZE as u32 * by)
}
#[inline]
pub fn brick_index_to_voxel(index: BrickIndex) -> BrickID {
    let x = index % BRICKMAP_SIZE as u32;
    let y = (index / BRICKMAP_SIZE as u32) % BRICKMAP_SIZE as u32;
    let z = index / (BRICKMAP_SIZE * BRICKMAP_SIZE) as u32;
    UVec3::new(x, y, z)
}
