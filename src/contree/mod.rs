pub mod chunk;
mod node;
mod pool;
pub mod res;
mod serialize;

use std::{collections::HashMap, sync::Arc};

use bevy::math::*;

use super::shared;

/**
    Tetrahexaconta tree (ConTree).
    The main tree that handles collecting 'blocks' and converting them to the tree.
    Serialization/optomization of the entire tree and more highlevel tree operations.
*/
#[derive(Debug)]
pub struct Contree {
    root: Arc<node::DagNode>,
    root_hash: u64,
    max_size: usize, //multiple of 4
    chunks: HashMap<IVec3, chunk::ContreeChunk>,
    offset: IVec3,

    update_cache:bool,
    serialized_cache: Option<Vec<u8>>,
    gpu_nodes_cache: Option<Vec<res::SvtGpuNode>>,
}
//Note will not clone the chunk hashset so not usable in render mode 
//But shouldn't be an issue.
impl Clone for Contree {
    fn clone(&self) -> Self {
        Self { root: self.root.clone(), root_hash: self.root_hash.clone(), max_size: self.max_size.clone(), 
            chunks: HashMap::new(),
             offset: self.offset.clone(), 
             update_cache: self.update_cache.clone(), 
             serialized_cache: self.serialized_cache.clone(), 
             gpu_nodes_cache: self.gpu_nodes_cache.clone() }
    }
}


//-------------------------------------- INIT -----------------------------------------------//
impl Default for Contree {
    fn default() -> Self {
        Contree {
            root: Arc::new(node::DagNode::default()),
            max_size: 4096, //This is basically 4096x4096x4096 or 68 billion possible voxels.
            chunks: HashMap::new(),
            offset: IVec3::ZERO,
            root_hash: 0,
            serialized_cache: None, // Cache for serialized data
            gpu_nodes_cache: None,  // Cache for GPU nodes
            update_cache: false,
            
        }
    }
}

impl Contree {
    pub fn from_serialized(bits: &mut Vec<u8>, root_size: usize, max_size: usize)  {
        //Call deserialize
        //Seperate into chunks
        //Init default
        //Unify from chunks.
    }
    //------------------------------ GETTERS AND HELPERS --------------------------------------//

    pub fn get_root_size(&self) -> usize {
        self.root.get_voxel_extent()
    }

    pub fn get_root_min(&self) -> IVec3 {
        IVec3::splat(-(self.root.get_voxel_extent() as i32) / 2)
    }

    pub fn get_offset(&self) -> IVec3 {
        self.offset
    }

    pub fn get_material(&self, mut voxel: shared::res::VoxelVec) -> Option<u8> {
        let chunk_coord= voxel.localize();
        if let Some(chunk) = self.chunks.get(&chunk_coord) {
            return chunk.get_material(voxel.pos);
        }

        return None;
    }

    pub fn print_root(&self) {
        //Can only print after build.
        if self.root_hash!=0 {
            let mut path = Vec::new();
            self.root.print_tree(0, self.get_root_size(),self.offset, &mut path);
        }
    }

    pub fn chunk_in_bounds(&self,chunk_coord: IVec3) -> bool {
        let half = self.max_size / 2;
        let min = -IVec3::splat(half as i32);
        let max =  IVec3::splat(half as i32);

        let block_min = chunk_coord * 64;
        let block_max = block_min + IVec3::splat(64);
        block_min.cmplt(max).all() && block_max.cmpgt(min).all()
    }
    pub fn add_chunk(&mut self, chunk_coord: IVec3, chunk_data: chunk::ContreeChunk) {
        self.chunks.insert(chunk_coord,chunk_data);
    }

    pub fn has_chunk(&self, chunk_coord: IVec3)->bool {
        self.chunks.contains_key(&chunk_coord)
    }
 
 
    //-------------------------------- ADD AND REMOVE ----------------------------------------//
    pub fn build_voxels(&mut self, mut voxel: shared::res::VoxelVec) {
        let chunk_coord = voxel.localize();

        if !self.chunk_in_bounds(chunk_coord) {
            return;
        }

        if voxel != None {
            let chunk = self
                .chunks
                .entry(chunk_coord)
                .or_insert_with(|| (chunk::ContreeChunk::new(shared::maths::total_nodes(64))));
           
            chunk.build_voxel(voxel);

        }
        else if let Some(chunk) = self.chunks.get_mut(&chunk_coord) {
            chunk.build_voxel(voxel);

            // If chunk is now empty, drop it entirely
            if chunk.is_empty() {
                self.chunks.remove(&chunk_coord);
            }
        }
    }

    pub fn build_all_voxels(&mut self,voxels: Vec<shared::res::VoxelVec>) 
    {
        //Split into chunks then build.
        for voxel in voxels {
             self.build_voxels(voxel);
        }
    }

    //----------------------------------- BUILD CHUNK ------------------------------------------//

   pub fn unify_chunks(&mut self) {
 
        // Collect voxel data from all chunks
        let mut voxel_data = Vec::new();
        for (chunk_coord, chunk) in &self.chunks {
            chunk.collect_leaf_data(*chunk_coord, &mut voxel_data);
        }

        // Normalize voxel positions to unified space
        for voxel in &mut voxel_data {
            voxel.normalize(self.max_size as i32);
        }

        // New pool for unified tree
        let mut pool = pool::ContreeNodePool::new(
            shared::maths::estimate_nodes(voxel_data.len(), self.max_size));

        // Allocate the root node
        let root_idx = pool.get_root_node(self.max_size / 4);
        let base = pool.get_next_children_block(self.max_size / 16);
        pool.get_child_mut(root_idx).set_pool_idx(Some(base));


        // Sort by Morton code for radix grouping
        //voxel_data.sort_unstable_by_key(|v| v.morton);

        // Batch build all voxels in one go
        let voxel_size = voxel_data.len();
        pool.build_all_voxels(
            root_idx,
            voxel_data.as_mut(),
            0,
            voxel_size,
            self.max_size,
            IVec3::ZERO, // starting origin
        );

        // 1. Trim single-octant layers
        self.offset = pool.trim_single_64(root_idx);

        // 2. Collapse fully filled subtrees
        pool.collapse_filled(root_idx);

        // 3. DAGify
        let mut dag_cache = HashMap::new();
        let (dag_root, dag_hash) = pool.dagify(root_idx, &mut dag_cache);
        self.root = dag_root;
        self.root_hash = dag_hash;

        self.update_cache = true;
    }
    //------------------------------------ SERIALIZE -------------------------------------------//

    pub fn serialize_with_cache(
    &mut self,
    global_dag_cache: &mut HashMap<u64, (Vec<u8>, Vec<res::SvtGpuNode>)>,
    ) -> (&[u8], Vec<res::SvtGpuNode>) {
        if self.update_cache || self.serialized_cache.is_none() || self.gpu_nodes_cache.is_none() {
            let mut bit_writer = serialize::BitstreamWriter::new();
            let mut gpu_writer = serialize::GpuNodeWriter::new();
            let mut seen_hashes = HashMap::new();

            serialize::serialize_nodes_dag(
                &self.root,
                self.root_hash,
                &mut bit_writer,
                &mut gpu_writer,
                global_dag_cache,
                &mut seen_hashes,
            );

            self.serialized_cache = Some(bit_writer.bits);
            self.gpu_nodes_cache = Some(gpu_writer.get_nodes().to_vec());
            self.update_cache = false;
        }
        (
            self.serialized_cache.as_ref().unwrap(),
            self.gpu_nodes_cache.as_ref().unwrap().clone(),
        )
    }
    //----------------------------------- COMPRESS ------------------------------------------//

    /// Compresses the tree for File upload: trims, collapses, and serializes.
    pub fn compress_data(&mut self) -> (Vec<u8>, Vec<i32>) {
        let mut global_cache = HashMap::new();
        let (bitstream, gpu_data) = self.serialize_with_cache(&mut global_cache);

        let mut bits = bitstream.to_vec();
        while bits.len() < 4 {
            bits.push(0);
        }

        // println!("Serialized for save {} bytes length", bits.len());
        // println!(
        //     "Serialized data: [{}]",
        //     bits.iter()
        //         .map(|b| format!("{:08b}", b))
        //         .collect::<Vec<_>>()
        //         .join(" ")
        // );
        // for (i, node) in gpu_data.iter().enumerate() {
        //     let not_a_leaf = (node.material & 0x8000_0000) != 0;
        //     let material_index = node.material & 0xFF;
        //     let mask_low = (node.child_mask & 0xFFFF_FFFF) as u32;
        //     let mask_high = (node.child_mask >> 32) as u32;
        //     println!(
        //         "GPU Node {}: child_mask: low {:032b} high {:032b}, first_child_idx: {}, material: {}, not_a_leaf: {}",
        //         i, mask_low, mask_high, node.first_child_idx, material_index, not_a_leaf
        //     );
        // }

        let mut packed_array: Vec<i32> = Vec::new();
        for node in gpu_data.iter() {
            let mask_low = (node.child_mask & 0xFFFF_FFFF) as u32;
            let mask_high = (node.child_mask >> 32) as u32;
            packed_array.push(mask_low as i32);
            packed_array.push(mask_high as i32);
            packed_array.push(node.first_child_idx as i32);
            packed_array.push(node.material as i32);
        }
        (bits, packed_array)
    }
    
    
}
