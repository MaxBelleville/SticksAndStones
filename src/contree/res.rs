use std::hash::{Hash, Hasher};

use super::shared;

pub const NOT_A_LEAF_FLAG: u32 = 1 << 31;
pub const MATERIAL_MASK: u32 = 0xFF; // Lower 8 bits
#[derive(Clone, Copy, Default, Debug)]
pub struct SvtGpuNode {
    pub child_mask: u64,      // 64 bits = one per child(or leaf in array) (4x4x4)
    pub first_child_idx: u32, // index of the first child(or leaf in array) (flat array)
    pub material: u32, // material ID or surface info max value 255 = leaf element value to be used later ex grass
} //Given that material from contree_node is u8, we can have reserved bits

#[derive(Debug, Clone, PartialEq)]
pub enum LeafData {
    Empty,
    Uniform(u8),                // All 64 voxels are the same material
    Arr(Box<[Option<u8>; 64]>), // Per-voxel material
    //TODO: Potentially add MixedArray which holds array of different material types other then 0
}

impl Hash for LeafData {
    // Used in hasher
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            LeafData::Empty => {
                0u8.hash(state);
            }
            LeafData::Uniform(val) => {
                1u8.hash(state);
                val.hash(state);
            }
            LeafData::Arr(arr) => {
                // compute mask then compacted materials
                let mut mask: u64 = 0;
                for (i, v) in arr.iter().enumerate() {
                    if v.is_some() {
                        mask |= 1 << i;
                    }
                }
                // include the mask (so sparse layout is unambiguous)
                mask.hash(state);

                // include the compacted sequence of (index, material) OR only material in order
                for i in 0..64 {
                    if let Some(mat) = arr[i] {
                        // include slot index for absolute clarity (optional)
                        i.hash(state);
                        mat.hash(state);
                    }
                }
            }
        }
    }
}

impl LeafData {
    /// Check if this is Uniform
    pub fn is_array(&self) -> bool {
        matches!(self, LeafData::Arr(_))
    }

    /// Check if this is Uniform
    pub fn is_uniform(&self) -> bool {
        matches!(self, LeafData::Uniform(_))
    }

    /// Check if this is Empty
    pub fn is_empty(&self) -> bool {
        matches!(self, LeafData::Empty)
    }

    pub fn get(&self, index: usize) -> Option<u8> {
        match self {
            LeafData::Empty => None,
            LeafData::Uniform(val) => Some(*val),
            LeafData::Arr(array) => array.get(index).copied().flatten(),
        }
    }

    pub fn get_full(&self) -> Box<[Option<u8>; 64]> {
        match self {
            LeafData::Empty => Box::new([None; 64]),
            LeafData::Uniform(val) => Box::new([Some(*val); 64]),
            LeafData::Arr(array) => array.clone(),
        }
    }

    pub fn get_material_mask(&self) -> (u64, u32) {
        match self {
            LeafData::Empty => (0, NOT_A_LEAF_FLAG), // no material at all

            LeafData::Uniform(mat) => (u64::MAX, *mat as u32), // fully uniform

            LeafData::Arr(array) => {
                let mut mask = 0u64;
                let mut uniform_mat: Option<u32> = None;
                for (i, val) in array.iter().enumerate() {
                    if let Some(m) = val {
                        mask |= 1 << i;
                        match uniform_mat {
                            None => uniform_mat = Some(*m as u32),
                            Some(prev) if prev != *m as u32 => {
                                uniform_mat = Some(NOT_A_LEAF_FLAG); // mixed
                            }
                            _ => {}
                        }
                    }
                }
                (mask, uniform_mat.unwrap_or(0))
            }
        }
    }

    /// Set a voxel value at index.
    pub fn set(&mut self, value: Option<u8>, index: Option<usize>) {
        match (value, index) {
            // Case 1: Clear all
            (None, None) => {
                *self = LeafData::Empty;
            }

            // Case 2: Set uniform
            (Some(val), None) => {
                *self = LeafData::Uniform(val);
            }

            // Case 3: Remove single voxel
            (None, Some(i)) => match self {
                LeafData::Arr(array) if i < 64 => {
                    array[i] = None;
                }
                _ => {
                    *self = LeafData::Arr(Box::new([None; 64]));
                }
            },

            // Case 4: Set voxel at index
            (Some(val), Some(i)) if i < 64 => {
                match self {
                    LeafData::Empty => {
                        let mut array = Box::new([None; 64]);
                        array[i] = Some(val);
                        *self = LeafData::Arr(array);
                    }
                    LeafData::Uniform(current) => {
                        if *current == val {
                            return;
                        } else {
                            let mut array = Box::new([Some(*current); 64]);
                            array[i] = Some(val);
                            *self = LeafData::Arr(array);
                        }
                    }
                    LeafData::Arr(array) => {
                        array[i] = Some(val);

                        // Check for uniformity
                        if let Some(first_val) = array[0] {
                            if array.iter().all(|&x| x == Some(first_val)) {
                                *self = LeafData::Uniform(first_val);
                            }
                        }
                    }
                }
            }

            // Case 4 fallback (invalid index)
            _ => {}
        }
    }

    pub fn print(
        &self,
        depth: usize,
        tree_scale: usize,
        root_size: usize,
        path: &mut Vec<Option<u8>>,
    ) {
        let indent = "  ".repeat(depth * 2);
        match self {
            LeafData::Empty => {}
            LeafData::Uniform(mat) => {
                for i in 0..64 {
                    path.push(Some(i as u8));
                    let pos = shared::maths::pathtrace_to_world_pos(root_size, path);
                    println!(
                        "{}Leaf(size={}, mat={}, depth={}, pos={})",
                        indent, tree_scale, mat, depth, pos
                    );
                    path.pop();
                }
            }
            LeafData::Arr(array) => {
                for (i, mat) in array.iter().enumerate() {
                    if let Some(m) = mat {
                        path.push(Some(i as u8));
                        let pos = shared::maths::pathtrace_to_world_pos(root_size, path);
                        println!(
                            "{}Leaf(size={}, mat={}, depth={}, pos={})",
                            indent, tree_scale, m, depth, pos
                        );
                        path.pop();
                    }
                }
            }
        }
    }
}
