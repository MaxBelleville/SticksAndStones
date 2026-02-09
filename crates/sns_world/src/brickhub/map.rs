use bevy_math::*;
use sns_core::prelude::LinearAllocator;

use super::{
    BRICK_SIZE, BRICKHUB_CELL_SIZE, BRICKS_PER_CELL, VOXELS_PER_BRICK, brick, index, material,
};
pub struct BrickMap {
    /// Local brick storage
    brick_allocator: LinearAllocator<brick::Brick>,

    /// Local material storage
    material_allocator: material::MaterialAllocator,

    /// Extended brickmap index grid
    index_grid: index::IndexGrid,
    brick_status_grid: brick::BrickStatusGrid,

    active_bricks: u32,
}

impl Default for BrickMap {
    fn default() -> Self {
        let brick_capacity = BRICKS_PER_CELL;

        let brick_allocator = LinearAllocator::new(brick_capacity);
        let material_allocator =
            material::MaterialAllocator::new(brick_capacity * VOXELS_PER_BRICK);

        let index_grid = index::IndexGrid::default();
        let brick_status_grid = brick::BrickStatusGrid::new(brick_capacity);

        Self {
            brick_allocator,
            material_allocator,
            index_grid,
            brick_status_grid,
            active_bricks: 0,
        }
    }
}
impl BrickMap {
    pub fn insert(&mut self, voxel: UVec3, material: u16) -> BrickChange {
        debug_assert!(voxel.x < (BRICKHUB_CELL_SIZE * BRICK_SIZE) as u32);
        debug_assert!(voxel.y < (BRICKHUB_CELL_SIZE * BRICK_SIZE) as u32);
        debug_assert!(voxel.z < (BRICKHUB_CELL_SIZE * BRICK_SIZE) as u32);

        let flipped_y = (BRICKHUB_CELL_SIZE * BRICK_SIZE) as u32 - 1 - voxel.y;
        let brick_coord = UVec3::new(voxel.x, flipped_y, voxel.z) / BRICK_SIZE as u32;

        let (brick_index, brick_was_new) = match self.index_grid.get(brick_coord) {
            Some(idx) => (idx as usize, false),
            None => {
                // Allocate new brick
                let idx = self.brick_allocator.allocate(brick::Brick::default()) as usize;

                self.index_grid.set(brick_coord, idx as u32);
                self.brick_status_grid.set_active(idx);
                (idx, true)
            }
        };

        let brick = self.brick_allocator.get_mut(brick_index);
        if brick.is_empty() {
            self.active_bricks += 1;
        }

        // --- Compute voxel index within brick ---
        let voxel_index = voxel_in_brick(UVec3::new(voxel.x, flipped_y, voxel.z)) as usize;

        // --- Ensure material start index ---
        let material_base_allocated = if brick.get_start_index().is_unset() {
            let base = self.material_allocator.alloc_next_block();
            brick.set_start_index(base, index::IndexType::VoxelStartIndex);
            Some(base)
        } else {
            None
        };
        let material_id: u8 = self.material_allocator.write_shared_mat(material);
        let voxel_offset = brick.get_start_index().value() as usize + voxel_index;
        self.material_allocator.write_index(voxel_offset, material_id);


        let word = voxel_index >> 6;
        let bit = voxel_index & 63;
        brick.set_occupancy(word, 1u64 << bit);
        brick.set_lod(voxel);

        BrickChange {
            brick_index,
            brick_was_new,
            brick_became_empty: false,
            material_base_allocated,
            material_index: material_id as usize,
        }
    }

    pub fn remove(&mut self, voxel: UVec3) -> Option<BrickChange> {
        let flipped_y = (BRICKHUB_CELL_SIZE * BRICK_SIZE) as u32 - 1 - voxel.y;
        let brick_coord = UVec3::new(voxel.x, flipped_y, voxel.z) / BRICK_SIZE as u32;

        let brick_index = self.index_grid.get(brick_coord)? as usize;
        let brick = self.brick_allocator.get_mut(brick_index);

        let voxel_index = voxel_in_brick(UVec3::new(voxel.x, flipped_y, voxel.z)) as usize;

        let word = voxel_index >> 6;
        let bit = voxel_index & 63;

        if !brick.test_occupancy(word, bit) {
            return None; // already empty
        }

        let was_only_voxel = brick.occupied_voxel_count() == 1;

        brick.clear_occupancy(word, bit);
        brick.clear_lod(voxel);

        let material_index = brick.get_start_index().value() as usize + voxel_index;
        self.material_allocator.clear_id(material_index);

        if was_only_voxel {
            self.index_grid.clear(brick_coord);
            self.brick_status_grid.set_inactive(brick_index);
            self.active_bricks -= 1;
        }

        Some(BrickChange {
            brick_index,
            brick_was_new: false,
            brick_became_empty: was_only_voxel,
            material_base_allocated: None,
            material_index,
        })
    }

    pub fn clear(&mut self) {
        self.brick_allocator.clear();
        self.material_allocator = material::MaterialAllocator::new(BRICKS_PER_CELL);
        self.index_grid = index::IndexGrid::default();
        self.brick_status_grid = brick::BrickStatusGrid::default();
        self.active_bricks = 0;
    }
}

pub struct BrickChange {
    brick_index: usize,
    brick_was_new: bool,
    brick_became_empty: bool,
    material_base_allocated: Option<u32>,
    material_index: usize,
}

impl BrickChange {
    pub fn get_brick_index(&self) -> usize {
        self.brick_index
    }
    pub fn is_brick_new(&self) -> bool {
        self.brick_was_new
    }
    pub fn is_brick_empty(&self) -> bool {
        self.brick_became_empty
    }
    pub fn get_mat_base_allocated(&self) -> Option<u32> {
        self.material_base_allocated
    }
    pub fn get_mat_index(&self) -> usize {
        self.material_index
    }
}
#[inline]
pub fn voxel_in_brick(voxel: UVec3) -> u32 {
    let bx = voxel.x & (BRICK_SIZE as u32 - 1);
    let by = voxel.y & (BRICK_SIZE as u32 - 1);
    let bz = voxel.z & (BRICK_SIZE as u32 - 1);
    bx + BRICK_SIZE as u32 * (bz + BRICK_SIZE as u32 * by)
}
