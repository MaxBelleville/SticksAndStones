use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    sync::Arc,
};
use twox_hash::XxHash64;

use bevy::math::*;

use super::{res, serialize, shared};

#[derive(Debug, Clone)]
pub struct ContreeNode {
    children: Option<Box<[ContreeNode; 64]>>, //64 children for ConTree
    update_cache: bool, // Flag to indicate if the node is dirty and needs to be serialized again
    tree_scale: usize,  // Each node spans (tree_scale × 4)³ voxel so 4 tree_scale = 16³
    leaves: res::LeafData, // Optional mat index
    serialized_cache: Option<Vec<u8>>,
    gpu_nodes_cache: Option<Vec<res::SvoGpuNode>>,
}
//----------------------------------------- INIT ------------------------------------------------//
impl Default for ContreeNode {
    fn default() -> Self {
        ContreeNode {
            children: None,
            update_cache: false,
            tree_scale: 1, // Default size of the root node
            leaves: res::LeafData::Empty,
            serialized_cache: None, // Cache for serialized data
            gpu_nodes_cache: None,  // Cache for GPU nodes
        }
    }
}

impl ContreeNode {
    pub fn new(tree_scale: usize) -> Self {
        ContreeNode {
            children: None,
            update_cache: false, // Initially not dirty
            tree_scale,
            leaves: res::LeafData::Empty,
            serialized_cache: None, // Initialize with no cached serialization
            gpu_nodes_cache: None,  // Initialize with no cached GPU nodes
        }
    }
    //---------------------------- GETTERS/ SETTERS / HELPERS ---------------------------------//
    pub fn is_children_empty(&self) -> bool {
        self.children.is_none()
    }

    pub fn is_empty(&self) -> bool {
        self.leaves.is_empty() && self.children.is_none()
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

    pub fn get_children(&self) -> Option<&Box<[ContreeNode; 64]>> {
        self.children.as_ref()
    }

    pub fn get_children_mut(&mut self) -> Option<&mut Box<[ContreeNode; 64]>> {
        self.children.as_mut()
    }

    pub fn set_material(&mut self, value: Option<u8>, index: Option<usize>) {
        self.leaves.set(value, index);
        self.update_cache = true; // Mark as dirty when material is set
    }

    pub fn set_leaves(&mut self, leaves: res::LeafData) {
        self.leaves = leaves;
        self.update_cache = true; // Mark as dirty when material is set
    }

    pub fn set_children(&mut self, children: Option<Box<[ContreeNode; 64]>>) {
        self.children = children;
        self.update_cache = true; // Mark as dirty when material is set
    }

    pub fn update_cache(&mut self) {
        // This method can be used to mark the node as dirty
        self.update_cache = true;
    }

    pub fn get_material_local(&self, point: IVec3) -> Option<u8> {
        let mut node = self;
        let size = node.tree_scale * 4;
        let levels = (size as f32).log(4.0) as usize;

        for depth in 0..levels {
            let level_size = size / 4_usize.pow(depth as u32 + 1);
            let child_index = shared::maths::ileave3(point / level_size as i32);

            // If this is the final level, look in leaves
            if depth == levels - 1 {
                return node.leaves.get(child_index);
            }

            // Otherwise, descend
            match &node.children {
                Some(children) => {
                    node = &children[child_index as usize];
                }
                None => return None,
            }
        }

        // Edge case: if size == 1, return root material
        None
    }

    pub fn collect_leaf_data(
        &self,
        pos: IVec3,
        extent: usize,
        path: &mut Vec<u8>,
        out: &mut Vec<(IVec3, u8, u8)>,
        chunks: &mut HashMap<Vec<u8>, Box<ContreeNode>>,
    ) {
        if extent == 0 {
            return;
        }

        match &self.leaves {
            res::LeafData::Uniform(mat) => {
                out.push((pos, *mat, 0));
            }
            res::LeafData::Arr(array) => {
                for (i, m) in array.iter().enumerate() {
                    if let Some(val) = m {
                        let offset = shared::maths::deileave3(i) * (extent as i32 / 4);
                        out.push((pos + offset, *val, 0)); // or some depth
                    }
                }
            }
            _ => {}
        }

        if extent == 64 {
            chunks.insert(path.clone(), Box::new(self.clone()));
        }

        if let Some(children) = self.get_children() {
            let child_extent = extent / 4;
            for (i, child) in children.iter().enumerate() {
                if !child.is_empty() {
                    path.push(i as u8);
                    let child_pos = pos + shared::maths::deileave3(i) * child_extent as i32;
                    child.collect_leaf_data(child_pos, child_extent, path, out, chunks);
                    path.pop();
                }
            }
        }
    }

    //----------------------------------- SERAILIZE ------------------------------------------//
    pub fn serialize(&self) -> (Vec<u8>, Vec<res::SvoGpuNode>) {
        let mut bit_writer = serialize::BitstreamWriter::new();
        let mut gpu_writer = serialize::GpuNodeWriter::new();
        let mut seen_hashes = HashMap::new();
        serialize::serialize_nodes(self, &mut bit_writer, &mut gpu_writer, &mut seen_hashes);
        (bit_writer.bits, gpu_writer.get_nodes().to_vec())
    }

    pub fn deserialize(&mut self, packed_bytes: &[u8]) {
        let mut reader = serialize::BitstreamReader::new(packed_bytes);
        serialize::deserialize_node(self, &mut reader);
    }

    pub fn serialize_with_cache(
        &mut self,
        global_dag_cache: &mut HashMap<u64, (Vec<u8>, Vec<res::SvoGpuNode>)>,
    ) -> (&[u8], Vec<res::SvoGpuNode>) {
        if self.update_cache || self.serialized_cache.is_none() || self.gpu_nodes_cache.is_none() {
            let mut bit_writer = serialize::BitstreamWriter::new();
            let mut gpu_writer = serialize::GpuNodeWriter::new();
            let dagified = self.dagify(&mut HashMap::new());
            let mut seen_hashes = HashMap::new();

            serialize::serialize_nodes_dag(
                &dagified,
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
    //-------------------------------- ADD AND REMOVE ----------------------------------------//

    pub fn insert_local(&mut self, point: IVec3, material: Option<u8>, levels_to_go: u8) {
        let mut node = self;
        let size = node.tree_scale * 4;
        let max_levels = (size as f32).log(4.0) as usize;
        
        
        // Compute total levels from root down to leaves
        let stop_depth = max_levels.saturating_sub(levels_to_go as usize);
        
        for depth in 0..=max_levels {
            let level_size = size / 4_usize.pow(depth as u32 + 1);
            let local = point / level_size as i32;
            let child_index = shared::maths::ileave3(local);
            
            if depth == stop_depth || node.tree_scale < 4 {
                node.leaves.set(material, Some(child_index as usize));
                node.update_cache = true;
                break;
            }

            if node.children.is_none() {
                node.subdivide(); // <-- safe now
            }

            node = {
                let children = node.children.as_mut().unwrap();
                &mut children[child_index as usize]
            };
        }
    }

    pub fn subdivide(&mut self) {
        if self.tree_scale < 4 {
            println!(
                "Refusing to subdivide: tree_scale={} too small",
                self.tree_scale
            );
            return;
        }
        let child_size = self.tree_scale / 4;
        assert!(child_size >= 1);
        println!(
            "Subdividing node of size {} into child size {}",
            self.tree_scale, child_size
        );

        self.children = Some(Box::new(std::array::from_fn(|_| {
            ContreeNode::new(child_size)
        })));
        self.update_cache = true;
    }

    //------------------------------------- OPTOMIZE -------------------------------------------//

    // Recursively collapses fully filled subtrees into a single node.
    pub fn collapse_filled(&mut self) {
        if let Some(children) = &mut self.children {
            // First, recursively collapse all children
            for child in children.iter_mut() {
                child.collapse_filled();
            }

            // Attempt to find the first child's leaf data
            if let res::LeafData::Uniform(val) = &children[0].leaves {
                let all_uniform = children
                    .iter()
                    .all(|child| child.leaves == res::LeafData::Uniform(*val));

                if all_uniform {
                    self.leaves = res::LeafData::Uniform(*val);
                    self.children = None;
                    self.update_cache = true;
                }
            }
        }
    }

    //------------------------------------ DAG & HASH -------------------------------------------//
    pub fn hash_node(&self, child_hashes: &[u64]) -> u64 {
        let mut hasher = XxHash64::with_seed(0); // or any fixed seed
        self.leaves.hash(&mut hasher);
        child_hashes.hash(&mut hasher);
        hasher.finish()
    }

    pub fn dagify(&self, cache: &mut HashMap<u64, res::DagNode>) -> res::DagNode {
        // Base case: leaf node
        if self.children.is_none() {
            let hash = self.hash_node(&[]);
            println!("Dagify node hash: {:X} | deduplicated: {}", hash, cache.contains_key(&hash));
            if let Some(shared) = cache.get(&hash) {
                return Arc::clone(shared);
            }

            let new_node = Arc::new(self.clone());
            cache.insert(hash, Arc::clone(&new_node));
            return new_node;
        }

        // Build children and record their hashes
        let children = self.children.as_ref().unwrap();
        let mut child_hashes = [0u64; 64];
        let child_array: [ContreeNode; 64] = std::array::from_fn(|i| {
            let child = &children[i];
            let arc_node = child.dagify(cache);
            child_hashes[i] = cache
                .iter()
                .find(|(_, v)| Arc::ptr_eq(v, &arc_node))
                .map(|(k, _)| *k)
                .unwrap_or_else(|| arc_node.hash_node(&[])); // fallback in case not cached
            (*arc_node).clone()
        });

        // Construct deduplicated node
        let mut new_node = self.clone();
        new_node.children = Some(Box::new(child_array));
        let hash = new_node.hash_node(&child_hashes);
          println!("WithChild Dagify node hash: {:X} | deduplicated: {}", hash, cache.contains_key(&hash));
        if let Some(shared) = cache.get(&hash) {
            return Arc::clone(shared);
        }

        let shared_node = Arc::new(new_node);
        cache.insert(hash, Arc::clone(&shared_node));
        shared_node
    }
    //------------------------------------- DEBUGGIN -------------------------------------------//

    pub fn print_tree(&self, depth: usize, root_size: usize, path: &mut Vec<Option<u8>>) {
        let indent = "  ".repeat(depth * 2);

        self.leaves
            .print(depth, self.tree_scale, root_size, path);

        if let Some(children) = &self.children {
            println!("{}Node(size={}, depth={})", indent, self.tree_scale, depth);

            for (i, child) in children.iter().enumerate() {
                if child.is_empty() {
                    continue;
                }

                path.push(Some(i as u8));
                let pos = shared::maths::pathtrace_to_world_pos(root_size, path);
                println!("{}  Child[{}](depth={}, pos={}):", indent, i, depth, pos);
                child.print_tree(depth + 1, root_size, path);
                path.pop();
            }
        }
    }
}
