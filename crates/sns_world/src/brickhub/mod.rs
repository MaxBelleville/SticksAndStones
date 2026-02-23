mod allocators;
mod brick;
mod clipmap;
mod device;
mod index;
mod map;
mod material;
use bevy_math::*;

use std::collections::HashMap;

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
    cell_state: device::CellDeltaGroup,
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
                self.cell_state
                    .register::<device::IndicesStream>(brick_index);
                self.cell_state.register::<device::BrickStream>(brick_index);
            }

            map::BrickEditEvent::MaterialBlockAllocated { base } => {
                self.cell_state.register_range::<device::MaterialStream>(
                    base as usize,
                    base as usize + VOXELS_PER_BRICK,
                );
            }

            map::BrickEditEvent::VoxelMaterialChanged { index } => {
                self.cell_state.register::<device::MaterialStream>(index);
            }

            map::BrickEditEvent::BrickBecameEmpty { brick_index } => {
                self.cell_state.register::<device::BrickStream>(brick_index);
                self.cell_state
                    .register::<device::IndicesStream>(brick_index);
            }
        }
    }
}

#[derive(Default)]
pub struct BrickHub {
    cells: HashMap<IVec3, BrickHubCell>,
    device: device::DeviceBuffers,
    allocator: allocators::DeviceAllocator,
}

impl BrickHub {
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
            device: device::DeviceBuffers::default(),
            allocator: allocators::DeviceAllocator::default(),
        }
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
