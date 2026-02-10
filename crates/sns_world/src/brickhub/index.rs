use crate::brickhub::index;

use super::{BRICKHUB_CELL_SIZE, BRICKS_PER_CELL};
use bevy_math::*;
use bytemuck;
#[repr(C)]

pub struct IndexGrid {
    indices: [u32; BRICKS_PER_CELL],
}

impl Default for IndexGrid {
    fn default() -> Self {
        Self {
            indices: [0; BRICKS_PER_CELL],
        }
    }
}
impl IndexGrid {
    pub fn index(coord: UVec3) -> usize {
        let cell_size = BRICKHUB_CELL_SIZE as u32;
        (coord.x + coord.y * cell_size + coord.z * cell_size * cell_size) as usize
    }

    pub fn get(&self, coord: UVec3) -> Option<u32> {
        let v = self.indices[Self::index(coord)];
        if v == 0 { None } else { Some(v - 1) }
    }

    pub fn set(&mut self, coord: UVec3, brick_index: u32) {
        self.indices[Self::index(coord)] = brick_index + 1;
    }
    pub fn clear(&mut self, coord: UVec3) {
        self.indices[Self::index(coord)] = 0;
    }   
    pub fn raw_index(&self, index: u32) -> u32 {
        self.indices[index as usize]
    }
    pub fn slice(&self, start: usize, count: usize) -> &[u32] {
        &self.indices[start..start + count]
    }
    
}
#[repr(u32)]
pub enum IndexType {
    VoxelStartIndex = 0,
    BrickLodIndex = 1,
}

#[repr(transparent)]
#[derive(Copy, Clone, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct StartIndex(pub u32);

impl StartIndex {
    pub fn new(value: u32, kind: IndexType) -> Self {
        StartIndex((value & 0x7FFF_FFFF) | ((kind as u32) << 31))
    }

    pub fn is_unset(self) -> bool {
        self.0 == u32::MAX
    }

    pub fn value(self) -> u32 {
        self.0 & 0x7FFF_FFFF
    }

    pub fn kind(self) -> IndexType {
        if self.0 >> 31 == 1 {
            IndexType::BrickLodIndex
        } else {
            IndexType::VoxelStartIndex
        }
    }
}
pub const UNSET_START_INDEX: StartIndex = StartIndex(u32::MAX);
