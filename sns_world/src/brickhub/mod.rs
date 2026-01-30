mod brick;
mod device;
mod index;
mod map;
mod material;
use bevy_math::*;

use std::collections::HashMap;

// --- Constants (Adjust these to match your headers) ---
pub const BRICK_SIZE: usize = 8;
pub const VOXELS_PER_BRICK: usize = BRICK_SIZE * BRICK_SIZE * BRICK_SIZE; // 512
pub const MATERIAL_BUCKET_SIZE: usize = VOXELS_PER_BRICK;

pub const BRICKHUB_CELL_SIZE: usize = 8; // bricks per axis
pub const BRICKS_PER_CELL: usize = BRICKHUB_CELL_SIZE * BRICKHUB_CELL_SIZE * BRICKHUB_CELL_SIZE;
pub const INVALID_BRICK: u32 = u32::MAX;

// --- Core Structures ---
#[derive(Default)]
pub struct BrickHubCell {
    coord: Vec3,
    
    //TODO: cache a local sample of terrain and climate data at this location

    map: map::BrickMap,
    device_state: device::CellDeviceState,
    dirty: bool,
}
impl BrickHubCell {
    pub fn insert(&mut self, voxel: UVec3, material: u16) {
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
    pub fn remove(&mut self, voxel: UVec3) {
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

    pub fn clear(&mut self) {
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
    device_queue: device::HubDeviceQueue,
}

impl BrickHub {
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
            device: device::HubDevice::default(),
            device_queue: device::HubDeviceQueue::default(),
        }
    }

    pub fn get_or_create_cell(&mut self, coord: IVec3) -> &mut BrickHubCell {
        self.cells
            .entry(coord)
            .or_insert_with(|| BrickHubCell::default())
    }

    pub fn unload_cell(&mut self, coord: IVec3) {
        self.cells.remove(&coord);
    }
}
