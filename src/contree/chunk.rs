
use bevy::math::*;


use super::shared;
use super::node;
#[derive(Debug, Clone)]
pub struct ContreeChunk {
    pool: node::ContreeNodePool,
    root: usize, // index of the root node in the pool
    chunk_size: usize,
}

impl Default for ContreeChunk {
    fn default() -> Self {
        // Default chunk is empty with size=64 (can be parameterized)
        let mut pool = node::ContreeNodePool::new();
        let root = pool.alloc_node(64/4);
        let base = pool.alloc_children(64/16);
        pool.get_child_mut(root,0).set_pool_idx(Some(base));
        Self { pool, root, chunk_size:64 }
    }
}

impl ContreeChunk {
    pub fn new(chunk_size: usize) -> Self {
        let mut pool = node::ContreeNodePool::new(); // always alloc 64 children per subdivision
        let root = pool.alloc_node(chunk_size/4);
        let base = pool.alloc_children(chunk_size/16);
        pool.get_child_mut(root,0).set_pool_idx(Some(base));
        Self { pool, root, chunk_size }
    }

    pub fn get_is_uniform(&self) -> bool {
        self.pool.get_is_uniform(self.root)
    }

    pub fn is_empty(&self) -> bool {
        self.pool.get_child(self.root, 0).is_empty()
    }

    pub fn build_voxel(&mut self, voxel: shared::res::VoxelVec) {
        self.pool.build_voxel(self.root, voxel);
    }


    pub fn build_all_voxels(&mut self,mut voxels: Vec<shared::res::VoxelVec>)
    {
        let vox_size = voxels.len();
        self.pool.build_all_voxels(self.root, voxels.as_mut(),0,vox_size,self.chunk_size,IVec3::ZERO);
    }

    pub fn get_material(&self, local_point: IVec3) -> Option<u8> {
        self.pool.get_material(self.root, local_point)
    }

    pub fn collect_leaf_data(
        &self,
        chunk_coord: IVec3,
        chunk_size: usize,
        out: &mut Vec<shared::res::VoxelVec>,
    ) {
        let base_pos = chunk_coord * chunk_size as i32;
        let extent = self.pool.get_child(self.root, 0).get_voxel_extent();
        self.pool.collect_leaf_data(self.root, base_pos, extent, out);
    }
}