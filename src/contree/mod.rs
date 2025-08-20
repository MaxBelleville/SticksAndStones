mod res;
mod node;
mod serialize;

use std::collections::HashMap;

use bevy::{math::*};

use super::shared;

/**
    ContreeNode is a node in a tetrahexaconta tree (ConTree).
    It can either be a leaf node with a material, or a non-leaf node with children.
    Each node can have up to 64 children, representing subdivisions of the space.
*/
#[derive(Debug,Clone)]
pub struct Contree {
    root: node::ContreeNode,
    max_size: usize, //multiple of 4
    chunks: HashMap<Vec<u8>,Box<node::ContreeNode>>,
    offset: IVec3,
}
    //-------------------------------------- INIT -----------------------------------------------//
impl Default for Contree {
    fn default() -> Self {
        Contree {
            root:  node::ContreeNode::default(),
            max_size: 4096, //This is basically 4096x4096x4096 or 68 billion possible voxels. 
            chunks: HashMap::new(),
            offset: IVec3::ZERO,
        }
    }
}

impl Contree {
    pub fn new(max_size: usize) -> Self {
        Contree {
            root:  node::ContreeNode::default(),
            max_size,
            chunks: HashMap::new(),
            offset: IVec3::ZERO,
        }
    }

    pub fn from_serialized(bits: &mut Vec<u8>, root_size: usize, max_size: usize) -> Self {
        let mut root = node::ContreeNode::new(root_size); // or the correct size if you store it
        root.deserialize(bits);
        Contree {
            root,
            max_size,
            chunks: HashMap::new(),//TODO load chunks from root during deserialization.
            offset: IVec3::ZERO,
        }
    }
    //------------------------------ GETTERS AND HELPERS --------------------------------------//
    
    pub fn get_root_size(&self) -> usize {
        self.root.get_voxel_extent()
    }
    pub fn is_small_root(&self) -> bool {
        self.root.get_voxel_extent() < 64
    }

    pub fn get_root_min(&self) -> IVec3 {
        IVec3::splat(-(self.root.get_voxel_extent() as i32) / 2)
    }

    pub fn get_offset(&self) -> IVec3 {
        self.offset
    }

    pub fn is_out_of_bounds(&self, point: &IVec3) -> bool {
        let min = self.get_root_min();                    // -extent / 2
        let max = min + IVec3::splat(self.get_root_size() as i32); // exclusive upper bound
        point.x < min.x || point.y < min.y || point.z < min.z ||
        point.x >= max.x || point.y >= max.y || point.z >= max.z
    }

    pub fn normalize_point(&self, point: &IVec3) -> IVec3 {
        *point + IVec3::splat((self.get_root_size() as i32)/2)
    }

    pub fn serialize(&mut self) -> Vec<u8> {
        self.optomize();
        let (x, _) = self.root.serialize(); //Since serialize proccesses both gpu and cpu data ignore gpu 
        return x;
    }

    pub fn get_material(&self, point: IVec3) -> Option<u8> {
        if self.is_out_of_bounds(&point) {
            println!("Voxel at {:?} is out of max bounds!", point);
            return None;
        }
        let normalized = self.normalize_point(&point);
        self.root.get_material_local(normalized)
    }


    pub fn deserialize(&mut self, bits: &mut Vec<u8>) {
        self.root.deserialize(bits);
    }

    pub fn rebuild_chunk_map(&mut self) {
        let mut chunks = HashMap::new();
        self.root.collect_leaf_data(IVec3::ZERO, self.root.get_voxel_extent(), &mut Vec::new(), &mut Vec::new(), &mut chunks);
        self.chunks = chunks;
    }

    pub fn print_root(&self) {
        let mut path = Vec::new();
        self.root.print_tree(0, self.get_root_size(), &mut path);
    }
     //------------------------------ DYNAMIC EXPANSION --------------------------------------//
    pub fn expand(&mut self, _point: IVec3) {
        let old_root = std::mem::replace(&mut self.root, node::ContreeNode::default());
        let old_size = old_root.get_voxel_extent();
        let new_size = old_size as i32 * 4;

        if new_size > self.max_size as i32 {
            self.root = old_root;
            return;
        }

        let mut voxel_data: Vec<(IVec3, u8, u8)> = Vec::new();
        let mut chunks_to_register = HashMap::new();

        old_root.collect_leaf_data(
            IVec3::ZERO,
            old_size,
            &mut Vec::new(),
            &mut voxel_data,
            &mut chunks_to_register,
        );

        let mut new_root = node::ContreeNode::new(old_size as usize);
        new_root.subdivide();

        for (world_pos, mat, lod_depth) in voxel_data {
            let offset = IVec3::splat(new_size / 2 - old_size as i32 / 2);
            let new_pos = world_pos + offset;
           // println!("Remap: old={:?} → new={:?}, lod_depth -> {:?}", world_pos, new_pos, lod_depth);
            new_root.insert_local(new_pos, Some(mat), lod_depth);
        }

        // Replace chunks
        self.chunks = chunks_to_register;
        self.root = new_root;
    }

    //-------------------------------- ADD AND REMOVE ----------------------------------------//
    pub fn insert(&mut self, point: IVec3, material_id: Option<u8>, an_level: Option<u8>) {
        let mut old_size = self.root.get_voxel_extent();
        while self.is_out_of_bounds(&point) && self.root.get_voxel_extent() < self.max_size {
            println!("Expanding {}", point);
            self.expand(point);
            if old_size == self.root.get_voxel_extent() {
                println!("Voxel at {:?} expand failed", point);
                return;
            }
            old_size = self.get_root_size();
        }

        if self.is_out_of_bounds(&point) {
            println!("Voxel at {:?} is out of max bounds", point);
            return;
        }
        // Start filling from the root node
        let normalized = self.normalize_point(&point);
        if let Some(curr_an_level) = an_level {
           self.root.insert_local(normalized, material_id, curr_an_level);
        }
        else {
           self.root.insert_local(normalized, material_id, 0);
        }

        self.root.update_cache();
    }

    pub fn fill(&mut self, point: IVec3, material_id: Option<u8>, depth: u8) {
        self.insert(point, material_id, Some(depth));
    }

    pub fn remove(&mut self, point: IVec3) {
        self.insert(point, None, None);
    }

    pub fn clear(&mut self, point: IVec3, depth: u8) {
        self.insert(point, None, Some(depth));
    }

    //------------------------------------- OPTOMIZE -------------------------------------------//

    pub fn optomize(&mut self) {
        // 1. Trim single-octant layers
        self.offset = self.trim_single_64();

        // 2. Collapse fully filled subtrees
        self.root.collapse_filled();

        // 3. Update cache for the root node
        self.root.update_cache();
    }

    pub fn trim_single_64(&mut self) -> IVec3 {
        let mut offset = IVec3::ZERO;
        let old_min = self.get_root_min();
        // While root has exactly one non-empty child, trim
        loop {
            let root_size = self.get_root_size();

            // Stop if the node is too small to subdivide (size == 1 or <4)
            if root_size < 4 {
                break;
            }

            let children = match self.root.get_children_mut() {
                Some(children) => children,
                None => break,
            };

            // Find the one non-empty child
            let mut non_empty = None;
            for (i, child) in children.iter().enumerate() {
                if !child.is_empty() {
                    if non_empty.is_some() {
                        // More than one non-empty child, stop trimming
                        return old_min + offset - self.get_root_min();
                    }
                    non_empty = Some(i);
                }
            }

            let idx = match non_empty {
                Some(i) => i,
                None => break, // All children empty
            };

            // For a 4x4x4 layout: index = (z << 4) | (y << 2) | x
            let child_width = root_size / 4;

            offset += shared::maths::deileave3(idx) * child_width as i32;

            // Promote the only non-empty child as new root
            self.root = std::mem::replace(&mut children[idx], node::ContreeNode::new(child_width));
        }

        old_min + offset - self.get_root_min()
    }


    //----------------------------------- COMPRESS ------------------------------------------//

    /// Compresses the tree for File upload: trims, collapses, and serializes.
    pub fn compress_data(&mut self) -> (Vec<u8>, Vec<i32>) {
        self.optomize();
         println!("Serialized start");
        let mut global_cache = HashMap::new();
        let (bitstream, gpu_data) = self.root.serialize_with_cache(&mut global_cache);

        let mut bits = bitstream.to_vec();
        while bits.len() < 4 {
            bits.push(0);
        }

        println!("Serialized for save {} bytes length", bits.len());
        println!(
            "Serialized data: [{}]",
            bits.iter()
                .map(|b| format!("{:08b}", b))
                .collect::<Vec<_>>()
                .join(" ")
        );
        for (i, node) in gpu_data.iter().enumerate() {
            let not_a_leaf = (node.material & 0x8000_0000) != 0;
            let material_index = node.material & 0xFF;
            let mask_low = (node.child_mask & 0xFFFF_FFFF) as u32;
            let mask_high = (node.child_mask >> 32) as u32;
            println!(
                "GPU Node {}: child_mask: low {:032b} high {:032b}, first_child_idx: {}, material: {}, not_a_leaf: {}",
                i,
                mask_low,
                mask_high,
                node.first_child_idx,
                material_index,
                not_a_leaf
            );
        }

        let mut packed_array: Vec<i32> = Vec::new();
        for node in gpu_data.iter() {
            let mask_low = (node.child_mask & 0xFFFF_FFFF) as u32;
            let mask_high = (node.child_mask >> 32) as u32;
            packed_array.push(mask_low as i32);
            packed_array.push(mask_high as i32);
            packed_array.push(node.first_child_idx as i32);
            packed_array.push(node.material as i32);
        }
        //Probably should only build if the size has changed/there's an offset
        self.rebuild_chunk_map();//Just incase 
        (bits, packed_array)
    }


}