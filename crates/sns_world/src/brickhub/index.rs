use std::marker::PhantomData;
use super::{BRICKMAP_SIZE, BRICKS_PER_BRICKMAP};
use bevy_math::*;
use bytemuck::{self, Pod, Zeroable};
#[repr(C)]

pub struct IndexGrid {
    indices: [u32; BRICKS_PER_BRICKMAP],
}

impl Default for IndexGrid {
    fn default() -> Self {
        Self {
            indices: [0; BRICKS_PER_BRICKMAP],
        }
    }
}
impl IndexGrid {
    pub fn index(coord: UVec3) -> usize {
        let cell_size = BRICKMAP_SIZE as u32;
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
    pub fn slice_range(&self, from: usize, to: usize) -> &[u32] {
        &self.indices[from..to]
    }
    
}

// 1. The "Raw" data type that bytemuck can easily handle
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Pod, Zeroable)]
pub struct RawIndex(pub u32);
impl RawIndex {
    pub fn is_lod(&self) -> bool  {
        self.0 & 0x8000_0000 != 0
    }
    pub fn is_unset(&self) -> bool {
        self.0 == 0
    }
    pub fn value(&self) -> u32 {
        self.0 & 0x7FFF_FFFF
    }
}

// 2. The Marker Trait
pub trait IndexKind: Copy + 'static {
    const FLAG: u32;
}

#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub struct VoxelKind;
impl IndexKind for VoxelKind { const FLAG: u32 = 0x0; }

#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub struct LodKind;
impl IndexKind for LodKind { const FLAG:u32 = 0x8000_0000; }

// 3. The Typed Wrapper (No PhantomData in the Pod struct!)
#[repr(transparent)]
pub struct StartIndex<K: IndexKind> {
    pub raw: RawIndex,
    _marker: PhantomData<K>,
}
impl<K: IndexKind> StartIndex<K> {
    pub fn new(value: u32) -> Self {
        debug_assert!(value & 0x8000_0000 == 0, "Index value too large!");
        Self {
            raw: RawIndex((value & 0x7FFF_FFFF) | K::FLAG),
            _marker: PhantomData,
        }
    }
    pub fn get_raw(&self) -> RawIndex {
        self.raw
    }
    pub fn value(&self) -> u32 {
        self.raw.value()
    }
    pub fn is_unset(&self) -> bool {
        self.raw.is_unset()
    }
}

pub const UNSET_RAW_INDEX: RawIndex = RawIndex(0);