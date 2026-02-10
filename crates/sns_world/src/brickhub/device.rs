use super::brick;
use bevy_math::*;
use sns_core::prelude::*;

#[derive(Clone, Copy, Default)]
pub struct HubDevice {
    pub bricks: GpuBuffer<brick::Brick>,
    pub materials: GpuBuffer<u16>,
    pub indices: GpuBuffer<u32>,
}
impl HubDevice {
    pub fn ensure_capacity(
        &mut self,
        gpu: &dyn GpuDevice,
        brick_cap: usize,
        material_cap: usize,
        index_cap: usize,
    ) {
        if self.bricks.capacity() < brick_cap {
            self.bricks.grow(gpu, brick_cap.next_power_of_two(), true);
        }

        if self.materials.capacity() < material_cap {
            self.materials.grow(gpu, material_cap.next_power_of_two(), true);
        }

        if self.indices.capacity() < index_cap {
            self.indices.grow(gpu, index_cap.next_power_of_two(), true);
        }
    }
}


#[derive(Clone, Copy, PartialEq)]
pub enum DeltaState {
    Invalid,
    Inactive,
    Active,
}

pub struct DeviceDataDelta {
    state: DeltaState,
    range: (usize, usize), // (from, to)
}
impl Default for DeviceDataDelta {
    fn default() -> Self {
        Self {
            state: DeltaState::Inactive,
            range: (usize::MAX, 0),
        }
    }
}

impl DeviceDataDelta {
    pub fn reset(&mut self) {
        self.state = DeltaState::Inactive;
        self.range = (usize::MAX, 0);
    }

    pub fn register(&mut self, index: usize) {
        self.state = DeltaState::Active;
        self.range = (self.range.0.min(index), self.range.1.max(index + 1));
    }

    pub fn register_range(&mut self, from: usize, to: usize) {
        self.state = DeltaState::Active;
        self.range = (self.range.0.min(from), self.range.1.max(to));
    }
    pub fn is_active(&self) -> bool {
        self.state == DeltaState::Active
    }
    pub fn get_range(&self) -> (usize, usize) {
        self.range
    }
}
#[derive(Default)]
pub struct CellDeviceState {
    // CPU → GPU deltas
    brick_delta: DeviceDataDelta,
    material_delta: DeviceDataDelta,
    index_delta: DeviceDataDelta,
}
impl CellDeviceState {
    pub fn get_brick_delta_mut(&mut self) -> &mut DeviceDataDelta {
        &mut self.brick_delta
    }
    pub fn get_material_delta_mut(&mut self) -> &mut DeviceDataDelta {
        &mut self.material_delta
    }
    pub fn get_index_delta_mut(&mut self) -> &mut DeviceDataDelta {
        &mut self.index_delta
    }
    pub fn get_brick_delta(&self) -> &DeviceDataDelta {
        &self.brick_delta
    }
    pub fn get_material_delta(&self) -> &DeviceDataDelta {
        &self.material_delta
    } 
    pub fn get_index_delta(&self) -> &DeviceDataDelta {
        &self.index_delta
    }

}
