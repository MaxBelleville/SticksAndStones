use bevy_math::*;
use smallvec::SmallVec;
use sns_core::prelude::Range;

use super::{BRICKS_PER_BRICKMAP, CellID, VOXELS_PER_BRICK, clipmap, device};
use core::alloc;
use std::collections::HashMap;

#[derive(Default)]
pub struct CellDeltaRange {
    range: Range<usize>,
    state: device::DeltaState,
}

impl CellDeltaRange {
    pub fn register(&mut self, index: usize) {
        self.state = device::DeltaState::Active;
        self.range.set_start(self.range.start().min(index));
        self.range.set_end(self.range.end().max(index + 1));
    }

    pub fn register_range(&mut self, from: usize, to: usize) {
        self.state = device::DeltaState::Active;
        self.range.set_start(self.range.start().min(from));
        self.range.set_end(self.range.end().max(to));
    }

    pub fn take(&mut self) -> Option<(usize, usize)> {
        if self.state == device::DeltaState::Active {
            let r = (self.range.start(), self.range.end());
            *self = Self::default();
            Some(r)
        } else {
            //TODO: Potentially do something if invalid?
            None
        }
    }
    pub fn is_active(&self) -> bool {
        self.state == device::DeltaState::Active
    }
    pub fn is_state(&self, state: device::DeltaState) -> bool {
        self.state == state
    }
}

pub struct CellDelta {
    pub dirty_brick: CellDeltaRange,
    pub dirty_material: CellDeltaRange,
    pub dirty_indices: CellDeltaRange,
}
impl Default for CellDelta {
    fn default() -> Self {
        Self {
            dirty_brick: CellDeltaRange::default(),
            dirty_material: CellDeltaRange::default(),
            dirty_indices: CellDeltaRange::default(),
        }
    }
}

impl CellDelta {
    pub fn register(&mut self, storage_type: device::StorageType, index: usize) {
        match storage_type {
            device::StorageType::Brick => self.dirty_brick.register(index),
            device::StorageType::Material => self.dirty_material.register(index),
            device::StorageType::Indices => self.dirty_indices.register(index),
        }
    }
    pub fn register_range(&mut self, storage_type: device::StorageType, to: usize, from: usize) {
        match storage_type {
            device::StorageType::Brick => self.dirty_brick.register_range(to, from),
            device::StorageType::Material => self.dirty_material.register_range(to, from),
            device::StorageType::Indices => self.dirty_indices.register_range(to, from),
        }
    }

    pub fn take(&mut self, storage_type: device::StorageType) -> Option<(usize, usize)> {
        match storage_type {
            device::StorageType::Brick => self.dirty_brick.take(),
            device::StorageType::Material => self.dirty_material.take(),
            device::StorageType::Indices => self.dirty_indices.take(),
        }
    }
    pub fn is_active(&self, storage_type: device::StorageType) -> bool {
        match storage_type {
            device::StorageType::Brick => self.dirty_brick.is_active(),
            device::StorageType::Material => self.dirty_material.is_active(),
            device::StorageType::Indices => self.dirty_indices.is_active(),
        }
    }

    pub fn clear(&mut self) {
        self.dirty_brick = CellDeltaRange::default();
        self.dirty_material = CellDeltaRange::default();
        self.dirty_indices = CellDeltaRange::default();
    }
}

#[derive(Clone, Copy)]
pub struct CellAllocation {
    brick_base: usize,
    brick_count: usize,

    material_base: usize,
    material_count: usize,

    index_base: usize,
    index_count: usize,
    ring: u8,
}
impl CellAllocation {
    pub fn get_brick_base(&self) -> usize {
        self.brick_base
    }

    pub fn get_material_base(&self) -> usize {
        self.material_base
    }

    pub fn get_index_base(&self) -> usize {
        self.index_base
    }
    pub fn get_ring(&self) -> u8 {
        self.ring
    }
}

struct FreeRange {
    base: usize,
    count: usize,
}

#[derive(Default)]
pub struct RangeAllocator {
    next: usize,
    free: Vec<FreeRange>,
}

impl RangeAllocator {
    pub fn alloc(&mut self, count: usize) -> usize {
        if let Some((idx, range)) = self.free.iter().enumerate().find(|(_, r)| r.count >= count) {
            let base = range.base;

            if range.count == count {
                self.free.swap_remove(idx);
            } else {
                self.free[idx].base += count;
                self.free[idx].count -= count;
            }

            return base;
        }

        let base = self.next;
        self.next += count;
        base
    }
    pub fn has_free_space(&self) -> bool {
        !self.free.is_empty()
    }

    pub fn free(&mut self, base: usize, count: usize) {
        self.free.push(FreeRange { base, count });
        self.free.sort_by_key(|r| r.base);

        // Merge contiguous ranges
        let mut merged: Vec<FreeRange> = Vec::with_capacity(self.free.len());
        for r in self.free.drain(..) {
            if let Some(last) = merged.last_mut() {
                if last.base + last.count == r.base {
                    last.count += r.count;
                    continue;
                }
            }
            merged.push(r);
        }
        self.free = merged;
    }

    pub fn capacity(&self) -> usize {
        self.next
    }

    pub fn total_free(&self) -> usize {
        self.free.iter().map(|r| r.count).sum()
    }

    pub fn largest_free_block(&self) -> usize {
        self.free.iter().map(|r| r.count).max().unwrap_or(0)
    }

    pub fn is_fragmented_for(&self, size: usize) -> bool {
        self.total_free() >= size && self.largest_free_block() < size
    }
}

pub struct CompactionCandidate {
    id: CellID, // cell id
    old_alloc: CellAllocation,
    new_alloc: Option<CellAllocation>, // will be filled in during compaction
}
impl CompactionCandidate {
    pub fn get_id(&self) -> IVec3 {
        self.id
    }
    pub fn get_old_alloc(&self) -> &CellAllocation {
        &self.old_alloc
    }
    pub fn get_new_alloc(&self) -> Option<&CellAllocation> {
        self.new_alloc.as_ref()
    }
    pub fn from_alloc(alloc: &CellAllocation, id: IVec3) -> Self {
        Self {
            id,
            old_alloc: alloc.clone(),
            new_alloc: None,
        }
    }
}

#[derive(Default)]
pub struct DeviceAllocator {
    bricks: RangeAllocator,
    materials: RangeAllocator,
    indices: RangeAllocator,
    allocations: HashMap<CellID, CellAllocation>,
}

impl DeviceAllocator {
    pub fn admit_cell(&mut self, coord: IVec3, ring: u8) -> &CellAllocation {
        self.allocations.entry(coord).or_insert_with(|| {
            let brick_count = BRICKS_PER_BRICKMAP;
            let material_count = BRICKS_PER_BRICKMAP * VOXELS_PER_BRICK;
            let index_count = BRICKS_PER_BRICKMAP;

            // Allocate from free ranges or grow
            let brick_base = self.bricks.alloc(brick_count);
            let material_base = self.materials.alloc(material_count);
            let index_base = self.indices.alloc(index_count);
            CellAllocation {
                brick_base,
                brick_count,
                material_base,
                material_count,
                index_base,
                index_count,
                ring,
            }
        })
    }

    pub fn allocate_new_ranges(&mut self, candidates: &mut [CompactionCandidate]) {
        for candidate in candidates.iter_mut() {
            let old = &candidate.old_alloc;
            let brick_base = self.bricks.alloc(old.brick_count);
            let material_base = self.materials.alloc(old.material_count);
            let index_base = self.indices.alloc(old.index_count);

            candidate.new_alloc = Some(CellAllocation {
                brick_base,
                brick_count: old.brick_count,
                material_base,
                material_count: old.material_count,
                index_base,
                index_count: old.index_count,
                ring: old.ring,
            });
        }
    }

    pub fn finalize_compaction(&mut self, candidates: &[CompactionCandidate]) {
        for cand in candidates {
            if let Some(new_alloc) = &cand.new_alloc {
                let old = &cand.old_alloc;
                self.bricks.free(old.brick_base, old.brick_count);
                self.materials.free(old.material_base, old.material_count);
                self.indices.free(old.index_base, old.index_count);

                self.allocations.insert(cand.id, new_alloc.clone());
            }
        }
    }
    pub fn is_fragmented_for(&self, size: usize) -> bool {
        // Fragmented if we have free ranges but ring is still full
        self.bricks.is_fragmented_for(size)
            || self.materials.is_fragmented_for(size)
            || self.indices.is_fragmented_for(size)
    }

    pub fn select_compaction_candidates(&self, ring: u8) -> Vec<CompactionCandidate> {
        self.allocations
            .iter()
            .filter(|(_, a)| a.get_ring() == ring)
            .map(|(id, alloc)| CompactionCandidate::from_alloc(alloc, id.clone()))
            .take(2) // start small
            .collect()
    }

    /// Remove a cell allocation and return it
    pub fn remove(&mut self, id: CellID) -> Option<CellAllocation> {
        if let Some(alloc) = self.allocations.remove(&id) {
            return Some(alloc);
        }
        None
    }

    /// Free ranges for a removed cell back to RangeAllocator
    pub fn free_cell_allocation(&mut self, alloc: CellAllocation) {
        self.bricks.free(alloc.brick_base, alloc.brick_count);
        self.materials
            .free(alloc.material_base, alloc.material_count);
        self.indices.free(alloc.index_base, alloc.index_count);
    }

    /// Lookup a cell allocation
    pub fn get(&self, id: CellID) -> Option<&CellAllocation> {
        self.allocations.get(&id)
    }

    pub fn values(&self) -> impl Iterator<Item = &CellAllocation> {
        self.allocations.values()
    }

    pub fn contains_key(&self, id: CellID) -> bool {
        self.allocations.contains_key(&id)
    }

    pub fn brick_capacity(&self) -> usize {
        self.bricks.capacity()
    }

    pub fn material_capacity(&self) -> usize {
        self.materials.capacity()
    }

    pub fn index_capacity(&self) -> usize {
        self.indices.capacity()
    }
}
