use std::sync::atomic::AtomicU32;
use super::brick;
use bevy_math::*;
use sns_core::prelude::GpuBuffer;

#[derive(Clone, Copy, Default)]
pub struct HubDevice {
    /// bricks per axis in a cell
    brick_dim: UVec3,

    /// total voxels per axis in the hub or clipmap region
    voxel_dim: UVec3,

    /// World-space scale per voxel
    voxel_scale: f32,

    /// Padding for alignment
    _pad0: u32,

    /// Raymarch minimum step
    min_point_base: Vec4,

    /// Max bounds and brick scale
    max_point_scale: Vec4,
}


#[derive(Clone, Copy,Default)]
pub struct BrickLoadRequest {
    cell: IVec3,
    brick_index: u32,
}
#[derive(Default)]
pub struct HubDeviceQueue {
    brick_load_queue: Vec<BrickLoadRequest>,
    brick_load_count: AtomicU32,

    brick_upload_buffer: GpuBuffer<brick::Brick>,
    index_upload_buffer: GpuBuffer<u32>,

    material_upload_buffer: GpuBuffer<u16>,
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
            range: (usize::MAX,0)
        }
    }
}

impl DeviceDataDelta {
    #[inline]
    pub fn reset(&mut self) {
        self.state = DeltaState::Inactive;
        self.range = (usize::MAX,0);
    }

    #[inline]
    pub fn register(&mut self, index: usize) {
        self.state = DeltaState::Active;
        self.range = (self.range.0.min(index), self.range.1.max(index + 1));
    }

    #[inline]
    pub fn register_range(&mut self, from: usize, to: usize) {
        self.state = DeltaState::Active;
        self.range = (self.range.0.min(from), self.range.1.max(to));
    }
}
#[derive(Default)]
pub struct CellDeviceState {
     // CPU → GPU deltas
    brick_delta: DeviceDataDelta,
    material_delta: DeviceDataDelta,
    index_delta: DeviceDataDelta,

      // GPU residency
    gpu_brick_capacity: usize,
    gpu_brick_count: usize,
    gpu_bricks: GpuBuffer<brick::Brick>,
    gpu_indices: GpuBuffer<u32>,
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
}