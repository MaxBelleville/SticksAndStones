
use std::collections::HashMap;
use std::sync::Arc;

use bevy::math::*;
use super::node;
use super::res;
use super::shared;
/**
    ContreeNodePool memory manager for an entire series of ContreeNodes
    This also proccesses all the functions nodes need as it reduces the overhead typically required
    With allocating memory children memory of reach node.
*/



#[derive(Debug)]
pub struct ContreeNodePool {
    storage: Vec<Option<node::ContreeNode>>,
    free_list: Vec<usize>, // stores starting indices of freed child blocks
    pool_size: usize,
    next_free_block: usize,
}

//----------------------------------------- INIT ------------------------------------------------//
impl ContreeNodePool {
    pub fn new(estimate_nodes:usize,) -> Self {
        let mut storage = Vec::with_capacity(estimate_nodes);
        storage.resize(estimate_nodes, None);
        ContreeNodePool {
            storage,
            free_list: Vec::new(),
            pool_size: 64,
            next_free_block: 0,
        }
    }


    //---------------------------- GETTERS/ SETTERS / HELPERS ---------------------------------//

    pub fn get_children_mut(&mut self, idx: usize) -> impl Iterator<Item = &mut node::ContreeNode> {
        self.storage[idx..idx + self.pool_size]
            .iter_mut()
            .map(|opt| opt.as_mut().expect("Child not initialized"))
    }

    pub fn get_children(&self, idx: usize) -> impl Iterator<Item = &node::ContreeNode> {
        self.storage[idx..idx + self.pool_size]
            .iter()
            .map(|opt| opt.as_ref().expect("Child not initialized"))
    }

    pub fn get_child(&self, pool_idx: usize) -> &node::ContreeNode {
        self.storage[pool_idx]
        .as_ref()
        .expect("Tried to access uninitialized node!")
    }

    /// Access a child node mutably
    pub fn get_child_mut(&mut self, pool_idx: usize) -> &mut node::ContreeNode {
         self.storage[pool_idx]
        .as_mut()
        .expect("Tried to access uninitialized node!")
    }

    pub fn get_material(&self, mut pool_idx: usize, point: IVec3) -> Option<u8> {
        let size = self.get_child(pool_idx).get_voxel_extent();
        let max_levels = (size as f32).log(4.0) as usize;

        for depth in 0..max_levels {
            let level_size = self.pool_size / 4_usize.pow(depth as u32 + 1);
            let child_index = shared::maths::ileave3(point / level_size as i32);

            let node = self.get_child(pool_idx+child_index);

            // If this is the final level, look in leaves
            if depth == max_levels - 1 {
                return node.get_leaves().get(child_index);
            }

            let pool = node.get_pool_idx().unwrap();
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
        let node = self.get_child(node_idx);

        match &node.get_leaves() {
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
            if let Some(pool_idx) = node.get_pool_idx() {
                if let res::LeafData::Uniform(val) =  self.get_child(pool_idx).get_leaves() {
                    let lod_level = (extent as f32).log(4.0) as u8;
                    out.push(shared::res::VoxelVec::new(base_pos, Some(*val), lod_level));
                    return; // stop, don't expand children
                }
            }
        }

        if let Some(pool_idx) = node.get_pool_idx() {
            let child_extent = extent / 4;

            for (child_idx, child) in self.get_children(pool_idx).enumerate() {
                 if !child.is_empty() {
                    let child_pos =
                        base_pos + shared::maths::deileave3(child_idx, child_extent as i32);
                    self.collect_leaf_data(pool_idx + child_idx, child_pos, child_extent, out);
                }
               
            }
        }
    }

    pub fn get_is_uniform(&self, node_idx: usize) -> bool {
        let node = &self.get_child(node_idx);

        if let Some(pool_idx) = node.get_pool_idx() {
            // First, recursively collapse all children
            let children = &self.storage[pool_idx..pool_idx + self.pool_size];

            for (child_idx, _child) in children.iter().enumerate() {
                if !self.get_is_uniform(child_idx + pool_idx) {
                    return false;
                }
            }
            let Some(first_child) = &children[0] else { panic!("Not initialized"); };
            // Attempt to find the first child's leaf data
            if let res::LeafData::Uniform(val) = &first_child.get_leaves() {
                let not_uniform = self.get_children(pool_idx)
                    .any(|child| {
                        child.get_leaves() != &res::LeafData::Uniform(*val)
                    });
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
        if let Some(pool_idx) = self.get_child(node_idx).get_pool_idx() {
            // First, recursively collapse all children
            for child_idx in pool_idx..pool_idx + self.pool_size {
                if !self.check_leave_empty(child_idx) {
                    return false;
                }
            }

            // Check if all children are Empty
            let all_empty = self.get_children(pool_idx)
                .all(|child| {
                    matches!(child.get_leaves(), res::LeafData::Empty)});

            if all_empty {
                // Free the block
                self.free_list.push(pool_idx);

                // Collapse this node
                self.get_child_mut(node_idx).set_to_leaf(res::LeafData::Empty);
                return true;
            }

            return false;
        }

        // Leaf node: return true if empty
        matches!(self.get_child(node_idx).get_leaves(), res::LeafData::Empty)
    }

    //-------------------------------- ADD AND REMOVE ----------------------------------------//

    pub fn build_voxel(&mut self, mut pool_idx: usize, voxel: shared::res::VoxelVec) {
        let size = self.get_child(pool_idx).get_voxel_extent();
        let max_levels = shared::maths::log4(size);
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

       
            let node = self.get_child_mut(pool_idx);

            if depth == stop_depth {
                node.get_leaves_mut().set(voxel.mat, None);
                free_block = node.get_pool_idx();
                node.set_pool_idx(None);
                is_set = true;
            } else if node.get_logical_size() < 4 {
                node.get_leaves_mut().set(voxel.mat, Some(child_index));
                free_block = node.get_pool_idx();
                node.set_pool_idx(None);
                is_set = true;
            }
            if node.get_pool_idx().is_none() {
                need_subdivide = true;
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
            let pool = self.get_child(pool_idx).get_pool_idx().unwrap();
            pool_idx = pool + child_index;
        }
    }

    pub fn build_all_voxels(
        &mut self,
        node_idx: usize,
        vox: &[shared::res::VoxelVec], // sorted by morton ascending
        lo: usize,
        hi: usize,
        size: usize,   // node extent in voxels per axis
        origin: IVec3, // implicit offset of this node in world-normalized coords
    ) {
        let n = hi - lo;
        // Empty ⇒ mark empty + free children (if any)
        if n == 0 {
            if let Some(pool) = self.get_child(node_idx).get_pool_idx() {
                self.free_list.push(pool);
            }
             self.get_child_mut(node_idx).set_to_leaf(res::LeafData::Empty);
            return;
        }

        // If the span is a single "uniform block" voxel whose extent exactly matches this node,
        // we can set this node to Uniform and stop.
        if n == 1 {
            let v = &vox[lo];
            let want = shared::maths::level_extent(v.level);
            if want == size {    
                if let Some(pool) = self.get_child(node_idx).get_pool_idx() {
                    self.free_list.push(pool);
                }
                self.get_child_mut(node_idx).set_to_leaf(res::LeafData::Uniform(v.mat.unwrap_or(0)));
                return;
            }
        }

        // Leaf node of your layout: tree_scale == 1 ⇒ size == 4
        if size == 4 {
            // If we still have the single-voxel uniform case at leaf size (level==1), handle it:
            if n == 1 {
                let v = &vox[lo];
                if shared::maths::level_extent(v.level) == 4 {
                   
                    if let Some(pool) = self.get_child(node_idx).get_pool_idx() {
                        self.free_list.push(pool);
                    }
                    self.get_child_mut(node_idx).set_to_leaf(res::LeafData::Uniform(v.mat.unwrap_or(0)));
                    return;
                }
            }

            // Otherwise write individual leaf cells (level==0 expected here).
            for v in &vox[lo..hi] {
                // Compute local coord without mutating the voxel
                let local = v.pos - origin; // should be in [0..4)^3
                debug_assert!(
                    local.x >= 0
                        && local.x < 4
                        && local.y >= 0
                        && local.y < 4
                        && local.z >= 0
                        && local.z < 4
                );
                let child_index = shared::maths::ileave3(local) as usize;
                self.get_child_mut(node_idx).get_leaves_mut().set(v.mat, Some(child_index));
            }

            if let Some(pool) = self.get_child(node_idx).get_pool_idx() {
                self.free_list.push(pool);
            }
            self.get_child_mut(node_idx).set_pool_idx(None);
            return;
        }
        // Internal node: subdivide (allocate 64 children) if needed
        if  self.get_child(node_idx).get_pool_idx().is_none() {
            self.subdivide(node_idx);
        }
        let base =  self.get_child(node_idx).get_pool_idx().unwrap();

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
            let child_origin = origin + shared::maths::deileave3(first_ci, child_size as i32);

            // --- Early-out uniform span (gap-free) ---
            // Only attempt if child is small and voxel span could possibly fill it
            let span_len = j - i;
            let max_cells = child_size * child_size * child_size;
            let full_mask: u64 = (!0u64) >> (64 - max_cells.min(64));
            if child_size <= 4 && span_len >= max_cells {
                let mut mask: u64 = 0;
                let mut uniform_mat: Option<u8> = None;
                let mut full = true;

                'outer: for v in &vox[i..j] {
                    let extent = shared::maths::level_extent(v.level) as i32;
                    let local_min = v.pos - child_origin;
                    let local_max = local_min + IVec3::splat(extent - 1);

                    // bounds check
                    if local_min.cmplt(IVec3::ZERO).any()
                        || local_max.cmpge(IVec3::splat(child_size as i32)).any()
                    {
                        full = false;
                        break;
                    }

                    for x in local_min.x..=local_max.x {
                        for y in local_min.y..=local_max.y {
                            for z in local_min.z..=local_max.z {
                                let idx = shared::maths::ileave3(IVec3::new(x, y, z)) as usize;
                                let bit = 1u64 << idx;

                                if (mask & bit) != 0 {
                                    if uniform_mat != v.mat {
                                        full = false;
                                        break 'outer;
                                    }
                                } else {
                                    mask |= bit;
                                    if let Some(m) = v.mat {
                                        if uniform_mat.is_none() {
                                            uniform_mat = Some(m);
                                        } else if uniform_mat != v.mat {
                                            full = false;
                                            break 'outer;
                                        }
                                    }
                            
                                    if mask == full_mask {
                                        break 'outer;
                                    }
                                }
                            }
                        }
                    }
                }

                if full && mask == full_mask {
                    let child_idx = base + first_ci;
                   
                    if let Some(pool) = self.get_child(child_idx).get_pool_idx() {
                        self.free_list.push(pool);
                    }
                    self.get_child_mut(child_idx).set_to_leaf(res::LeafData::Uniform(uniform_mat.unwrap_or(0)));
                    i = j;
                    continue;
                }
            }

            // If this span is exactly one uniform voxel that covers the child node,
            // we can set the child to Uniform without descending further.
            if j - i == 1 {
                let v = &vox[i];
                if shared::maths::level_extent(v.level) == child_size {
                    let child_idx = base + first_ci;
                    if let Some(pool) = self.get_child(child_idx).get_pool_idx() {
                        self.free_list.push(pool);
                    }
                    self.get_child_mut(child_idx).set_to_leaf(res::LeafData::Uniform(v.mat.unwrap_or(0)));
                    i = j;
                    continue;
                }
            }

            // Otherwise descend into that child with an implicit origin
            let child_origin = origin + shared::maths::deileave3(first_ci, child_size as i32);
            self.build_all_voxels(base + first_ci, vox, i, j, child_size, child_origin);

            i = j;
        }
    }
    pub fn subdivide(&mut self, pool_idx: usize) {
        if self.get_child(pool_idx).get_logical_size() < 4 {
            eprintln!("Out of bounds");
            return;
        }

        if self.get_child(pool_idx).get_pool_idx().is_some() {
            return; // already subdivided
        }

        let child_size = self.get_child(pool_idx).get_logical_size() / 4;

        // --- Step 1: Try free_list (best reuse) ---
        if let Some(free_idx) = self.free_list.pop() {
            for i in free_idx..free_idx + self.pool_size {
                self.storage[i] = Some(node::ContreeNode::new(child_size));
            }
            self.get_child_mut(pool_idx).set_pool_idx(Some(free_idx));
            return;
        }

        // --- Step 2: Try preallocated space ---
        if self.next_free_block + self.pool_size <= self.storage.len() {
            let idx = self.next_free_block;
            self.next_free_block += self.pool_size;

            for i in idx..idx + self.pool_size {
                self.storage[i] = Some(node::ContreeNode::new(child_size));
            }

            self.get_child_mut(pool_idx).set_pool_idx(Some(idx));
            return;
        }

        // --- Step 3: Fallback: dynamically grow Vec ---
        let idx = self.storage.len();
        self.storage.reserve(self.pool_size);
        for _ in 0..self.pool_size {
            self.storage.push(Some(node::ContreeNode::new(child_size)));
        }

        self.get_child_mut(pool_idx).set_pool_idx(Some(idx));
    }

    /// Allocate 64 children at once, return a mutable slice
    pub fn get_root_node(&mut self, tree_scale: usize) -> usize {
        // root is always one node, so same strategy as above
        if self.next_free_block < self.storage.len() {
            let idx = self.next_free_block;
            self.next_free_block += 1;
            self.storage[idx] = Some(node::ContreeNode::new(tree_scale));
            idx
        } else {
            let idx = self.storage.len();
            self.storage.push(Some(node::ContreeNode::new(tree_scale)));
            idx
        }
    }
    /// Get the next 64-node children block (already preallocated)
    pub fn get_next_children_block(&mut self, tree_scale: usize) -> usize {
        // reuse, preallocate, fallback (same as subdivide)
        if let Some(free_idx) = self.free_list.pop() {
            for i in free_idx..free_idx + self.pool_size {
                self.storage[i] = Some(node::ContreeNode::new(tree_scale));
            }
            return free_idx;
        }

        if self.next_free_block + self.pool_size <= self.storage.len() {
            let idx = self.next_free_block;
            self.next_free_block += self.pool_size;
            for i in idx..idx + self.pool_size {
                self.storage[i] = Some(node::ContreeNode::new(tree_scale));
            }
            return idx;
        }

        let idx = self.storage.len();
        self.storage.reserve(self.pool_size);
        for _ in 0..self.pool_size {
            self.storage.push(Some(node::ContreeNode::new(tree_scale)));
        }
        idx
    }


    //------------------------------------- OPTOMIZE -------------------------------------------//

    pub fn trim_single_64(&mut self, root_idx: usize) -> IVec3 {
        let current_idx = root_idx;
        let mut offset = IVec3::ZERO;

        // old_min computed from the original root extent (matches original node-based impl)
        
        let initial_extent = self.get_child(current_idx).get_voxel_extent();
        let old_min = IVec3::splat(-(initial_extent as i32) / 2);

        loop {
            let root_size = self.get_child(current_idx).get_voxel_extent();

            // Stop if the node is too small to subdivide (size == 1 or <4)
            if root_size < 4 {
                break;
            }

            let pool_idx = match self.get_child(current_idx).get_pool_idx() {
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
            for (i, opt_child) in children.iter().enumerate() {
                let child = opt_child.as_ref().expect("Nope");
                if !child.is_empty() {
                    if non_empty.is_some() {
                        // More than one non-empty child -> stop trimming and return offset
                        let size = self.get_child(current_idx).get_voxel_extent();
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
                std::mem::replace(&mut children[child_i], Some(node::ContreeNode::new(child_width)));
            // place moved_child into the parent's slot (head[current_idx] is the parent)
            let _tmp = std::mem::replace(&mut head[current_idx], moved_child);

            // Loop continues with the same current_idx (we physically replaced the parent node
            // content with the child content, matching the behavior of the original function).
        }

        // final adjustment (same as original node implementation)
        let size = self.get_child(current_idx).get_voxel_extent();
        old_min + offset - IVec3::splat(-(size as i32) / 2)
    }

    // Recursively collapses fully filled subtrees into a single node.
    pub fn collapse_filled(&mut self, node_idx: usize) {
        if self.get_child(node_idx).get_logical_size() == 64 / 4 {
            return;
        }

        if let Some(pool_idx) = self.get_child(node_idx).get_pool_idx() {
            // Recursively collapse children
            for child_idx in pool_idx..pool_idx + self.pool_size {
                self.collapse_filled(child_idx);
            }

            // Extract the data we need without keeping children borrowed
            let mut all_uniform = false;
            let mut val = 0u8; // or whatever type

            if let res::LeafData::Uniform(first_val) =
               self.get_child(pool_idx).get_leaves()
            {
                val = *first_val;
                all_uniform = (pool_idx..pool_idx + self.pool_size)
                    .all(|i| self.get_child(i).get_leaves() == &res::LeafData::Uniform(val));
            }

            if all_uniform {
                self.free_list.push(pool_idx);
                self.get_child_mut(node_idx).set_to_leaf(res::LeafData::Uniform(val));
            }
        }
    }

    //----------------------------------------- DAG ------------------------------------------------//

    /// Returns the canonical Arc-ed DAG node, inserting into cache if new.
    pub fn dagify(
        &self,
        node_idx: usize,
        cache: &mut HashMap<u64, Arc<node::DagNode>>,
    ) -> (Arc<node::DagNode>, u64) {
        let node = &self.get_child(node_idx);

        // Case 1: leaf / no children
        if node.get_pool_idx().is_none() {
            let hash = res::hash_node(node.get_leaves(), &[]);
            if let Some(existing) = cache.get(&hash) {
                return (Arc::clone(existing), hash);
            }
            let new_node = Arc::new(node::DagNode {
                leaves: node.get_leaves().clone(),
                tree_scale: node.get_logical_size(),
                children: None,
                hash,
            });
            cache.insert(hash, Arc::clone(&new_node));
            return (new_node, hash);
        }

        // Case 2: has children
        let pool_idx = node.get_pool_idx().unwrap();
        let mut new_children: [Arc<node::DagNode>; 64] =
            std::array::from_fn(|_| Arc::new(node::DagNode::default()));
        let mut child_hashes = [0u64; 64];

        for i in 0..64 {
            let (child_arc, child_hash) = self.dagify(pool_idx + i, cache);
            child_hashes[i] = child_hash;
            new_children[i] = child_arc;
        }
        let hash = res::hash_node(node.get_leaves(), &child_hashes);

        let new_node = node::DagNode {
            leaves: node.get_leaves().clone(),
            tree_scale: node.get_logical_size(),
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
