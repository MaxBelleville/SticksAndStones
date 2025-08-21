use std::{
    collections::{HashMap, VecDeque},
    u32,
};

use super::{node, res};
use bevy::ecs::query::Has;
use twox_hash::XxHash64;
pub struct BitstreamWriter {
    pub bits: Vec<u8>,
    pub bit_offset: usize,
}

impl BitstreamWriter {
    pub fn new() -> Self {
        BitstreamWriter {
            bits: Vec::new(),
            bit_offset: 0,
        }
    }

    pub fn write_bits(&mut self, mut value: u32, mut count: usize) {
        while count > 0 {
            let byte_index = self.bit_offset / 8;
            let bit_in_byte = self.bit_offset % 8;

            if byte_index >= self.bits.len() {
                self.bits.push(0);
            }

            let space_in_byte = 8 - bit_in_byte;
            let bits_to_write = count.min(space_in_byte);
            let mask = (1u32 << bits_to_write) - 1;
            let bits = (value & mask) as u8;

            self.bits[byte_index] |= bits << bit_in_byte;

            self.bit_offset += bits_to_write;
            value >>= bits_to_write;
            count -= bits_to_write;
        }
    }

    pub fn write_2bits(&mut self, value: u8) {
        self.write_bits(value as u32, 2);
    }

    pub fn write_byte(&mut self, value: u8) {
        self.align_to_byte();
        self.bits.push(value);
        self.bit_offset += 8;
    }

    pub fn write_u16(&mut self, value: u16) {
        self.align_to_byte();
        let bytes = value.to_le_bytes();
        self.bits.push(bytes[0]);
        self.bits.push(bytes[1]);
        self.bit_offset += 16;
    }

    pub fn reserve_u16(&mut self) -> usize {
        self.align_to_byte();
        let pos = self.bits.len();
        self.bits.push(0);
        self.bits.push(0);
        self.bit_offset += 16;
        pos
    }

    pub fn write_u16_at(&mut self, pos: usize, value: u16) {
        let bytes = value.to_le_bytes();
        self.bits[pos] = bytes[0];
        self.bits[pos + 1] = bytes[1];
    }

    pub fn total_bits(&self) -> usize {
        self.bit_offset
    }

    pub fn align_to_byte(&mut self) {
        let remainder = self.bit_offset % 8;
        if remainder != 0 {
            self.bit_offset += 8 - remainder;
            let byte_index = self.bit_offset / 8;
            if byte_index > self.bits.len() {
                self.bits.push(0);
            }
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.bits
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.bits
    }
}

pub struct BitstreamReader<'a> {
    pub bits: &'a [u8],
    pub bit_offset: usize,
}

impl<'a> BitstreamReader<'a> {
    pub fn new(bits: &'a [u8]) -> Self {
        BitstreamReader {
            bits,
            bit_offset: 0,
        }
    }

    pub fn get_bit_offset(&self) -> usize {
        self.bit_offset
    }

    pub fn set_bit_offset(&mut self, offset: usize) {
        self.bit_offset = offset;
    }

    pub fn read_2bits(&mut self) -> Option<u8> {
        self.read_bits(2)
    }

    pub fn read_byte(&mut self) -> Option<u8> {
        self.read_bits(8)
    }

    pub fn read_u16(&mut self) -> Option<u16> {
        let lo = self.read_bits(8)? as u16;
        let hi = self.read_bits(8)? as u16;
        Some(lo | (hi << 8))
    }

    fn read_bits(&mut self, count: usize) -> Option<u8> {
        let mut val = 0u8;
        for i in 0..count {
            let byte_index = self.bit_offset / 8;
            let bit_in_byte = self.bit_offset % 8;

            if byte_index >= self.bits.len() {
                return None;
            }

            let bit = (self.bits[byte_index] >> bit_in_byte) & 1;
            val |= bit << i;
            self.bit_offset += 1;
        }
        Some(val)
    }
}

pub struct GpuNodeWriter {
    pub nodes: Vec<res::SvtGpuNode>,
}
#[derive(Debug, Clone)]
enum NodeKind<'a> {
    Internal(&'a node::DagNode, u64),
    LeafBlock(&'a res::LeafData, u64),
}

impl GpuNodeWriter {
    pub fn new() -> Self {
        GpuNodeWriter { nodes: Vec::new() }
    }

    /// Converts a single `ContreeNode` into a `SvtGpuNode`
    pub fn make_gpu_internal(&mut self, node: &node::DagNode) {
        let mut child_mask = 0u64;
        let mut material = res::NOT_A_LEAF_FLAG;

        if let Some(children) = node.get_children() {
            for (i, child) in children.iter().enumerate() {
                if !child.is_empty() {
                    child_mask |= 1 << i;
                }
            }
        }
        else {
            //Pretty much should only happen if root has no children.
            (child_mask, material) = node.get_leaves().get_material_mask();
        }

        self.nodes.push(res::SvtGpuNode {
            child_mask,
            material,
            first_child_idx: u32::MAX, // Shouldn't matter will get overwritten
        });
    }

    pub fn make_gpu_leaf(&mut self, leaves: &res::LeafData) -> bool {
        let (child_mask, material) = leaves.get_material_mask();

        self.nodes.push(res::SvtGpuNode {
            child_mask,
            material,
            first_child_idx: 0, //Set End of the tree incase error but this shouldn't matter as it's leaf
        });
        material == res::NOT_A_LEAF_FLAG
    }

    pub fn get_nodes(&self) -> &[res::SvtGpuNode] {
        &self.nodes
    }
}

fn serialize_node_bitstream(node: &node::DagNode, bit_writer: &mut BitstreamWriter) {
    let node_type = match (node.get_children(), node.get_leaves()) {
        (None, res::LeafData::Empty) => 0b00,      // air
        (None, res::LeafData::Uniform(_)) => 0b01, // solid
        (None, res::LeafData::Arr(_)) => 0b11,     // raw leaf array
        (Some(_), _) => 0b10,                      // has children
    };

    bit_writer.write_2bits(node_type);

    if node_type == 0b01 {
        if let res::LeafData::Uniform(val) = node.get_leaves() {
            bit_writer.write_byte(*val);
        } else {
            panic!("Expected Uniform leaf in node_type == 0b01!");
        }
    }

    if node_type == 0b11 {
        let materials = node.get_leaves().get_full(); // Always 64
        for mat in materials.iter() {
            let leaf_type = match mat {
                None => 0b00,
                Some(_) => 0b01,
            };
            bit_writer.write_2bits(leaf_type);
            if let Some(mat) = mat {
                bit_writer.write_byte(*mat);
            }
        }
    }
}

pub fn serialize_nodes(
    root: &node::DagNode,
    root_hash: u64,
    bit_writer: &mut BitstreamWriter,
    gpu_writer: &mut GpuNodeWriter,
    seen_hashes: &mut HashMap<u64, usize>,
) {
    let mut internal_queue = VecDeque::new();
    let mut mixed_leaf = Vec::new();
    let mut clone_addr: HashMap<usize, Vec<usize>> = HashMap::new();

    serialize_node_bitstream(root, bit_writer);
    let idx = gpu_writer.nodes.len();
    gpu_writer.make_gpu_internal(root);
    internal_queue.push_back((root, root_hash, idx));
    seen_hashes.insert(root_hash, idx);

    while let Some((node, hash, parent_idx)) = internal_queue.pop_front() {
        let Some(children) = node.get_children() else {
            continue;
        };
        let mut items: Vec<NodeKind> = Vec::new();

        let mut child_mask: u64 = 0;

        //Collect all the children and
        for (i, child) in children.iter().enumerate() {
            if child.is_empty() {
                continue;
            } // Probably impossible
            child_mask |= 1u64 << i;
            if let Some(_grandcs) = child.get_children() {
                items.push(NodeKind::Internal(child, child.get_hash()));
            } else {
                let leaves = child.get_leaves();
                let leaf_hash = res::hash_leaf(&leaves);
                items.push(NodeKind::LeafBlock(leaves, leaf_hash));
            }
        }
        gpu_writer.nodes[parent_idx].child_mask = child_mask;

        if items.is_empty() {
            continue;
        }

        let start_idx = gpu_writer.nodes.len();
        gpu_writer.nodes[parent_idx].first_child_idx = start_idx as u32;
       
        if let Some(pending) = clone_addr.remove(&parent_idx) {
            for alias in pending {
                gpu_writer.nodes[alias].first_child_idx = start_idx as u32;
            }
           
        }

        for kind in items {
            match kind {
                NodeKind::Internal(node, ch_hash) => {
                    serialize_node_bitstream(node, bit_writer);
                    let idx = gpu_writer.nodes.len();
                    if let Some(&existing_idx) = seen_hashes.get(&ch_hash) {
                       clone_addr.entry(existing_idx).or_default().push(idx);
                     
                       gpu_writer.nodes.push(gpu_writer.nodes[existing_idx]);
                       continue;
                    }

                    gpu_writer.make_gpu_internal(node);
                    internal_queue.push_back((node, ch_hash, idx));
                    seen_hashes.insert(ch_hash, idx);
                }
                NodeKind::LeafBlock(leaves, leaf_hash) => {
                    let idx = gpu_writer.nodes.len();
                    if gpu_writer.make_gpu_leaf(leaves) {
                        //If mixed
                        mixed_leaf.push((leaves, leaf_hash, idx));
                    }
                }
            }
        }
    }

    //Handle case were it's mixed leafs.
    for (leaves, hash, pidx) in mixed_leaf {
        if let Some(&existing_idx) = seen_hashes.get(&hash) {
            gpu_writer.nodes[pidx].first_child_idx = existing_idx as u32;
            continue;
        }

        let node_idx = gpu_writer.nodes.len();
        gpu_writer.nodes[pidx].first_child_idx = node_idx as u32;

        for mat in leaves.get_full().iter().filter_map(|&m| m) {
            gpu_writer.nodes.push(res::SvtGpuNode {
                material: mat as u32,
                ..Default::default()
            });
        }
        seen_hashes.insert(hash, node_idx);
    }
}
// pub fn deserialize_node(node: &mut node::DagNode, reader: &mut BitstreamReader) {
//     if let Some(node_type) = reader.read_2bits() {
//         match node_type {
//             // 0b00 = Air (Empty)
//             0b00 => {
//                 node.set_leaves(res::LeafData::Empty);
//                 node.set_children(None);
//             }

//             // 0b01 = Solid (Uniform material)
//             0b01 => {
//                 if let Some(mat) = reader.read_byte() {
//                     node.set_leaves(res::LeafData::Uniform(mat));
//                     node.set_children(None);
//                 } else {
//                     // Fallback: corrupted stream, treat as empty
//                     node.set_leaves(res::LeafData::Empty);
//                     node.set_children(None);
//                 }
//             }

//             // 0b10 = Inner node (has children)
//             0b10 => {
//                 node.subdivide();
//                 if let Some(children) = node.get_children_mut() {
//                     for child in children.iter_mut() {
//                         deserialize_node(child, reader);
//                     }
//                 }
//                 node.set_leaves(res::LeafData::Empty); // No leaf data when inner node
//             }

//             // 0b11 = Raw leaf array (64 voxels, 2-bit tags + optional byte)
//             0b11 => {
//                 let mut leaves = Box::new([None; 64]);

//                 for i in 0..64 {
//                     let leaf_type = reader.read_2bits();
//                     match leaf_type {
//                         Some(0b00) => {
//                             // Air: leave as None
//                         }
//                         Some(0b01) => {
//                             if let Some(mat) = reader.read_byte() {
//                                 leaves[i] = Some(mat);
//                             } else {
//                                 // Truncated stream: fallback
//                                 leaves[i] = None;
//                             }
//                         }
//                         _ => {
//                             // Invalid leaf type (reserved or corrupt)
//                             leaves[i] = None;
//                         }
//                     }
//                 }

//                 node.set_leaves(res::LeafData::Arr(leaves));
//                 node.set_children(None);
//             }

//             _ => {
//                 // Invalid type — fallback to air
//                 node.set_leaves(res::LeafData::Empty);
//                 node.set_children(None);
//             }
//         }
//     } else {
//         // Corrupted stream or EOF
//         node.set_leaves(res::LeafData::Empty);
//         node.set_children(None);
//     }
// }

pub fn serialize_nodes_dag(
    root: &node::DagNode,
    root_hash: u64,
    bit_writer: &mut BitstreamWriter,
    gpu_writer: &mut GpuNodeWriter,
    dag_cache: &mut HashMap<u64, (Vec<u8>, Vec<res::SvtGpuNode>)>,
    seen_hashes: &mut HashMap<u64, usize>,
) {
    if let Some((cached_bits, cached_gpu_nodes)) = dag_cache.get(&root_hash) {
        bit_writer.bits.extend(cached_bits);

        if let Some(&_gpu_offset) = seen_hashes.get(&root_hash) {
            // Already seen in GPU writer — reuse existing offset
            return;
        } else {
            // First time seeing this hash for GPU output, so insert the cached GPU nodes
            let offset = gpu_writer.nodes.len();
            gpu_writer.nodes.extend_from_slice(cached_gpu_nodes);
            seen_hashes.insert(root_hash, offset);
            return;
        }
    }

    let bit_start = bit_writer.bits.len();
    let gpu_start = gpu_writer.nodes.len();

    // Actual serialization logic
    serialize_nodes(root, root_hash, bit_writer, gpu_writer, seen_hashes);

    // Cache results
    let bits = bit_writer.bits[bit_start..].to_vec();
    let gpu_nodes = gpu_writer.nodes[gpu_start..].to_vec();
    // Do NOT slice GPU output, since we’re reusing offsets instead
    dag_cache.insert(root_hash, (bits, gpu_nodes)); // GPU nodes not stored
    seen_hashes.insert(root_hash, gpu_start); // Points to real data location
}
