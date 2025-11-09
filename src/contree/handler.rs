use bevy::math::*;
use std::{
    borrow::Cow,
    hash::{Hash, Hasher},
};
use twox_hash::XxHash64;

use super::{leaf, node, shared};
#[derive(Debug)]
pub struct Handler {
    leaves_pool: leaf::LeafPool,
    node_pool: node::NodePool,
}

impl Handler {
    //Return self and root_idx
    pub fn new(cap_nodes: usize, depth: u8) -> (Self, usize) {
        let mut node_pool = node::NodePool::new(cap_nodes);
        let root = node_pool.alloc_node(depth);
        (
            Self {
                leaves_pool: leaf::LeafPool::new(),
                node_pool,
            },
            root,
        )
    }

    pub fn get_material(&self, local_point: IVec3, mut base: usize, max_level: usize) -> u8 {
        for depth in 0..=max_level {
            let level_size = node::SLAB_SIZE / 4_usize.pow(depth as u32 + 1);
            let child_index = shared::maths::ileave3(local_point / level_size as i32);

            if base >= self.node_pool.size() {
                return 0;
            }
            if !self.node_pool.check_alloc_children(base) {
                return 0;
            }
            let node = self.node_pool.get_node(base).unwrap();
            let leaf_indx = node.body.offset_or_flags;

            if leaf_indx != leaf::NO_LEAF {
                return self.leaves_pool.get_voxel(leaf_indx as usize, child_index);
            }

            let child_base = node.body.first_child_idx as usize;
            base = child_base + child_index;
        }
        0
    }

    pub fn is_uniform(&self, base: usize, depth: usize) -> bool {
        false
    }
    pub fn is_empty(&self, base: usize) -> bool {
        false
    }

    /*
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

     */

    pub fn collect_voxels(&self, base: usize, depth: usize) -> Vec<shared::res::Voxel> {
        Vec::new()
    }

    pub fn build_voxel(
        &mut self,
        voxel: shared::res::Voxel,
        base: usize,
        depth: usize,
        max_depth: usize,
    ) {
        let stop_depth = max_depth.saturating_sub(voxel.level as usize);

        if depth == stop_depth {
            if self.node_pool.get_first_child(base).unwrap() == node::NO_CHILD {
                self.collapse_children_of(base);
            }
            self._update_leaf(
                base,
                shared::maths::ileave3(voxel.pos & IVec3::splat(3)),
                voxel.mat,
            );
            return;
        }

        if self.node_pool.get_first_child(base).unwrap() == node::NO_CHILD {
            self.node_pool.subdivide(base, depth + 1);
            self.node_pool.reset_child_mask(base);
        }

        let shift = (max_depth - depth - 1) * 6;
        let child_idx = shared::maths::child_index(voxel.morton, shift);
        let child_base = self.node_pool.get_first_child(base).unwrap() as usize + child_idx;

        // Recurse into child
        self.build_voxel(voxel, child_base, depth + 1, max_depth);

        // Update this node's mask
        self.node_pool.set_child_mask(base, child_idx);

        // Recompute this node's dag hash
        let mut hasher = XxHash64::default();
        hasher.write_u64(self.node_pool.get_child_mask(base).unwrap());
        hasher.write_u8(self.node_pool.get_node_depth(base).unwrap());

        for child in self.node_pool.get_children(base) {
            hasher.write_u64(child.dag_hash);
        }

        self.node_pool.set_node_hash(base, hasher.finish());
    }

    pub fn build_region(
        &mut self,
        voxels: Vec<shared::res::Voxel>,
        base: usize,
        extent: usize,
        origin: IVec3,
        min_level: usize,
    ) {
        let region = SubRegion::new(voxels.as_slice(), base, extent, origin, min_level);
        self._build_region(region, base, 0, shared::maths::log4(extent));
    }

    pub fn collapse_children_of(&mut self, base: usize) {
        let freed_leaves = self.node_pool.collapse(base);
        for leaf_idx in freed_leaves {
            self.leaves_pool.free_leaf(leaf_idx);
        }
    }

    fn _update_leaf(&mut self, base: usize, child_index: usize, mat: u8) {
        let node = self.node_pool.get_node_mut(base).unwrap();
        let leaf_idx = if node.body.offset_or_flags == leaf::NO_LEAF {
            self.leaves_pool.push_leaf(leaf::LeafType::Empty)
        } else {
            node.body.offset_or_flags as usize
        };

        let leaf_type = self.leaves_pool.update_voxel(leaf_idx, child_index, mat);
        self._register_leaf(base, leaf_type);
    }

    //Need to call allocate and classify first.
    fn _register_leaf(&mut self, node_idx: usize, leaf_type: leaf::LeafType) {
        let Some(node) = self.node_pool.get_node_mut(node_idx) else {
            return;
        };

        // If the new type is Empty → free existing leaf if there is one
        if leaf_type == leaf::LeafType::Empty {
            if node.body.offset_or_flags != leaf::NO_LEAF as u32 {
                self.leaves_pool
                    .free_leaf(node.body.offset_or_flags as usize);
            }
            node.dag_hash = 0;
            node.internal = node::InternalFlags::Emtpy;
            node.body.first_child_idx = node::NO_CHILD;
            node.body.child_mask = 0;
            node.body.offset_or_flags = leaf::NO_LEAF;
            return;
        }

        //Set the hash for leaf
        let mut hasher = XxHash64::default();
        leaf_type.hash(&mut hasher);
        node.dag_hash = hasher.finish();

        // Update node flags
        match leaf_type {
            leaf::LeafType::Uniform(_) => node.internal = node::InternalFlags::Uniform,
            leaf::LeafType::Compact(_) => node.internal = node::InternalFlags::CompactArray,
            leaf::LeafType::Raw64(_) => node.internal = node::InternalFlags::RawArray,
            leaf::LeafType::Empty => node.internal = node::InternalFlags::Emtpy, // should not hit here
        }

        // Otherwise, either overwrite an existing leaf or allocate new one
        let leaf_idx = if node.body.offset_or_flags != leaf::NO_LEAF as u32 {
            // Already had a leaf → overwrite
            let idx = node.body.offset_or_flags as usize;
            self.leaves_pool.update_leaf(idx, leaf_type);
            idx
        } else {
            // Allocate new
            self.leaves_pool.push_leaf(leaf_type)
        };

        node.body.child_mask = 0;
        node.body.first_child_idx = node::NO_CHILD;
        node.body.offset_or_flags = leaf_idx as u32;
    }

    fn _build_region(&mut self, region: SubRegion, base: usize, depth: usize, max_level: usize) {
        let (pre_mat, pre_size) = self._node_is_uniform(base, region.extent);

        // uniform check with voxel volumes
        if region.is_uniform(pre_mat, pre_size) {
            let total = region.extent * region.extent * region.extent;
            if pre_size >= total {
                return;
            }

            if region.first_material() != 0 {
                let leaf_type =
                    leaf::LeafType::Uniform(leaf::LeafUniformBlob::new(region.first_material()));
                self._register_leaf(base, leaf_type);
            } else {
                self.collapse_children_of(base);
                self._register_leaf(base, leaf::LeafType::Empty);
            }

            return;
        }

        //stopping condition: extent small OR LOD of voxels says we should make leaf
        let depth_level = max_level.saturating_sub(depth); // e.g. max_depth=3, depth=0 => 3
        if region.extent <= 4 || region.min_voxel_level >= depth_level {
            if let Some(node) = self.node_pool.get_node(base) {
                if node.body.first_child_idx != node::NO_CHILD {
                    self.collapse_children_of(base);
                }
            }

            let voxels64 = region.to_voxel_array();
            let leaf_type = leaf::LeafType::classify(voxels64);
            self._register_leaf(base, leaf_type);
            return;
        }

        // else, subdivide and recurse
        if !self.node_pool.check_alloc_children(base) {
            self.node_pool.subdivide(base, depth);
        }

        let mut child_hashes = Vec::new();
        for (child_index, child_opt) in region.subdivide().into_iter().enumerate() {
            if let Some(child) = child_opt {
                let child_node_idx = self.node_pool.get_child_index(base, child_index);
                if child_node_idx == None {
                    return;
                }
                self._build_region(child, child_node_idx.unwrap(), depth + 1, max_level);

                child_hashes.push(
                    self.node_pool
                        .get_node(child_node_idx.unwrap())
                        .unwrap()
                        .dag_hash,
                );
                let node = self.node_pool.get_node_mut(base).unwrap();

                node.body.child_mask |= 1u64 << child_index;
            }
        }

        //Do the hash for the node after proccessing children
        let mut hasher = XxHash64::default();
        let node = self.node_pool.get_node_mut(base).unwrap();
        hasher.write_u64(node.body.child_mask);
        hasher.write_u8(node.depth);
        child_hashes.hash(&mut hasher);

        node.dag_hash = hasher.finish()
    }

    fn _node_is_uniform(&self, base: usize, extent: usize) -> (u8, usize) {
        let Some(node) = self.node_pool.get_node(base) else {
            return (0, 0);
        };

        // If node is internal (has children), fast-return: no preexisting uniform coverage
        if node.body.first_child_idx != node::NO_CHILD {
            return (0, 0);
        }

        // Node is a leaf or empty
        let leaf_idx = node.body.offset_or_flags as usize;
        if leaf_idx == leaf::NO_LEAF as usize {
            return (0, 0);
        }

        self.leaves_pool.is_uniform(leaf_idx, extent)
    }
}

pub struct SubRegion<'a> {
    extent: usize,
    pool_idx: usize,
    origin: IVec3,
    voxels: Cow<'a, [shared::res::Voxel]>,
    min_voxel_level: usize,
}

impl<'a> SubRegion<'a> {
    pub fn new(
        voxels: &'a [shared::res::Voxel],
        pool_idx: usize,
        extent: usize,
        origin: IVec3,
        min_voxel_level: usize,
    ) -> Self {
        Self {
            extent,
            pool_idx,
            origin,
            voxels: Cow::Borrowed(voxels),
            min_voxel_level,
        }
    }

    pub fn first_material(&self) -> u8 {
        if self.voxels.is_empty() {
            return 0;
        }
        self.voxels[0].mat
    }

    pub fn to_voxel_array(&self) -> [u8; 64] {
        assert!(self.extent == 4, "to_voxel_array only valid for extent=4");

        // Start with all air (0)
        let mut array = [0u8; 64];

        for voxel in self.voxels.iter() {
            let local_index = shared::maths::ileave3(voxel.pos & IVec3::splat(3)); // 0..3 local within 4×4×4 leaf
            array[local_index] = voxel.mat;
        }

        array
    }

    pub fn is_uniform(&self, prexisting_mat: u8, prexisting_size: usize) -> bool {
        if self.voxels.is_empty() {
            return true;
        }
        let first = self.voxels[0].mat;
        let mut volume = 0;
        for voxel in self.voxels.iter() {
            if first != voxel.mat {
                // If there is a single element wrong return immediately
                return false;
            } else {
                volume += voxel.volume();
            }
        }
        let total = self.extent * self.extent * self.extent;
        if volume + prexisting_size >= total && first == prexisting_mat && prexisting_size > 0 {
            //Prexisting+Volume makes up the full extent meaning it's a uniform cube and first is the same material as prexisting
            return true;
        }
        if volume >= total {
            //Volume size fils out the extent meaning it's a full uniform cube.
            return true;
        }
        false
    }
    pub fn subdivide(&'_ self) -> [Option<SubRegion<'_>>; 64] {
        assert!(self.extent % 4 == 0);
        let child_extent = self.extent / 4;

        let mut children: [Option<SubRegion>; 64] = std::array::from_fn(|_| None);

        // Partition voxels by child index (already contiguous if Morton-sorted)
        let mut start = 0;
        while start < self.voxels.len() {
            let child_index = shared::maths::child_index(self.voxels[start].morton, child_extent);
            let mut end = start + 1;
            while end < self.voxels.len()
                && shared::maths::child_index(self.voxels[end].morton, child_extent) == child_index
            {
                end += 1;
            }

            let offset = shared::maths::deileave3(child_index, child_extent);
            children[child_index] = Some(SubRegion {
                extent: child_extent,
                pool_idx: self.pool_idx + child_index, // depends on allocator
                origin: self.origin + offset,
                voxels: Cow::Borrowed(&self.voxels[start..end]),
                min_voxel_level: self.min_voxel_level,
            });

            start = end;
        }
        children
    }
}
