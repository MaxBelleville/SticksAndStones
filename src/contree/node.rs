use std::sync::Arc;

use bevy::math::*;

use super::{res, shared};

/**
    ContreeNode is a node in a tetrahexaconta tree (ConTree).
    It can either be a leaf node with a material, or a non-leaf node with children.
    Each node can have up to 64 children, representing subdivisions of the space.
*/
#[derive(Debug,Clone)]
pub struct ContreeNode {
    pool_idx: Option<usize>, //64 children for ConTree
    tree_scale: usize,       // Each node spans (tree_scale × 4)³ voxel so 4 tree_scale = 16³
    leaves: res::LeafData,   // Optional mat index
}
impl Default for ContreeNode {
    fn default() -> Self {
        ContreeNode {
            pool_idx: None,
            tree_scale:1,
            leaves: res::LeafData::Empty,
        }
    }
}


impl ContreeNode {
    pub fn new(tree_scale: usize) -> Self {
        ContreeNode {
            pool_idx: None,
            tree_scale,
            leaves: res::LeafData::Empty,

        }
    }

    pub fn is_empty(&self) -> bool {
        self.leaves.is_empty() && self.pool_idx.is_none()
    }

    pub fn has_material(&self) -> bool {
        !self.leaves.is_empty()
    }

    pub fn get_mat_at(&self, index: usize) -> Option<u8> {
        self.leaves.get(index)
    }

    pub fn get_leaves(&self) -> &res::LeafData {
        &self.leaves
    }

    pub fn get_leaves_mut(&mut self) -> &mut res::LeafData {
        &mut self.leaves
    }
      
    pub fn set_size(&mut self, size: usize) {
        self.tree_scale = size / 4;
    }

    pub fn set_mat(&mut self, value: Option<u8>, index: Option<usize>) {
        self.leaves.set(value, index);
    }

    pub fn set_leaves(&mut self, leaves: res::LeafData) {
        self.leaves = leaves;
    }

     pub fn set_to_leaf(&mut self, leaves: res::LeafData) {
        self.leaves = leaves;
        self.pool_idx = None;
    }
    
    pub fn get_logical_size(&self) -> usize {
        self.tree_scale
    }
    pub fn get_voxel_extent(&self) -> usize {
        self.tree_scale * 4
    }
    pub fn get_pool_idx(&self) -> Option<usize> {
        self.pool_idx
    }

    pub fn set_pool_idx(&mut self, pool: Option<usize>) {
        self.pool_idx = pool;
    }
}

#[derive(Debug, Clone)]
pub struct DagNode {
    pub leaves: res::LeafData,
    pub tree_scale: usize,
    pub children: Option<[Arc<DagNode>; 64]>,
    pub hash: u64, // cached
}
//----------------------------------------- INIT ------------------------------------------------//
impl Default for DagNode {
    fn default() -> Self {
        DagNode {
            children: None,
            tree_scale: 0, // Default size of the root node
            leaves: res::LeafData::Empty,
            hash: 0,
        }
    }
}

impl DagNode {
    pub fn new(hash: u64, tree_scale: usize) -> Self {
        DagNode {
            children: None,
            tree_scale,
            leaves: res::LeafData::Empty,
            hash,
        }
    }
    //---------------------------- GETTERS/ SETTERS / HELPERS ---------------------------------//
    pub fn is_children_empty(&self) -> bool {
        self.children.is_none()
    }

    pub fn is_empty(&self) -> bool {
        self.leaves.is_empty() && self.children.is_none()
    }

    pub fn get_hash(&self) -> u64 {
        self.hash
    }

    pub fn has_material(&self) -> bool {
        !self.leaves.is_empty()
    }

    pub fn get_material_at(&self, index: usize) -> Option<u8> {
        self.leaves.get(index)
    }

    pub fn get_leaves(&self) -> &res::LeafData {
        &self.leaves
    }

    pub fn get_logical_size(&self) -> usize {
        self.tree_scale
    }
    pub fn get_voxel_extent(&self) -> usize {
        self.tree_scale * 4
    }


    pub fn get_children(&self) -> Option<&[Arc<DagNode>; 64]> {
        self.children.as_ref()
    }

    pub fn get_children_mut(&mut self) -> Option<&mut [Arc<DagNode>; 64]> {
        self.children.as_mut()
    }

    pub fn print_tree(&self, depth: usize, root_size: usize, offset: IVec3, path: &mut Vec<u8>) {
        let indent = "  ".repeat(depth * 2);

        self.leaves
            .print(depth, self.tree_scale, root_size, offset, path);

        if let Some(children) = &self.children {
            println!("{}Node(size={}, depth={})", indent, self.tree_scale, depth);

            for (i, child) in children.iter().enumerate() {
                if child.is_empty() {
                    continue;
                }

                path.push(i as u8);
                let pos = shared::maths::pathtrace_to_world_pos(root_size, path);
                println!(
                    "{}  Child[{}](depth={}, pos={}):",
                    indent,
                    i,
                    depth,
                    pos + offset
                );
                child.print_tree(depth + 1, root_size, offset, path);
                path.pop();
            }
        }
    }
}
