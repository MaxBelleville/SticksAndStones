use std::{collections::HashMap, sync::atomic::{AtomicU32, Ordering}};

use super::VOXELS_PER_BRICK;

#[derive(Default)]
pub struct MaterialAllocator {
    material_indices: Vec<u8>,
    materials: Vec<u16>,        // flat array
    /// Lookup table for deduplication (CPU-only)
    material_lookup: HashMap<u16, u8>,
    next_voxel_index: AtomicU32,      // offset cursor
}

impl MaterialAllocator {
    pub fn new(capacity: usize) -> Self {
        Self {
            material_indices: vec![0; capacity],
            materials: Vec::new(),
            material_lookup: HashMap::new(),
            next_voxel_index: AtomicU32::new(0),
        }
    }
    /// Allocates a 512-byte slice of the u8 voxel-material-index buffer
    pub fn alloc_next_block(&self) -> u32 {
        let start = self.next_voxel_index.fetch_add(
            VOXELS_PER_BRICK as u32,
            Ordering::Relaxed,
        );
        assert!(
            (start as usize) + VOXELS_PER_BRICK <= self.material_indices.len()
        );
        start
    }
    pub fn write_shared_mat(&mut self, material: u16) -> u8 {
        if let Some(&idx) = self.material_lookup.get(&material) {
            return idx;
        }

        let idx = self.materials.len() as u8;
        assert!(idx < u8::MAX);

        self.materials.push(material);
        self.material_lookup.insert(material, idx);
        idx
    }

    pub fn write_index(&mut self, index: usize, material_id: u8) { 
        self.material_indices[index] = material_id;
    }
 
    pub fn clear_id(&mut self, index: usize) {
        self.material_indices[index] = 0;
    }

}