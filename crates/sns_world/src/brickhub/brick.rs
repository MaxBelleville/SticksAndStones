use bevy_math::prelude::*;
use bytemuck;
use super::index;
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Brick {
    /// 1 bit per voxel (512 bits)
    occupancy: [u64; 8], // 8 * 64 = 512 bits
    lod_mask_4x4x4: u64,
    lod_mask_2x2x2: u8,
    lod_mask_1x1x1: u8,
    pub _pad: [u8; 2],

    raw_index: index::RawIndex,
}
impl Default for Brick {
    fn default() -> Self {
        Self {
            occupancy: [0; 8],
            lod_mask_4x4x4: 0,
            lod_mask_2x2x2: 0,
            lod_mask_1x1x1: 0,
            _pad: [0; 2],
            raw_index: index::UNSET_RAW_INDEX,
        }
    }
}

impl Brick {
    pub fn is_empty(&self) -> bool {
        self.occupancy.iter().all(|&m| m == 0)
    }
    pub fn test_occupancy(&self, word: usize, bit: usize) -> bool {
        (self.occupancy[word] & (1 << bit)) != 0
    }
    pub fn clear_occupancy(&mut self, word: usize, bit: usize) {
        self.occupancy[word] &= !(1 << bit);
    }
    pub fn occupied_voxel_count(&self) -> usize {
        self.occupancy
            .iter()
            .map(|&m| m.count_ones() as usize)
            .sum()
    }
    pub fn get_raw_index(&self) -> index::RawIndex {
        self.raw_index
    }

    pub fn set_start_index<K: index::IndexKind>(&mut self, start_index: u32) {
        self.raw_index = index::StartIndex::<K>::new(start_index).get_raw();
    }
    pub fn set_occupancy(&mut self, word: usize, mask: u64) {
        self.occupancy[word] |= mask;
    }

    pub fn get_lod_4x4x4_idx(&self, coord: UVec3) -> usize {
        (coord.x as usize >> 1) + ((coord.y as usize >> 1) << 2) + ((coord.z as usize >> 1) << 4)
    }
    pub fn get_lod_2x2x2_idx(&self, coord: UVec3) -> usize {
        (coord.x as usize >> 2) + ((coord.y as usize >> 2) << 1) + ((coord.z as usize >> 2) << 2)
    }

    pub fn any_voxel_lod_4x4x4(&self, coord: UVec3) -> bool {
        let bx = (coord.x >> 1) << 1;
        let by = (coord.y >> 1) << 1;
        let bz = (coord.z >> 1) << 1;

        for z in bz..bz + 2 {
            for y in by..by + 2 {
                for x in bx..bx + 2 {
                    let idx = x + y * 8 + z * 64;
                    let word = (idx >> 6) as usize;
                    let bit = idx & 63;
                    if (self.occupancy[word] & (1u64 << bit)) != 0 {
                        return true;
                    }
                }
            }
        }
        false
    }
    pub fn any_voxel_2x2x2(&self, coord: UVec3) -> bool {
        let bx = (coord.x >> 2) << 2;
        let by = (coord.y >> 2) << 2;
        let bz = (coord.z >> 2) << 2;

        for z in bz..bz + 4 {
            for y in by..by + 4 {
                for x in bx..bx + 4 {
                    let idx = x + y * 8 + z * 64;
                    let word = (idx >> 6) as usize;
                    let bit = idx & 63;
                    if (self.occupancy[word] & (1u64 << bit)) != 0 {
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn set_lod(&mut self, coord: UVec3) {
        self.lod_mask_4x4x4 |= 1u64 << self.get_lod_4x4x4_idx(coord);
        self.lod_mask_2x2x2 |= 1u8 << self.get_lod_2x2x2_idx(coord);
        self.lod_mask_1x1x1 = 1;
    }

    pub fn clear_lod(&mut self, coord: UVec3) {
        if !self.any_voxel_lod_4x4x4(coord) {
            self.lod_mask_4x4x4 &= !(1u64 << self.get_lod_4x4x4_idx(coord));
            if self.lod_mask_4x4x4 == 0 {
                self.lod_mask_2x2x2 = 0;
                self.lod_mask_1x1x1 = 0;
            } else if !self.any_voxel_2x2x2(coord) {
                self.lod_mask_2x2x2 &= !(1u8 << self.get_lod_2x2x2_idx(coord));
                if self.lod_mask_2x2x2 == 0 {
                    self.lod_mask_1x1x1 = 0;
                }
            }
        }
    }
}
#[repr(transparent)]
#[derive(Clone, Copy, Default)]
pub struct BrickStatusMask {
    pub bits: u32,
}

impl BrickStatusMask {
    pub fn set_loaded(&mut self, bit: u8) {
        self.bits |= 1 << bit;
    }

    pub fn clear_loaded(&mut self, bit: u8) {
        self.bits &= !(1 << bit);
    }

    pub fn is_loaded(&self, bit: u8) -> bool {
        (self.bits & (1 << bit)) != 0
    }
}

#[derive(Default)]
pub struct BrickStatusGrid {
    /// 1 bit per brick
    masks: Vec<BrickStatusMask>, // each u32 = 32 bricks
}

impl BrickStatusGrid {
    pub fn new(brick_count: usize) -> Self {
        let mask_count = (brick_count + 31) / 32;
        Self {
            masks: vec![BrickStatusMask::default(); mask_count],
        }
    }

    pub fn set_active(&mut self, brick_index: usize) {
        let word = brick_index >> 5;
        let bit = brick_index & 31;
        self.masks[word].set_loaded(bit as u8);
    }

    pub fn set_inactive(&mut self, brick_index: usize) {
        let word = brick_index >> 5;
        let bit = brick_index & 31;
        self.masks[word].clear_loaded(bit as u8);
    }

}
