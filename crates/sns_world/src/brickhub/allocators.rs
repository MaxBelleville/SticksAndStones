
use bevy_math::*;
use sns_core::prelude::Range;



use super::{BRICKS_PER_BRICKMAP,VOXELS_PER_BRICK,CellID,device};
use std::collections::HashMap;

#[derive(Clone, Copy,Default)]
pub struct QueuedRange{
   cell_id: CellID,
   range: Range<usize>,
   stream_type: char,
}
#[derive(Default)]
pub struct DeviceQueue {
    ranges: Vec<QueuedRange>,
}
impl DeviceQueue {
    pub fn aggregate_cell(&mut self, group: &device::CellDeltaGroup) {
        self.aggregate_stream::<device::BrickStream>(group);
        self.aggregate_stream::<device::MaterialStream>(group);
        self.aggregate_stream::<device::IndicesStream>(group);
    }

    pub fn aggregate_stream<S: device::CellStream>(&mut self, group: &device::CellDeltaGroup) {
        let cell_id = group.get_cell_id();
        if let Some((from, to)) = group.get_active_range::<S>() {
            self.ranges.push(QueuedRange {
                cell_id,
                stream_type: S::STREAM_TYPE,
                range: Range::new(from,to)
            });
        }
    }

    pub fn range_from_stream<S: device::CellStream>(&self) -> Vec<&QueuedRange> {
       self.ranges.iter().filter(|r| r.stream_type == S::STREAM_TYPE).collect()
    }

    pub fn clear(&mut self) {
        self.ranges.clear();
    }
    pub fn commit_cell(group: &mut device::CellDeltaGroup) {
        group.reset();
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
    ring: u8, // future
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
        if let Some((idx, range)) = self.free
            .iter()
            .enumerate()
            .find(|(_, r)| r.count >= count)
        {
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
}

pub struct CompactionCandidate {
    coord: IVec3, // cell coord
    old_alloc: CellAllocation,
    new_alloc: Option<CellAllocation>, // will be filled in during compaction
}
impl CompactionCandidate {
    pub fn get_coord(&self) -> IVec3 {
        self.coord
    }
    pub fn get_old_alloc(&self) -> &CellAllocation {
        &self.old_alloc
    }
    pub fn get_new_alloc(&self) -> Option<&CellAllocation> {
        self.new_alloc.as_ref()
    }
    pub fn from_alloc(alloc: &CellAllocation, coord: IVec3) -> Self {
        Self {
            coord,
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

    allocations: HashMap<IVec3, CellAllocation>,
}


impl DeviceAllocator {
     pub fn admit_cell(
        &mut self,
        coord: IVec3,
        ring: u8,
    ) -> &CellAllocation {
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

    pub fn allocate_new_ranges(
        &mut self,
        candidates: &mut [CompactionCandidate],
    ) {
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

                self.allocations.insert(cand.coord, new_alloc.clone());
            }
        }
    }
    pub fn is_fragmented(&self, ring: u8) -> bool {
        // Fragmented if we have free ranges but ring is still full
        self.bricks.has_free_space()
            || self.materials.has_free_space()
            || self.indices.has_free_space()
    }

    pub fn select_compaction_candidates(&self, ring: u8,coord: IVec3) -> Vec<CompactionCandidate> {
        self.allocations
            .values()
            .filter(|a| a.get_ring() == ring)
            .map(|alloc| CompactionCandidate::from_alloc(alloc,coord))
            .take(2) // start small
            .collect()
    }

    /// Remove a cell allocation and return it
    pub fn remove(&mut self, coord: &IVec3) -> Option<CellAllocation> {
        self.allocations.remove(coord)
    }

    /// Free ranges for a removed cell back to RangeAllocator
    pub fn free_cell_allocation(&mut self, alloc: CellAllocation) {
        self.bricks.free(alloc.brick_base, alloc.brick_count);
        self.materials.free(alloc.material_base, alloc.material_count);
        self.indices.free(alloc.index_base, alloc.index_count);
    }

    /// Lookup a cell allocation
    pub fn get(&self, coord: IVec3) -> Option<&CellAllocation> {
        self.allocations.get(&coord)
    }

    pub fn values(&self) -> impl Iterator<Item = &CellAllocation> {
        self.allocations.values()
    }

    pub fn contains_key(&self, coord: &IVec3) -> bool {
        self.allocations.contains_key(coord)
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