pub mod chunk;
mod node;
mod res;
mod serialize;

use std::{collections::HashMap, sync::Arc};

use bevy::math::*;

use super::shared;

/**
    Tetrahexaconta tree (ConTree).
    The main tree that handles collecting 'blocks' and converting them to the tree.
    Serialization/optomization of the entire tree and more highlevel tree operations.
*/
#[derive(Debug, Clone)]
pub struct Contree {
    root: Arc<node::DagNode>,
    root_hash: u64,
    max_size: usize, //multiple of 4
    chunks: HashMap<IVec3, chunk::ContreeChunk>,
    chunk_size: usize,
    offset: IVec3,

    update_cache:bool,
    serialized_cache: Option<Vec<u8>>,
    gpu_nodes_cache: Option<Vec<res::SvtGpuNode>>,
}
//-------------------------------------- INIT -----------------------------------------------//
impl Default for Contree {
    fn default() -> Self {
        Contree {
            root: Arc::new(node::DagNode::default()),
            max_size: 4096, //This is basically 4096x4096x4096 or 68 billion possible voxels.
            chunks: HashMap::new(),
            chunk_size: 64,
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

    pub fn get_material(&self, point: IVec3) -> Option<u8> {
        let (chunk_coord, local) = self.localize_to_chunk(point);
        if let Some(chunk) = self.chunks.get(&chunk_coord) {
            return chunk.get_material(local);
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

    pub fn localize_to_chunk(&self, point: IVec3) -> (IVec3, IVec3) {
        let chunk_coord = shared::maths::div_floor_vec3(point, self.chunk_size as i32);
        let local_pos = shared::maths::mod_floor_vec3(point, self.chunk_size as i32);
      
        return (chunk_coord, local_pos);
    }

    pub fn chunk_in_bounds(&self,chunk_coord: IVec3) -> bool {
        let half = self.max_size / 2;
        let min = -IVec3::splat(half as i32);
        let max =  IVec3::splat(half as i32);

        let block_min = chunk_coord * self.chunk_size  as i32;
        let block_max = block_min + IVec3::splat(self.chunk_size  as i32);
        block_min.cmplt(max).all() && block_max.cmpgt(min).all()
    }
    pub fn add_chunk(&mut self, chunk_coord: IVec3, chunk_data: chunk::ContreeChunk) {
        self.chunks.insert(chunk_coord,chunk_data);
    }
    pub fn set_chunk_size(&mut self, chunk_size: usize) {
        self.chunk_size = chunk_size;
    }
    //-------------------------------- ADD AND REMOVE ----------------------------------------//
    pub fn insert(&mut self, point: IVec3, material_id: Option<u8>, an_level: Option<u8>) {
        let (chunk_coord,local) = self.localize_to_chunk(point);

        if !self.chunk_in_bounds(chunk_coord) {
            return;
        }

        let max_level = (self.chunk_size as f32).log(4.0) as u8;
        let chunk_level = an_level.map_or(0, |lvl| lvl.min(max_level));
            
        if material_id != None {
            let chunk = self
                .chunks
                .entry(chunk_coord)
                .or_insert_with(|| (chunk::ContreeChunk::new(self.chunk_size)));
           
            chunk.insert(local, material_id, chunk_level);

        }
        else if let Some(chunk) = self.chunks.get_mut(&chunk_coord) {
            chunk.insert(local, None, chunk_level);

            // If chunk is now empty, drop it entirely
            if chunk.is_empty() {
                self.chunks.remove(&chunk_coord);
            }
        }
    }

    pub fn insert_batch(&mut self,points: Vec<IVec3>,material_id: Option<u8>,an_level: Option<u8>) 
    {
        for point in points {
             self.insert(point, material_id, an_level);
        }
    }

    pub fn fill(&mut self, point: IVec3, material_id: Option<u8>, an_level: u8) {
        self.insert(point, material_id, Some(an_level));
    }

    pub fn fill_batch(&mut self, points: Vec<IVec3>, material_id: Option<u8>, an_level: u8) {
        self.insert_batch(points, material_id, Some(an_level));
    }

    pub fn remove(&mut self, point: IVec3) {
        self.insert(point, None, None);
    }

    pub fn remove_batch(&mut self, points: Vec<IVec3>) {
        self.insert_batch(points, None, None);
    }

    pub fn clear(&mut self, point: IVec3, an_level: u8) {
        self.insert(point, None, Some(an_level));
    }

    pub fn clear_batch(&mut self, points: Vec<IVec3>, an_level: u8) {
        self.insert_batch(points, None, Some(an_level));
    }
    //----------------------------------- BUILD CHUNK ------------------------------------------//

    pub fn unify_chunks(&mut self) {
        // Allocate the root node
        let mut pool = node::ContreeNodePool::new();

        // Allocate the root node
        let root_idx = pool.alloc_node(self.max_size/4);
        let base = pool.alloc_children(self.max_size/16);
        pool.get_child_mut(root_idx,0).set_pool_idx(Some(base));
        // Subdivide once to create the first layer
       
        // Subdivide once to create the first layer
        //pool.subdivide(root_idx, 0); // or: alloc_children inside subdivide if needed

        // Insert chunk data
        for (chunk_coord, chunk) in &self.chunks {
            let mut voxel_data = Vec::new();
            chunk.collect_leaf_data(*chunk_coord, self.chunk_size, &mut voxel_data);

            for (world_pos, mat, lod_depth) in voxel_data {
              
                let normalized = world_pos + IVec3::splat(self.max_size as i32 / 2);
                pool.insert_local(root_idx, normalized, Some(mat), lod_depth);
            }
        }

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

        println!("Serialized for save {} bytes length", bits.len());
        println!(
            "Serialized data: [{}]",
            bits.iter()
                .map(|b| format!("{:08b}", b))
                .collect::<Vec<_>>()
                .join(" ")
        );
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
