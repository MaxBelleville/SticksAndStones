use std::{collections::HashMap, sync::Arc};

use bevy::math::*;

use super::{res, shared};

/**
    ContreeNode is a node in a tetrahexaconta tree (ConTree).
    It can either be a leaf node with a material, or a non-leaf node with children.
    Each node can have up to 64 children, representing subdivisions of the space.
*/
#[derive(Debug, Clone)]
pub struct ContreeNode {
    pool_idx: Option<usize>, //64 children for ConTree
    tree_scale: usize,       // Each node spans (tree_scale × 4)³ voxel so 4 tree_scale = 16³
    leaves: res::LeafData,   // Optional mat index
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

    pub fn set_pool_idx(&mut self, pool: Option<usize>) {
        self.pool_idx = pool;
    }
}

/**
    ContreeNodePool memory manager for an entire series of ContreeNodes
    This also proccesses all the functions nodes need as it reduces the overhead typically required
    With allocating memory children memory of reach node.
*/

#[derive(Debug, Clone)]
pub struct ContreeNodePool {
    storage: Vec<ContreeNode>,
    free_list: Vec<usize>, // stores starting indices of freed child blocks
    pool_size: usize,
}

impl Default for ContreeNodePool {
    fn default() -> Self {
        ContreeNodePool {
            storage: Vec::new(),
            free_list: Vec::new(),
            pool_size: 64,
        }
    }
}

//----------------------------------------- INIT ------------------------------------------------//
impl ContreeNodePool {
    pub fn new() -> Self {
        ContreeNodePool {
            storage: Vec::new(),
            free_list: Vec::new(),
            pool_size: 64,
        }
    }
    //---------------------------- GETTERS/ SETTERS / HELPERS ---------------------------------//

    pub fn get_children_mut(&mut self, idx: usize) -> &mut [ContreeNode] {
        &mut self.storage[idx..idx + self.pool_size]
    }

    pub fn get_children(&self, idx: usize) -> &[ContreeNode] {
        &self.storage[idx..idx + self.pool_size]
    }

    pub fn get_child(&self, pool_idx: usize, child_idx: usize) -> &ContreeNode {
        &self.storage[pool_idx + child_idx]
    }

    pub fn get_child_mut(&mut self, pool_idx: usize, child_idx: usize) -> &mut ContreeNode {
        &mut self.storage[pool_idx + child_idx]
    }

    pub fn get_material(&self, mut pool_idx: usize, point: IVec3) -> Option<u8> {
        let size = self.storage[pool_idx].tree_scale * 4;
        let max_levels = (size as f32).log(4.0) as usize;

        for depth in 0..max_levels {
            let level_size = self.pool_size / 4_usize.pow(depth as u32 + 1);
            let child_index = shared::maths::ileave3(point / level_size as i32);

            let node = self.get_child(pool_idx, child_index);

            // If this is the final level, look in leaves
            if depth == max_levels - 1 {
                return node.leaves.get(child_index);
            }

            let pool = node.pool_idx.unwrap();
            pool_idx = pool + child_index;
        }

        // Edge case: if size == 1, return root material
        None
    }

    pub fn collect_leaf_data(
        &self,
        node_idx: usize,
        base_pos: IVec3,
        extent: usize,
        out: &mut Vec<shared::res::VoxelVec>,
    ) {
        if extent == 0 {
            return;
        }
        let node = &self.storage[node_idx];

        match &node.leaves {
            res::LeafData::Uniform(mat) => {
                out.push(shared::res::VoxelVec::new(base_pos, Some(*mat), 1));
            }
            res::LeafData::Arr(array) => {
                for (i, m) in array.iter().enumerate() {
                    if let Some(val) = m {
                        let offset = shared::maths::deileave3(i, extent as i32 / 4);
                        out.push(shared::res::VoxelVec::new(base_pos + offset, Some(*val), 0));
                    }
                }
            }
            _ => {}
        }

        if self.get_is_uniform(node_idx) {
            // Find material value from any uniform child
            if let Some(pool_idx) = node.pool_idx {
                if let res::LeafData::Uniform(val) = &self.storage[pool_idx].leaves {
                    let lod_level = (extent as f32).log(4.0) as u8;
                    out.push(shared::res::VoxelVec::new(base_pos, Some(*val), lod_level));
                    return; // stop, don't expand children
                }
            }
        }

        if let Some(pool_idx) = node.pool_idx {
            let child_extent = extent / 4;
            let children = &self.storage[pool_idx..pool_idx + self.pool_size];

            for (child_idx, child) in children.iter().enumerate() {
                if !child.is_empty() {
                    let child_pos =
                        base_pos + shared::maths::deileave3(child_idx, child_extent as i32);
                    self.collect_leaf_data(pool_idx + child_idx, child_pos, child_extent, out);
                }
            }
        }
    }

    pub fn get_is_uniform(&self, node_idx: usize) -> bool {
        let node = &self.storage[node_idx];

        if let Some(pool_idx) = node.pool_idx {
            // First, recursively collapse all children
            let children = &self.storage[pool_idx..pool_idx + self.pool_size];

            for (child_idx, _child) in children.iter().enumerate() {
                if !self.get_is_uniform(child_idx + pool_idx) {
                    return false;
                }
            }

            // Attempt to find the first child's leaf data
            if let res::LeafData::Uniform(val) = &children[0].leaves {
                let not_uniform = children
                    .iter()
                    .any(|child| child.leaves != res::LeafData::Uniform(*val));
                if not_uniform {
                    return false;
                }
            } else {
                return false;
            }
        }
        return true;
    }

    pub fn check_leave_empty(&mut self, node_idx: usize) -> bool {
        if let Some(pool_idx) = self.storage[node_idx].pool_idx {
            // First, recursively collapse all children
            for child_idx in pool_idx..pool_idx + self.pool_size {
                if !self.check_leave_empty(child_idx) {
                    return false;
                }
            }

            let children = &self.storage[pool_idx..pool_idx + self.pool_size];

            // Check if all children are Empty
            let all_empty = children
                .iter()
                .all(|child| matches!(child.leaves, res::LeafData::Empty));

            if all_empty {
                // Free the block
                self.free_list.push(pool_idx);

                // Collapse this node
                self.storage[node_idx].pool_idx = None;
                self.storage[node_idx].leaves = res::LeafData::Empty;
                return true;
            }

            return false;
        }

        // Leaf node: return true if empty
        matches!(self.storage[node_idx].leaves, res::LeafData::Empty)
    }

    //-------------------------------- ADD AND REMOVE ----------------------------------------//

    pub fn build_voxel(&mut self, mut pool_idx: usize, voxel: shared::res::VoxelVec) {
        let size = self.storage[pool_idx].tree_scale * 4;
        let max_levels = (size as f32).log(4.0) as usize;
        let stop_depth = max_levels.saturating_sub(voxel.level as usize);
        let mut is_set = false;

        for depth in 0..=max_levels {
            let levels_remaining = max_levels - depth;
            let shift = levels_remaining.saturating_sub(1) * 6;
            let child_index = if levels_remaining == 0 {
                // last level, directly localize
                shared::maths::ileave3(voxel.pos & IVec3::splat(3)) // 0..3 local within 4×4×4 leaf
            } else {
                shared::maths::child_index(voxel.morton, shift)
            };

            let mut free_block: Option<usize> = None;
            let mut need_subdivide = false;

            {
                let node = self.get_child_mut(pool_idx, 0);

                if depth == stop_depth {
                    node.leaves.set(voxel.mat, None);
                    free_block = node.pool_idx;
                    node.pool_idx = None;
                    is_set = true;
                } else if node.tree_scale < 4 {
                    node.leaves.set(voxel.mat, Some(child_index));
                    free_block = node.pool_idx;
                    node.pool_idx = None;
                    is_set = true;
                }
                if node.pool_idx.is_none() {
                    need_subdivide = true;
                }
            }

            if let Some(child_block) = free_block {
                self.free_list.push(child_block);
            }
            if is_set {
                return;
            }
            if need_subdivide {
                self.subdivide(pool_idx);
            }
            let pool = self.storage[pool_idx].pool_idx.unwrap();
            pool_idx = pool + child_index;
        }
    }

    pub fn build_all_voxels(
        &mut self,
        node_idx: usize,
        vox: &[shared::res::VoxelVec], // sorted by morton ascending
        lo: usize,
        hi: usize,
        size: usize,        // node extent in voxels per axis
        origin: IVec3,      // implicit offset of this node in world-normalized coords
    ) {
        let n = hi - lo;

        // Empty ⇒ mark empty + free children (if any)
        if n == 0 {
            let node = &mut self.storage[node_idx];
            node.leaves = res::LeafData::Empty;
            if let Some(pool) = node.pool_idx.take() {
                self.free_list.push(pool);
            }
            return;
        }

        // If the span is a single "uniform block" voxel whose extent exactly matches this node,
        // we can set this node to Uniform and stop.
        if n == 1 {
            let v = &vox[lo];
            let want = shared::maths::level_extent(v.level);
            if want == size {
                let node = &mut self.storage[node_idx];
                node.leaves = res::LeafData::Uniform(v.mat.unwrap_or(0));
                if let Some(pool) = node.pool_idx.take() {
                    self.free_list.push(pool);
                }
                return;
            }
        }

        // Leaf node of your layout: tree_scale == 1 ⇒ size == 4
        if size == 4 {
            let node = &mut self.storage[node_idx];

            // If we still have the single-voxel uniform case at leaf size (level==1), handle it:
            if n == 1 {
                let v = &vox[lo];
                if shared::maths::level_extent(v.level) == 4 {
                    node.leaves = res::LeafData::Uniform(v.mat.unwrap_or(0));
                    if let Some(pool) = node.pool_idx.take() {
                        self.free_list.push(pool);
                    }
                    return;
                }
            }

            // Otherwise write individual leaf cells (level==0 expected here).
            for v in &vox[lo..hi] {
                // Compute local coord without mutating the voxel
                let local = v.pos - origin;           // should be in [0..4)^3
                debug_assert!(local.x >= 0 && local.x < 4
                        && local.y >= 0 && local.y < 4
                        && local.z >= 0 && local.z < 4);
                let child_index = shared::maths::ileave3(local) as usize;
                node.leaves.set(v.mat, Some(child_index));
            }

            if let Some(pool) = node.pool_idx.take() {
                self.free_list.push(pool);
            }
            return;
        }

        // Internal node: subdivide (allocate 64 children) if needed
        if self.storage[node_idx].pool_idx.is_none() {
            self.subdivide(node_idx);
        }
        let base = self.storage[node_idx].pool_idx.unwrap();

        let child_size = size / 4;
        let shift = (((size.trailing_zeros() / 2) - 1) * 6) as usize; 

        // Walk contiguous spans that map to the same child (radix-style by top 6 Morton bits)
        let mut i = lo;
        while i < hi {
            let first_ci = shared::maths::child_index(vox[i].morton, shift);

            // Span [i, j) for this child
            let mut j = i;
            while j < hi && shared::maths::child_index(vox[j].morton, shift) == first_ci {
                j += 1;
            }

            // If this span is exactly one uniform voxel that covers the child node,
            // we can set the child to Uniform without descending further.
            if j - i == 1 {
                let v = &vox[i];
                if shared::maths::level_extent(v.level) == child_size {
                    let child_idx = base + first_ci;
                    let child = &mut self.storage[child_idx];
                    child.leaves = res::LeafData::Uniform(v.mat.unwrap_or(0));
                    if let Some(pool) = child.pool_idx.take() {
                        self.free_list.push(pool);
                    }
                    i = j;
                    continue;
                }
            }

            // Otherwise descend into that child with an implicit origin
            let child_origin = origin + shared::maths::deileave3(first_ci, child_size as i32);
            self.build_all_voxels(
                base + first_ci,
                vox,
                i,
                j,
                child_size,
                child_origin,
            );

            i = j;
        }
    }
    pub fn subdivide(&mut self, pool_idx: usize) {
        if self.storage[pool_idx].tree_scale < 4 {
            eprintln!("Out of bounds");
            return;
        }

        if self.storage[pool_idx].pool_idx.is_some() {
            return; // already subdivided
        }

        let child_size = self.storage[pool_idx].tree_scale / 4;

        // Store the offset into the pool
        let new_block = if let Some(free_idx) = self.free_list.pop() {
            // Reset old children
            for i in free_idx..free_idx + self.pool_size {
                self.storage[i] = ContreeNode::new(child_size);
            }
            free_idx
        } else {
            self.alloc_children(child_size)
        };

        self.storage[pool_idx].pool_idx = Some(new_block);
    }

    /// Allocate 64 children at once, return a mutable slice
    pub fn alloc_children(&mut self, tree_scale: usize) -> usize {
        let start = self.storage.len();
        self.storage.reserve(self.pool_size);

        for _ in 0..self.pool_size {
            self.storage.push(ContreeNode::new(tree_scale));
        }

        start
    }

    pub fn alloc_node(&mut self, tree_scale: usize) -> usize {
        let start = self.storage.len();
        self.storage.push(ContreeNode::new(tree_scale));

        start
    }

    //------------------------------------- OPTOMIZE -------------------------------------------//

    pub fn trim_single_64(&mut self, root_idx: usize) -> IVec3 {
        let current_idx = root_idx;
        let mut offset = IVec3::ZERO;

        // old_min computed from the original root extent (matches original node-based impl)
        let initial_extent = self.storage[current_idx].get_voxel_extent();
        let old_min = IVec3::splat(-(initial_extent as i32) / 2);

        loop {
            let root_size = self.storage[current_idx].get_voxel_extent();

            // Stop if the node is too small to subdivide (size == 1 or <4)
            if root_size < 4 {
                break;
            }

            let pool_idx = match self.storage[current_idx].pool_idx {
                Some(p) => p,
                None => break, // no children
            };

            // Sanity: parent should have been allocated before its children
            debug_assert!(
                current_idx < pool_idx,
                "parent index should be before its children"
            );

            // Split storage into two non-overlapping mutable slices so we can mutably
            // borrow the parent and the children at the same time.
            let (head, tail) = self.storage.split_at_mut(pool_idx);
            let children = &mut tail[0..self.pool_size];

            // Find the single non-empty child (if any)
            let mut non_empty: Option<usize> = None;
            for (i, child) in children.iter().enumerate() {
                if !child.is_empty() {
                    if non_empty.is_some() {
                        // More than one non-empty child -> stop trimming and return offset
                        let size = self.storage[current_idx].get_voxel_extent();
                        return old_min + offset - IVec3::splat(-(size as i32) / 2);
                    }
                    non_empty = Some(i);
                }
            }

            let child_i = match non_empty {
                Some(i) => i,
                None => break, // all children empty
            };

            // Child extent / width (same arithmetic as original)
            let child_width = root_size / 4;

            // Add child's spatial offset (deileave3 gives the child coords)
            offset += shared::maths::deileave3(child_i, child_width as i32);

            // Move the child node into the parent slot and replace the child slot with an empty node
            // (we use mem::replace so we don't violate borrows)
            let moved_child =
                std::mem::replace(&mut children[child_i], ContreeNode::new(child_width));
            // place moved_child into the parent's slot (head[current_idx] is the parent)
            let _tmp = std::mem::replace(&mut head[current_idx], moved_child);

            // Loop continues with the same current_idx (we physically replaced the parent node
            // content with the child content, matching the behavior of the original function).
        }

        // final adjustment (same as original node implementation)
        let size = self.storage[current_idx].get_voxel_extent();
        old_min + offset - IVec3::splat(-(size as i32) / 2)
    }

    // Recursively collapses fully filled subtrees into a single node.
    pub fn collapse_filled(&mut self, node_idx: usize, chunk_size: usize) {
        if self.storage[node_idx].tree_scale == chunk_size / 4 {
            return;
        }

        if let Some(pool_idx) = self.storage[node_idx].pool_idx {
            // First, recursively collapse all children

            for child_idx in pool_idx..pool_idx + self.pool_size {
                self.collapse_filled(child_idx, chunk_size);
            }
            let children = &self.storage[pool_idx..pool_idx + self.pool_size];
            // Attempt to find the first child's leaf data
            if let res::LeafData::Uniform(val) = &children[0].leaves {
                let all_uniform = children
                    .iter()
                    .all(|child| child.leaves == res::LeafData::Uniform(*val));
                if all_uniform {
                    self.free_list.push(pool_idx);
                    self.storage[node_idx].leaves = res::LeafData::Uniform(*val);
                    self.storage[node_idx].pool_idx = None;
                }
            }
        }
    }

    //----------------------------------------- DAG ------------------------------------------------//

    /// Returns the canonical Arc-ed DAG node, inserting into cache if new.
    pub fn dagify(
        &self,
        node_idx: usize,
        cache: &mut HashMap<u64, Arc<DagNode>>,
    ) -> (Arc<DagNode>, u64) {
        let node = &self.storage[node_idx];

        // Case 1: leaf / no children
        if node.pool_idx.is_none() {
            let hash = res::hash_node(&node.leaves, &[]);
            if let Some(existing) = cache.get(&hash) {
                return (Arc::clone(existing), hash);
            }
            let new_node = Arc::new(DagNode {
                leaves: node.leaves.clone(),
                tree_scale: node.tree_scale,
                children: None,
                hash,
            });
            cache.insert(hash, Arc::clone(&new_node));
            return (new_node, hash);
        }

        // Case 2: has children
        let pool_idx = node.pool_idx.unwrap();
        let mut new_children: [Arc<DagNode>; 64] =
            std::array::from_fn(|_| Arc::new(DagNode::default()));
        let mut child_hashes = [0u64; 64];

        for i in 0..64 {
            let (child_arc, child_hash) = self.dagify(pool_idx + i, cache);
            child_hashes[i] = child_hash;
            new_children[i] = child_arc;
        }
        let hash = res::hash_node(&node.leaves, &child_hashes);

        let new_node = DagNode {
            leaves: node.leaves.clone(),
            tree_scale: node.tree_scale,
            children: Some(new_children),
            hash,
        };

        if let Some(existing) = cache.get(&hash) {
            return (Arc::clone(existing), hash);
        }

        let canonical = Arc::new(new_node);
        cache.insert(hash, Arc::clone(&canonical));
        (canonical, hash)
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

    pub fn set_size(&mut self, size: usize) {
        self.tree_scale = size / 4;
    }

    pub fn get_children(&self) -> Option<&[Arc<DagNode>; 64]> {
        self.children.as_ref()
    }

    pub fn get_children_mut(&mut self) -> Option<&mut [Arc<DagNode>; 64]> {
        self.children.as_mut()
    }

    pub fn set_material(&mut self, value: Option<u8>, index: Option<usize>) {
        self.leaves.set(value, index);
    }

    pub fn set_leaves(&mut self, leaves: res::LeafData) {
        self.leaves = leaves;
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
