use bevy_math::*;

use std::sync::atomic::Ordering;
use std::sync::{atomic::AtomicU32, Mutex};
use std::cmp::{min, max};

// --- Constants ---
pub type AtomicCount = AtomicU32;
pub const BRICK_DIM: u32 = 4;
pub const VOXELS_PER_BRICK: u32 = BRICK_DIM * BRICK_DIM * BRICK_DIM; // 64
pub const BRICK_BYTES: u32 = VOXELS_PER_BRICK / 8; // 8
pub const BRICK_WORDS: u32 = BRICK_BYTES / 4; // 2
pub const BRICK_LOG2: u32 = VOXELS_PER_BRICK.ilog2(); // 6
pub const UNALLOCATED_INDEX: u32 = u32::MAX;

pub type BrickMap = u64; // std.meta.Int(.unsigned, 64)
pub type BrickMapLog2 = u8; // std.meta.Int(.unsigned, 6)
pub type MaterialIndices = u8;
pub type IndexToBrick = AtomicU32; // Changed to Atomic

#[derive(Default)]
pub struct State {
    pub brick_statuses: Vec<BrickStatusMask>, // Now contains AtomicU32
    pub brick_statuses_delta: DeviceDataDelta,

    pub grid_to_brick: Vec<IndexToBrick>,     // Now Vec<AtomicU32>
    pub grid_to_brick_delta: DeviceDataDelta,

    pub brick_occupancy: Vec<u8>,             // Accessed via unique brick_idx
    pub bricks_occupancy_delta: DeviceDataDelta,

    pub brick_material_base: Vec<StartIndex>, // See note below
    pub brick_material_base_delta: DeviceDataDelta,

    pub material_indices: Vec<u8>,
    pub material_indices_delta: DeviceDataDelta,

    pub active_bricks: AtomicU32,
    pub device_state: Device,
    pub work_segment_size: usize,
}


/// Type used to record changes in host/device buffers
#[derive(Default)]
pub struct DeviceDataDelta {
    inner: Mutex<DeltaInner>,
}
#[derive(Default)]
struct DeltaInner {
    // Range is [start, end)
    range: Option<std::ops::Range<usize>>,
}
impl DeviceDataDelta {
    pub fn register_delta(&self, index: usize) {
        self.register_delta_range(index, index + 1);
    }

    pub fn register_delta_range(&self, from: usize, to: usize) {
        let mut data = self.inner.lock().unwrap();
        if let Some(ref mut existing) = data.range {
            existing.start = existing.start.min(from);
            existing.end = existing.end.max(to);
        } else {
            data.range = Some(from..to);
        }
    }

    pub fn reset(&self) {
        let mut data = self.inner.lock().unwrap();
        data.range = None;
    }
}
// --- Device Struct ---

#[derive(Default, Copy, Clone)]
pub struct Device {
    pub voxel_dim: UVec3,
    pub dim: UVec3,
    pub padding: (u32,u32),
    pub min_point_base: Vec4,
    pub max_point_scale: Vec4,
}

// --- BrickStatusMask ---

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Status {
    Empty = 0,
    Loaded = 1,
}

#[repr(transparent)]
#[derive(Default)]
pub struct BrickStatusMask {
    pub bits: AtomicU32, // Changed to atomic for thread-safe status updates
}

impl BrickStatusMask {
    #[inline]
    pub fn set_loaded(&self, bit: u8) {
        debug_assert!(bit < 32, "Status bit index {} out of bounds", bit);
        self.bits.fetch_or(1 << bit, Ordering::Release);
    }

    #[inline]
    pub fn is_loaded(&self, bit: u8) -> bool {
        debug_assert!(bit < 32);
        (self.bits.load(Ordering::Acquire) & (1 << bit)) != 0
    }
}

// --- Brick ---

#[repr(u32)]
#[derive(Debug, Copy, Clone)]
pub enum IndexType {
    VoxelStartIndex = 0,
    BrickLodIndex = 1,
}

/// Rust doesn't have native u31, so we handle the bit-packing manually
#[repr(transparent)]
#[derive(Debug, Copy, Clone)]
pub struct StartIndex(pub u32);

impl StartIndex {
    pub fn new(value: u32, index_type: IndexType) -> Self {
        // value is 31 bits, index_type is the 32nd bit
        let packed = (value & 0x7FFF_FFFF) | ((index_type as u32) << 31);
        StartIndex(packed)
    }

    pub fn get_value(&self) -> u32 { self.0 & 0x7FFF_FFFF }
    pub fn get_type(&self) -> IndexType {
        if (self.0 >> 31) == 1 { IndexType::BrickLodIndex } else { IndexType::VoxelStartIndex }
    }
}

pub const UNSET_INDEX: StartIndex = StartIndex(u32::MAX);
pub const EMPTY_OCCUPANCY: [u8; BRICK_BYTES as usize] = [0; BRICK_BYTES as usize];


