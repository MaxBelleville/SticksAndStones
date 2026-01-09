use std::sync::atomic::{AtomicU32, Ordering};

use bevy_math::*;

use super::material::MaterialAllocator;
use super::state::{
    AtomicCount, VOXELS_PER_BRICK, BRICK_BYTES, BRICK_DIM, BrickStatusMask, Device, DeviceDataDelta,
    IndexType, StartIndex, State, Status, UNSET_INDEX, UNALLOCATED_INDEX
};

pub struct Config {
    pub brick_alloc: Option<usize>,
    pub base_t: f32,
    pub min_point: Vec3,
    pub scale: f32,
    pub workers_count: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            brick_alloc: None,
            base_t: 0.01,
            min_point: Vec3::ZERO,
            scale: 1.0,
            workers_count: 4,
        }
    }
}

pub struct BrickGrid {
    pub state: Box<State>,
    pub material_allocator: Box<MaterialAllocator>,
}

impl BrickGrid {
    pub fn new(dim: UVec3, config: Config) -> Self {
        assert!(config.workers_count > 0);
        let brick_count = dim.x * dim.y * dim.z;
        assert!(brick_count > 0);

        // 1. Initialize Buffers
        let status_mask_count = ((brick_count + 31) / 32) as usize; // divCeil logic
        let mut brick_statuses = Vec::with_capacity(status_mask_count);
        for _ in 0..status_mask_count {
            brick_statuses.push(BrickStatusMask::default());
        }
        let grid_to_brick: Vec<AtomicU32> = (0..brick_count)
        .map(|_| AtomicU32::new(UNALLOCATED_INDEX))
        .collect();
        let brick_alloc_count = config.brick_alloc.unwrap_or(brick_count as usize);

        let mut brick_occupancy = vec![0u8; brick_alloc_count * BRICK_BYTES as usize];
        let mut brick_material_base = vec![UNSET_INDEX; brick_alloc_count];

        let material_index_count = brick_alloc_count * VOXELS_PER_BRICK as usize;
        let mut material_indices = vec![0u8; material_index_count];

        // 2. Calculate Spatial Constants
        let mut min_point_base = Vec4::ZERO;

        min_point_base = config.min_point.extend(config.base_t);

        let max_point_scale = (config.min_point + dim.as_vec3() * config.scale).extend(config.scale);
 

        let work_segment_size =
            (dim.x + (config.workers_count as u32) - 1) / (config.workers_count as u32);

        // 3. Construct State and Allocator
        let device_state = Device {
            voxel_dim: dim * BRICK_DIM,
            dim,
            padding: (0,0),
            min_point_base,
            max_point_scale,
        };

        let state = Box::new(State {
            brick_statuses,
            brick_statuses_delta: DeviceDataDelta::default(),
            grid_to_brick,
            grid_to_brick_delta: DeviceDataDelta::default(),
            bricks_occupancy_delta: DeviceDataDelta::default(),
            brick_occupancy,
            brick_material_base_delta: DeviceDataDelta::default(),
            brick_material_base,
            material_indices_delta: DeviceDataDelta::default(),
            material_indices,
            active_bricks: AtomicCount::new(0),
            device_state,
            work_segment_size: work_segment_size as usize,
        });

        let material_allocator = Box::new(MaterialAllocator::new(material_index_count));

        Self {
            state,
            material_allocator,
        }
    }

    pub fn insert(&self, x: u32, y: u32, z: u32, material_index: u8) {
    let dev = &self.state.device_state;
    let flipped_y = dev.voxel_dim.y - 1 - y;

    let grid_idx = brick_grid_index(dev, UVec3::new(x, flipped_y, z));
    
    // 1. Atomic Load of the current brick index
    let mut brick_idx = self.state.grid_to_brick[grid_idx].load(Ordering::Acquire);

    // 2. If unallocated, attempt to claim a new brick
    if brick_idx == UNALLOCATED_INDEX {
        let new_idx = self.state.active_bricks.fetch_add(1, Ordering::Relaxed);
        
        // Atomically attempt to swap UNALLOCATED with our new_idx
        match self.state.grid_to_brick[grid_idx].compare_exchange(
            UNALLOCATED_INDEX,
            new_idx,
            Ordering::Release,
            Ordering::Acquire,
        ) {
            Ok(_) => {
                brick_idx = new_idx; // Success!
            }
            Err(actual_idx) => {
                brick_idx = actual_idx; // Someone else won the race
                // Optional: In a real system, you'd return 'new_idx' to a pool
            }
        }
    }

    // 3. Now that we have a safe, unique brick_idx, we update data
    let nth_bit = voxel_at_brick(UVec3::new(x, flipped_y, z));
    
    // ... Update Material Indices and Occupancy ...
    // Note: Since brick_idx is now unique to this grid coordinate, 
    // writes to brick_occupancy[brick_idx] are safe from other grid coordinates.
    
    // 4. Update the status mask atomically
    let status_idx = grid_idx / 32;
    let status_offset = (grid_idx % 32) as u8;
    self.state.brick_statuses[status_idx].set_loaded(status_offset);
    
    // 5. Track deltas
    self.state.grid_to_brick_delta.register_delta(grid_idx);
    self.state.brick_statuses_delta.register_delta(status_idx);
}
}

// --- Helpers ---

fn voxel_at_brick(coord: UVec3) -> u8 {
    let r = coord % BRICK_DIM;
    (r.x + BRICK_DIM * (r.z + BRICK_DIM * r.y)) as u8
}

fn brick_grid_index(dev: &Device, coord: UVec3) -> usize {
    let g = coord / BRICK_DIM;
    (g.x + dev.dim.x * (g.z + dev.dim.z * g.y)) as usize
}

fn count_bits(bits: [u8; BRICK_BYTES as usize], range_to: u32) -> u32 {
    let val = u64::from_le_bytes(bits);
    // Use a mask to only count bits up to range_to
    let mask = if range_to >= 64 {
        !0
    } else {
        (1u64 << range_to) - 1
    };
    (val & mask).count_ones()
}
