
use bevy::math::*;

use super::node;


#[derive(Debug,Clone)]
pub struct ContreeChunk {
    node: node::ContreeNode
}
impl Default for ContreeChunk {
    fn default() -> Self {
        ContreeChunk {
            node:  node::ContreeNode::default(), //Just incase but shouldn't use.
        }
    }
}
impl ContreeChunk {
    pub fn new(size: usize) -> Self {
        ContreeChunk {
            node:  node::ContreeNode::new(size/4)
        }
    }

    pub fn get_is_uniform(&self) -> bool {
        self.node.get_is_uniform()
    }
    pub fn get_node(&self) -> &node::ContreeNode {
        &self.node
    }
    pub fn is_empty(&self) -> bool {
        self.node.is_empty()
    }

    pub fn insert(&mut self, local_point: IVec3, material_id: Option<u8>, chunk_level: u8) {
        self.node.insert_local(local_point, material_id, chunk_level);
        if material_id == None {
            self.node.check_leave_empty();
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
        self.node.get_material_local(local_point)
    }
    
    pub fn collect_leaf_data(
        &self,
        chunk_coord: IVec3,        // the chunk's coordinate in chunk-space
        chunk_size: usize,         // usually 64
        out: &mut Vec<(IVec3, u8, u8)>,
    ) {
        let base_pos = chunk_coord * chunk_size as i32;
        let extent = self.node.get_voxel_extent(); // should be == chunk_size
        // Delegate to the node version, but offset positions into world-space
        self.node.collect_leaf_data(base_pos, extent, out);
    }

}