
use bevy::math::*;


use super::shared;
use super::node;
#[derive(Debug, Clone)]
pub struct ContreeChunk {
    pool: node::ContreeNodePool,
    root: usize, // index of the root node in the pool
}


impl ContreeChunk {
    pub fn new(voxel_size: usize) -> Self {
        let capacity = shared::maths::estimate_chunk(voxel_size);
        let mut pool = node::ContreeNodePool::new(capacity); // always alloc 64 children per subdivision
        let root = pool.get_root_node(64/4);
        let base = pool.get_next_children_block(64/16);
        pool.get_child_mut(root,0).set_pool_idx(Some(base));
        Self { pool, root }
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
        self.pool.build_all_voxels(self.root, voxels.as_mut(),0,vox_size,64,IVec3::ZERO);
    }

    pub fn get_material(&self, local_point: IVec3) -> Option<u8> {
        self.pool.get_material(self.root, local_point)
    }

 

    pub fn collect_leaf_data(
        &self,
        chunk_coord: IVec3,
        out: &mut Vec<shared::res::VoxelVec>,
    ) {
        let base_pos = chunk_coord * 64;
        let extent = self.pool.get_child(self.root, 0).get_voxel_extent();
        self.pool.collect_leaf_data(self.root, base_pos, extent, out);
    }
}