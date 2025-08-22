
use bevy::math::*;

use super::node;
#[derive(Debug, Clone)]
pub struct ContreeChunk {
    pool: node::ContreeNodePool,
    root: usize, // index of the root node in the pool
}

impl Default for ContreeChunk {
    fn default() -> Self {
        // Default chunk is empty with size=64 (can be parameterized)
        let mut pool = node::ContreeNodePool::new();
        let root = pool.alloc_node(64/4);
        let base = pool.alloc_children(64/16);
        pool.get_child_mut(root,0).set_pool_idx(Some(base));
        Self { pool, root }
    }
}

impl ContreeChunk {
    pub fn new(size: usize) -> Self {
        let mut pool = node::ContreeNodePool::new(); // always alloc 64 children per subdivision
        let root = pool.alloc_node(size/4);
        let base = pool.alloc_children(size/16);
        pool.get_child_mut(root,0).set_pool_idx(Some(base));
        Self { pool, root }
    }

    pub fn get_is_uniform(&self) -> bool {
        self.pool.get_is_uniform(self.root)
    }

    pub fn is_empty(&self) -> bool {
        self.pool.get_child(self.root, 0).is_empty()
    }

    pub fn insert(&mut self, local_point: IVec3, material_id: Option<u8>, chunk_level: u8) {
        self.pool.insert_local(self.root, local_point, material_id, chunk_level);
        if material_id.is_none() {
            self.pool.check_leave_empty(self.root);
        }
    }

    pub fn fill(&mut self, local_point: IVec3, material_id: Option<u8>, chunk_level: u8) {
        self.insert(local_point, material_id, chunk_level);
    }

    pub fn remove(&mut self, local_point: IVec3) {
        self.insert(local_point, None, 0);
    }

    pub fn clear(&mut self, point: IVec3, chunk_level: u8) {
        self.insert(point, None, chunk_level);
    }

    pub fn get_material(&self, local_point: IVec3) -> Option<u8> {
        self.pool.get_material_local(self.root, local_point)
    }

    pub fn collect_leaf_data(
        &self,
        chunk_coord: IVec3,
        chunk_size: usize,
        out: &mut Vec<(IVec3, u8, u8)>,
    ) {
        let base_pos = chunk_coord * chunk_size as i32;
        let extent = self.pool.get_child(self.root, 0).get_voxel_extent();
        self.pool.collect_leaf_data(self.root, base_pos, extent, out);
    }
}