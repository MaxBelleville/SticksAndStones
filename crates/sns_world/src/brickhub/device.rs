
use super::{BrickIndex, CellID, brick,MaterialID};
use bevy_math::*;
use sns_core::prelude::*;

#[derive(Clone, Copy, Default)]
pub struct Buffers {
    pub bricks: GpuBuffer<brick::Brick>,
    pub materials: GpuBuffer<u16>,
    pub indices: GpuBuffer<u32>,
}

impl Buffers {
    pub fn ensure_capacity(
        &mut self,
        gpu: &dyn GpuDevice,
        brick_cap: usize,
        material_cap: usize,
        index_cap: usize,
    ) {
        if self.bricks.capacity() < brick_cap {
            self.bricks.grow(gpu, brick_cap.next_power_of_two(), true);
        }

        if self.materials.capacity() < material_cap {
            self.materials
                .grow(gpu, material_cap.next_power_of_two(), true);
        }

        if self.indices.capacity() < index_cap {
            self.indices.grow(gpu, index_cap.next_power_of_two(), true);
        }
    }
    pub fn get_brick_mut(&mut self) -> &mut GpuBuffer<brick::Brick> {
        &mut self.bricks
    }
    pub fn get_material_mut(&mut self) -> &mut GpuBuffer<u16> {
        &mut self.materials
    }
    pub fn get_index_mut(&mut self) -> &mut GpuBuffer<u32> {
        &mut self.indices
    }

}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub enum DeltaState {
    #[default]
    Inactive,
    Active,
    Invalid,
}
#[derive(Clone, Copy)]
pub struct CellRange {
    cell_id: CellID,
    range: Range<usize>,
}
impl CellRange {
    pub fn get_id(&self) -> CellID {
        self.cell_id
    }
    pub fn get_range(&self) -> &Range<usize>{
        &self.range
    }
}

#[derive(Default)]
pub struct DeltaBatch {
    brick_ranges: Vec<CellRange>,
    material_ranges: Vec<CellRange>,
    index_ranges: Vec<CellRange>,
}
impl DeltaBatch {
    pub fn take_and_push(&mut self, id: CellID, delta: &mut super::allocators::CellDelta) {
        if let Some((from, to)) = delta.take(StorageType::Brick) {
            self.register_range(id, StorageType::Brick, from, to);
        }
        if let Some((from, to)) = delta.take(StorageType::Indices) {
            self.register_range(id, StorageType::Indices, from, to);
        }
        if let Some((from, to)) = delta.take(StorageType::Material) {
            self.register_range(id, StorageType::Material, from, to);
        }
    }
    pub fn register_range(
        &mut self,
        id: CellID,
        storage_type: StorageType,
        from: usize,
        to: usize,
    ) {
        match storage_type {
            StorageType::Brick => {
                self.brick_ranges.push(CellRange {
                    cell_id: id,
                    range: Range::new(from, to),
                });
            }
            StorageType::Material => {
                self.material_ranges.push(CellRange {
                    cell_id: id,
                    range: Range::new(from, to),
                });
            }
            StorageType::Indices => {
                self.index_ranges.push(CellRange {
                    cell_id: id,
                    range: Range::new(from, to),
                });
            }
        }
    }
    pub fn get_ranges(&self, storage_type: StorageType) -> &Vec<CellRange> {
        match storage_type {
            StorageType::Brick => &self.brick_ranges,
            StorageType::Material => &self.material_ranges,
            StorageType::Indices => &self.index_ranges,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub enum StorageType {
    Brick,
    Material,
    Indices,
}
impl StorageType {
    pub fn element_size(&self) -> usize {
        match self {
            StorageType::Brick => std::mem::size_of::<brick::Brick>(),
            StorageType::Material => std::mem::size_of::<MaterialID>(),
            StorageType::Indices => std::mem::size_of::<u32>(),
        }
    }
}