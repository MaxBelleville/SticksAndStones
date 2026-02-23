
use std::marker::PhantomData;

use super::{brick,CellID,BrickIndex};
use bevy_math::*;
use sns_core::prelude::*;

#[derive(Clone, Copy, Default)]
pub struct DeviceBuffers {
    pub bricks: GpuBuffer<brick::Brick>,
    pub materials: GpuBuffer<u16>,
    pub indices: GpuBuffer<u32>,
}
impl DeviceBuffers {
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
            self.materials.grow(gpu, material_cap.next_power_of_two(), true);
        }

        if self.indices.capacity() < index_cap {
            self.indices.grow(gpu, index_cap.next_power_of_two(), true);
        }
    }
}

#[derive(Clone, Copy, Default,PartialEq, Eq)]
pub struct BrickStream {
    index: BrickIndex,
}

#[derive(Clone, Copy, Default,PartialEq, Eq)]
pub struct MaterialStream {
    base: u32, 
    count: u32,
}

#[derive(Clone, Copy, Default,PartialEq, Eq)]
pub struct IndicesStream {
    index: u32,
}

pub trait CellStream: Default + Clone + Copy {
    const STREAM_TYPE: char;
    // We use Default for a basic 'new', or define a custom one
    fn new() -> Self;
    
    fn get_index(&self) -> u32;
    fn set_index(&mut self, val: u32);

    fn get_extra(&self) -> u32 { 0 }
    fn set_extra(&mut self, _val: u32) {}
}
impl CellStream for BrickStream {
    const STREAM_TYPE: char = 'B';
    fn new() -> Self {
        Self::default()
    }
    fn get_index(&self) -> u32 {
        self.index
    }
    fn set_index(&mut self, val: u32) {
        self.index = val;
    }
}
impl CellStream for MaterialStream {
    const STREAM_TYPE: char = 'M';
    fn new() -> Self {
        Self::default()
    }
    fn get_index(&self) -> u32 {
        self.base
    }
    fn set_index(&mut self, val: u32) {
        self.base = val;
    }
    fn get_extra(&self) -> u32 {
        self.count
    }
    fn set_extra(&mut self, val: u32) {
        self.count = val;
    }
}
impl CellStream for IndicesStream {
    const STREAM_TYPE: char = 'I';
    fn new() -> Self {
        Self::default()
    }
    fn get_index(&self) -> u32 {
        self.index
    }
    fn set_index(&mut self, val: u32) {
        self.index = val;
    }
}


#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub enum DeltaState {
    #[default]
    Inactive,
    Invalid,
    Active,
}
#[derive(Clone, Copy)]
pub struct CellDelta<S: CellStream>{
   range: Range<usize>,
   state: DeltaState,
   stream: PhantomData<S>,
}

impl<S: CellStream> Default for CellDelta<S> {
    fn default() -> Self {
        Self {
            range: Range::new(usize::MAX, 0),
            state: DeltaState::Inactive,
            stream: PhantomData::default(),
        }
    }
}
impl<S: CellStream> CellDelta<S> {
    pub fn reset(&mut self) {
        self.state = DeltaState::Inactive;
        self.range.set_range(usize::MAX, 0);
    }

    pub fn register(&mut self, index: usize) {
        self.state = DeltaState::Active;
        self.range.set_start(self.range.start().min(index));
        self.range.set_end(self.range.end().max(index + 1));
    }

    pub fn register_range(&mut self, from: usize, to: usize) {
        self.state = DeltaState::Active;
        self.range.set_start(self.range.start().min(from));
        self.range.set_end(self.range.end().max(to));
    }
    pub fn is_active(&self) -> bool {
        self.state == DeltaState::Active
    }
    pub fn get_active_range(&self) -> Option<(usize, usize)> {
        if self.state == DeltaState::Active {
            Some(self.range.get_range())
        } else {
            None
        }
    }
}
#[derive(Clone, Copy, Default)]
pub struct CellDeltaGroup {
    cell_id: CellID,
    brick_delta: CellDelta<BrickStream>,
    material_delta: CellDelta<MaterialStream>,
    index_delta: CellDelta<IndicesStream>,
}

impl CellDeltaGroup {
    pub fn new(cell_id: CellID) -> Self {
        Self {
            cell_id,
            brick_delta: CellDelta::default(),
            material_delta: CellDelta::default(),
            index_delta: CellDelta::default(),
        }
    }

    pub fn get_brick_delta(&self) -> &CellDelta<BrickStream> {
        &self.brick_delta
    }
    pub fn get_brick_delta_mut(&mut self) -> &mut CellDelta<BrickStream> {
        &mut self.brick_delta
    }
    pub fn get_material_delta(&self) -> &CellDelta<MaterialStream> {
        &self.material_delta
    }
    pub fn get_material_delta_mut(&mut self) -> &mut CellDelta<MaterialStream> {
        &mut self.material_delta
    }
    pub fn get_index_delta(&self) -> &CellDelta<IndicesStream> {
        &self.index_delta
    }
    pub fn get_index_delta_mut(&mut self) -> &mut CellDelta<IndicesStream> {
        &mut self.index_delta
    }

    pub fn get_active_range<S: CellStream>(&self) -> Option<(usize, usize)> {
        match S::STREAM_TYPE {
            'B' => self.brick_delta.get_active_range(),
            'M' => self.material_delta.get_active_range(),
            'I' => self.index_delta.get_active_range(),
            _ => panic!("Invalid stream type"),
        }
    }

    pub fn is_active<S: CellStream>(&self) -> bool {
        match S::STREAM_TYPE {
            'B' => self.brick_delta.is_active(),
            'M' => self.material_delta.is_active(),
            'I' => self.index_delta.is_active(),
            _ => panic!("Invalid stream type"),
        }
    }

    pub fn register<S: CellStream>(&mut self, index: usize) {
        match S::STREAM_TYPE {
            'B' => self.brick_delta.register(index),
            'M' => self.material_delta.register(index),
            'I' => self.index_delta.register(index),
            _ => panic!("Invalid stream type"),
        }
    }

    pub fn register_range<S: CellStream>(&mut self, from: usize, to: usize) {
        match S::STREAM_TYPE {
            'B' => self.brick_delta.register_range(from,to),
            'M' => self.material_delta.register_range(from,to),
            'I' => self.index_delta.register_range(from,to),
            _ => panic!("Invalid stream type"),
        }
    }


    pub fn get_stream_type<S: CellStream>(&self) -> char {
        S::STREAM_TYPE
    }

    pub fn get_cell_id(&self) -> CellID {
        self.cell_id
    }
    pub fn reset(&mut self) {
        self.brick_delta.reset();
        self.material_delta.reset();
        self.index_delta.reset();
    }

}