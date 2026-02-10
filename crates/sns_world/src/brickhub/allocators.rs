
use bevy_math::*;
use super::{device,BRICKS_PER_CELL,VOXELS_PER_BRICK,BrickHubCell};
use std::collections::{HashMap, HashSet};

pub struct BrickUpload {
    cell_coord: IVec3,
    brick_index: u32,
}
impl BrickUpload {
    pub fn get_cell_coord(&self) -> IVec3 {
        self.cell_coord
    }
    pub fn get_brick_index(&self) -> u32 {
        self.brick_index
    }
}

pub struct MaterialUpload {
    cell_coord: IVec3,
    base: u32,
    count: u32,
}
impl MaterialUpload {
    pub fn get_cell_coord(&self) -> IVec3 {
        self.cell_coord
    }
    pub fn get_base(&self) -> u32 {
        self.base
    }
    pub fn get_count(&self) -> u32 {
        self.count
    }
}

pub struct IndexUpload {
    cell_coord: IVec3,
    index: u32,
}
impl IndexUpload {
    pub fn get_cell_coord(&self) -> IVec3 {
        self.cell_coord
    }
    pub fn get_index(&self) -> u32 {
        self.index
    }
}

#[derive(Default)]
pub struct HubDeviceQueue {
    brick_updates: Vec<BrickUpload>,
    material_updates: Vec<MaterialUpload>,
    index_updates: Vec<IndexUpload>,
}

impl HubDeviceQueue {
    pub fn clear(&mut self) {
        self.brick_updates.clear();
        self.material_updates.clear();
        self.index_updates.clear();
    }
    pub fn get_brick_updates(&self) -> &[BrickUpload] {
        &self.brick_updates
    }
    pub fn get_material_updates(&self) -> &[MaterialUpload] {
        &self.material_updates
    }

    pub fn get_index_updates(&self) -> &[IndexUpload] {
        &self.index_updates
    }
    
     pub fn iter_cells(&self) -> impl Iterator<Item = IVec3> + '_ {
        self.brick_updates.iter().map(|u| u.cell_coord)
            .chain(self.material_updates.iter().map(|u| u.cell_coord))
            .chain(self.index_updates.iter().map(|u| u.cell_coord))
            .collect::<HashSet<_>>()
            .into_iter()
    }

    pub fn aggregate_cell(&mut self, coord: IVec3, cell: &mut device::CellDeviceState) {
        if let Some((from, to)) = self.aggregate_range(cell.get_brick_delta()) {
            for brick_index in from..to {
                self.brick_updates.push(BrickUpload {
                    cell_coord: coord,
                    brick_index: brick_index as u32,
                });
            }
        }
        if let Some((from, to)) = self.aggregate_range(cell.get_material_delta()) {
            self.material_updates.push(MaterialUpload {
                cell_coord: coord,
                base: from as u32,
                count: (to - from) as u32,
            });
        }

        if let Some((from, to)) = self.aggregate_range(cell.get_index_delta()) {
            for index in from..to {

                self.index_updates.push(IndexUpload {
                    cell_coord: coord,
                    index: index as u32,
                });
            }
        }

        cell.get_brick_delta_mut().reset();
        cell.get_material_delta_mut().reset();
        cell.get_index_delta_mut().reset();
    }

    pub fn aggregate_range(&self, delta: &device::DeviceDataDelta) -> Option<(usize, usize)> {
        if !delta.is_active() {
            return None;
        }
        Some(delta.get_range())
    }
    pub fn remove_cell_updates(&mut self, coord: IVec3) {
        self.brick_updates.retain(|u| u.cell_coord != coord);
        self.material_updates.retain(|u| u.cell_coord != coord);
        self.index_updates.retain(|u| u.cell_coord != coord);
    }

}
#[derive(Clone, Copy)]
pub struct CellGpuAllocation {
    brick_base: usize,
    brick_count: usize,

    material_base: usize,
    material_count: usize,

    index_base: usize,
    index_count: usize,
    ring: u8, // future
}
impl CellGpuAllocation {
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
    coord: IVec3,
    old_alloc: CellGpuAllocation,
    new_alloc: Option<CellGpuAllocation>, // will be filled in during compaction
}
impl CompactionCandidate {
    pub fn get_coord(&self) -> IVec3 {
        self.coord
    }
    pub fn get_old_alloc(&self) -> &CellGpuAllocation {
        &self.old_alloc
    }
    pub fn get_new_alloc(&self) -> Option<&CellGpuAllocation> {
        self.new_alloc.as_ref()
    }
    pub fn from_alloc(alloc: &CellGpuAllocation) -> Self {
        Self {
            coord: IVec3::new(alloc.brick_base as i32, alloc.material_base as i32, alloc.index_base as i32),
            old_alloc: alloc.clone(),
            new_alloc: None,
        }
    }
}

#[derive(Default)]
pub struct HubGpuAllocator {
    bricks: RangeAllocator,
    materials: RangeAllocator,
    indices: RangeAllocator,

    allocations: HashMap<IVec3, CellGpuAllocation>,
}


impl HubGpuAllocator {
     pub fn admit_cell(
        &mut self,
        coord: IVec3,
        ring: u8,
    ) -> &CellGpuAllocation {
        self.allocations.entry(coord).or_insert_with(|| {
            let brick_count = BRICKS_PER_CELL;
            let material_count = BRICKS_PER_CELL * VOXELS_PER_BRICK;
            let index_count = BRICKS_PER_CELL;

            // Allocate from free ranges or grow
            let brick_base = self.bricks.alloc(brick_count);
            let material_base = self.materials.alloc(material_count);
            let index_base = self.indices.alloc(index_count);

            CellGpuAllocation {
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

            candidate.new_alloc = Some(CellGpuAllocation {
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

    pub fn select_compaction_candidates(&self, ring: u8) -> Vec<CompactionCandidate> {
        self.allocations
            .values()
            .filter(|a| a.get_ring() == ring)
            .map(|alloc| CompactionCandidate::from_alloc(alloc))
            .take(2) // start small
            .collect()
    }

    /// Remove a cell allocation and return it
    pub fn remove(&mut self, coord: &IVec3) -> Option<CellGpuAllocation> {
        self.allocations.remove(coord)
    }

    /// Free ranges for a removed cell back to RangeAllocator
    pub fn free_cell_allocation(&mut self, alloc: CellGpuAllocation) {
        self.bricks.free(alloc.brick_base, alloc.brick_count);
        self.materials.free(alloc.material_base, alloc.material_count);
        self.indices.free(alloc.index_base, alloc.index_count);
    }

    /// Lookup a cell allocation
    pub fn get(&self, coord: IVec3) -> Option<&CellGpuAllocation> {
        self.allocations.get(&coord)
    }

    pub fn values(&self) -> impl Iterator<Item = &CellGpuAllocation> {
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