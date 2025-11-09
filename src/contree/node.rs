use bevy::math::*;

use super::{leaf, shared};

pub const NO_CHILD: u32 = u32::MAX; 
pub const SLAB_SIZE: usize = 64;
#[derive(Debug)]
pub struct NodePool {
    nodes: Vec<Node>,
    free_slabs: Vec<usize>, // base indices (multiples of 64)
    free_nodes: Vec<usize>, // individual nodes
}

impl NodePool {
    pub fn new(cap_nodes: usize) -> Self {
        let nodes = Vec::with_capacity(cap_nodes);
        Self {
            nodes,
            free_slabs: Vec::new(),
            free_nodes: Vec::new(),
        }
    }

    pub fn alloc_node(&mut self, depth: u8) -> usize {
        if let Some(base) = self.free_nodes.pop() {
            let node = &mut self.nodes[base];
            *node = Node::default();
            node.depth = depth;
            return base;
        }
        let base = self.nodes.len();
        self.nodes.push(Node {
            depth,
            ..Node::default()
        });
        base
    }
    //only use this on nodes that you know have no children.”
    pub fn free_node(&mut self, base: usize) {
        if base < self.nodes.len() {
            // clear node so no stale references remain
            self.nodes[base] = Node::default();
            self.free_nodes.push(base);
        }
    }
    pub fn free_node_recursive(&mut self, base: usize) {
        if base >= self.nodes.len() {
            return;
        }
        let child_base = self.nodes[base].body.first_child_idx;
        if child_base != NO_CHILD {
            self.free_slab_recursive(child_base as usize);
        }
        self.nodes[base] = Node::default();
        self.free_nodes.push(base);
    }

    pub fn alloc_slab(&mut self, child_depth: usize) -> usize {
        if let Some(base) = self.free_slabs.pop() {
            assert!(base + SLAB_SIZE <= self.nodes.len());
            for i in base..base + SLAB_SIZE {
                let node = &mut self.nodes[i];
                *node = Node::default();
                node.depth = child_depth as u8;
            }
            return base;
        }

        let base = self.nodes.len();
        self.nodes
            .resize_with(self.nodes.len() + SLAB_SIZE, || Node::new(child_depth));
        base
    }
    //only use this on slabs you know have no grand children.
    pub fn free_slab(&mut self, base: usize) {
        if (base as usize) + SLAB_SIZE <= self.nodes.len() {
            for i in 0..SLAB_SIZE {
                self.nodes[(base as usize) + i] = Node::default();
            }
            self.free_slabs.push(base);
        }
    }
    pub fn free_slab_recursive(&mut self, base: usize) -> Vec<usize> {
        let mut freed_leaves = Vec::new();

        if base + SLAB_SIZE > self.nodes.len() {
            return freed_leaves;
        }

        for idx in base..base + SLAB_SIZE {
            if self.nodes[idx].body.first_child_idx != NO_CHILD {
                freed_leaves.extend(
                    self.free_slab_recursive(self.nodes[idx].body.first_child_idx as usize),
                );
            } else if self.nodes[idx].body.offset_or_flags != leaf::NO_LEAF {
                // Collect leaf indices, but don't free here
                freed_leaves.push(self.nodes[idx].body.offset_or_flags as usize);
            }

            self.nodes[idx] = Node::default();
        }

        self.free_slabs.push(base);
        freed_leaves
    }

    pub fn subdivide(&mut self, parent: usize, child_depth: usize) {
        let base = self.alloc_slab(child_depth);
        let parent_node = &mut self.nodes[parent];
        parent_node.body.first_child_idx = base as u32;
    }

    pub fn collapse(&mut self, base: usize) -> Vec<usize> {
        let mut freed_leaves = Vec::new();
        if base >= self.nodes.len() {
            return freed_leaves;
        }

        let child_base = self.nodes[base].body.first_child_idx;
        if child_base != NO_CHILD {
            // recursively free the entire subtree rooted at base
            freed_leaves = self.free_slab_recursive(child_base as usize);

            // Reset the parent to be a leaf/empty
            self.nodes[base].body.first_child_idx = NO_CHILD;
            self.nodes[base].body.child_mask = 0;
            self.nodes[base].body.offset_or_flags = leaf::NO_LEAF;
            self.nodes[base].internal = InternalFlags::Interal;
            self.nodes[base].dag_hash = 0;
        }
        return freed_leaves;
    }

    pub fn check_alloc_children(&self, base: usize) -> bool {
        if base >= self.nodes.len() {
            return false;
        }
        let child_base = self.nodes[base].body.first_child_idx;
        child_base != NO_CHILD && (child_base as usize) + SLAB_SIZE <= self.nodes.len()
    }

    pub fn get_children(&self, base: usize) -> Vec<Node> {
        let indices = self.get_children_indices(base);
        indices.into_iter().map(|idx| self.nodes[idx]).collect()
    }

    pub fn get_child(&self, base: usize, child_idx: usize) -> Option<Node> {
        let Some(child) = self.get_child_index(base, child_idx) else {
            return None;
        };
        Some(self.nodes[child])
    }

    pub fn get_node_mut(&mut self, base: usize) -> Option<&mut Node> {
        if base >= self.nodes.len() {
            return None;
        }
        Some(&mut self.nodes[base])
    }

    pub fn get_node(&self, base: usize) -> Option<&Node> {
        if base >= self.nodes.len() {
            return None;
        }
        Some(&self.nodes[base])
    }

    pub fn get_node_depth(&self, base: usize) -> Option<u8> {
        if base >= self.nodes.len() {
            return None;
        }
        Some(self.nodes[base].depth)
    }

    pub fn get_node_hash(&self, base: usize) -> Option<u64> {
        if base >= self.nodes.len() {
            return None;
        }
        Some(self.nodes[base].dag_hash)
    }

    pub fn set_node_hash(&mut self, base: usize, hash: u64) {
        if base >= self.nodes.len() {
            return;
        }
        self.nodes[base].dag_hash = hash;
    }

    pub fn get_node_offset(&self, base: usize) -> Option<u32> {
        if base >= self.nodes.len() {
            return None;
        }
        Some(self.nodes[base].body.offset_or_flags)
    }

     pub fn get_child_offset(&self, base: usize, child_idx: usize) -> Option<u32> {
         let Some(child) = self.get_child_index(base, child_idx) else {
            return None;
        };
        Some(self.nodes[child].body.offset_or_flags)
    }


      pub fn get_first_child(&self, base: usize) -> Option<u32> {
        if base >= self.nodes.len() {
            return None;
        }
        Some(self.nodes[base].body.first_child_idx)
    }

     pub fn get_first_grandchild(&self, base: usize, child_idx: usize) -> Option<u32> {
         let Some(child) = self.get_child_index(base, child_idx) else {
            return None;
        };
        Some(self.nodes[child].body.first_child_idx)
    }

    pub fn reset_child_mask(&mut self, base: usize) {
        if base >= self.nodes.len() {
            return;
        }
        self.nodes[base].body.child_mask = 0;
    }

    pub fn set_child_mask(&mut self, base: usize, child_idx: usize) {
        if base >= self.nodes.len() {
            return;
        }
        self.nodes[base].body.child_mask |= 1u64 << child_idx;
    }

    pub fn get_child_mask(&self, base: usize) -> Option<u64> {
        if base >= self.nodes.len() {
            return None;
        }
        Some(self.nodes[base].body.child_mask)
    }

     pub fn get_grandchild_mask(&self, base: usize, child_idx: usize) -> Option<u64> {
         let Some(child) = self.get_child_index(base, child_idx) else {
            return None;
        };
        Some(self.nodes[child].body.child_mask)
    }



    pub fn get_child_mut(&mut self, base: usize, child_idx: usize) -> Option<&mut Node> {
        let Some(child) = self.get_child_index(base, child_idx) else {
            return None;
        };
        Some(&mut self.nodes[child])
    }

    pub fn get_children_indices(&self, base: usize) -> Vec<usize> {
        let mut children = Vec::new();
        let child_base = self.nodes[base].body.first_child_idx;
        let mask = self.nodes[base].body.child_mask;

        for i in 0..64 {
            if shared::maths::child_in_mask(mask, i) {
                children.push((child_base as usize) + i);
            }
        }
        children
    }

    pub fn get_child_index(&self, base: usize, child_idx: usize) -> Option<usize> {
        let child_base = self.nodes[base].body.first_child_idx;
        let mask = self.nodes[base].body.child_mask;
        if shared::maths::child_in_mask(mask, child_idx) {
            return Some(child_base as usize + child_idx);
        }
        return None;
    }
    pub fn size(&self) -> usize {
        self.nodes.len()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InternalFlags {
    Emtpy = -1,
    Interal,
    Uniform,
    RawArray,
    CompactArray,
}
impl Default for InternalFlags {
    fn default() -> InternalFlags {
        InternalFlags::Interal
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Node {
    pub body: NodeBody,          // Struct we will send to gpu.
    pub dag_hash: u64,           // only used for DAGify, not serialized
    pub depth: u8,               // depth of current node in tree
    pub internal: InternalFlags, // used in simplifying internal proccess.
    pub reserved: u16,
}

impl Node {
    fn new(depth: usize) -> Self {
        Self {
            body: NodeBody::default(),
            dag_hash: 0,
            depth: depth as u8,
            internal: InternalFlags::default(),
            reserved: 0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NodeBody {
    pub child_mask: u64,      // 64 children (or 8, depending on branching factor)
    pub first_child_idx: u32, // base index into flat array, or NO_CHILD
    pub offset_or_flags: u32, // bitfields: if leaf -> offset into leaf attribute buffer; if internal -> flags
}

impl Default for NodeBody {
    fn default() -> Self {
        Self {
            child_mask: 0,
            first_child_idx: NO_CHILD,
            offset_or_flags: leaf::NO_LEAF,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DagNode {
    pub child_mask: u64,      // 64 children (or 8, depending on branching factor)
    pub first_child_idx: u32, // base index into flat array, or NO_CHILD
    pub offset_or_flags: u32, // bitfields: if leaf -> offset into leaf attribute buffer; if internal -> flags
}

impl Default for DagNode {
    fn default() -> Self {
        Self {
            child_mask: 0,
            first_child_idx: NO_CHILD,
            offset_or_flags: leaf::NO_LEAF,
        }
    }
}
