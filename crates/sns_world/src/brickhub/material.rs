use std::{collections::HashMap, sync::atomic::{AtomicU32, Ordering}};

use crate::prelude::{MaterialID, MaterialIndex};

use super::VOXELS_PER_BRICK;

#[derive(Default)]
pub struct MaterialAllocator {
    material_bucket: Vec<MaterialID>,
    materials: Vec<MaterialID>,        // flat array
    /// Lookup table for deduplication (CPU-only)
    material_lookup: HashMap<MaterialID, MaterialIndex>,
    next_voxel_index: AtomicU32,      // offset cursor
}

impl MaterialAllocator {
    pub fn new(capacity: usize) -> Self {
        Self {
            material_bucket: vec![0; capacity],
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
            (start as usize) + VOXELS_PER_BRICK <= self.material_bucket.len()
        );
        start
    }
    pub fn write_shared_mat(&mut self, material: MaterialID) -> MaterialIndex {
        if let Some(&idx) = self.material_lookup.get(&material) {
            return idx;
        }

        let idx = self.materials.len() as MaterialIndex;
        assert!(idx < u16::MAX);

        self.materials.push(material);
        self.material_lookup.insert(material, idx);
        idx
    }

    pub fn write_bucket_index(&mut self, index: usize, material: MaterialID) { 
        self.material_bucket[index] = material;
    }
 
    pub fn remove_bucket_index(&mut self, index: usize) {
        self.material_bucket[index] = 0;
    }

    pub fn slice_bucket(&self, base: usize, count: usize) -> &[MaterialID] {
        &self.material_bucket[base..base + count]
    }




}