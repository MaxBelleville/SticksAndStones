use bevy_math::*;
use smallvec::SmallVec;
use sns_core::prelude::{LinearAllocator, Range};

use super::{
    BRICK_SIZE, BRICKMAP_SIZE, BRICKS_PER_BRICKMAP, BrickID, MaterialID, VOXELS_PER_BRICK, brick,
    index, material,
};

pub struct BrickMapStorage {
    bricks: LinearAllocator<brick::Brick>,
    materials: material::MaterialAllocator,
}

impl Default for BrickMapStorage {
    fn default() -> Self {
        let brick_capacity = BRICKS_PER_BRICKMAP;
        let bricks = LinearAllocator::new(brick_capacity);
        let materials = material::MaterialAllocator::new(brick_capacity * VOXELS_PER_BRICK);

        Self { bricks, materials }
    }
}

impl BrickMapStorage {
    pub fn allocate_material<K: index::IndexKind>(
        &mut self,
        brick_index: usize,
        events: &mut SmallVec<[BrickEditEvent;4]>,
    ) {
        let brick = self.bricks.get_mut(brick_index);
        if brick.get_raw_index().is_unset() {
            let base = self.materials.alloc_next_block();
            brick.set_start_index::<K>(base);
            events.push(BrickEditEvent::MaterialBlockAllocated { base });
        }
    }

    pub fn add_brick_occupancy(&mut self, brick_index: usize, voxel: UVec3, voxel_index: usize) {
        let brick = self.bricks.get_mut(brick_index);
        let word = voxel_index >> 6;
        let bit = voxel_index & 63;
        brick.set_occupancy(word, 1u64 << bit);
        brick.set_lod(voxel);
    }


    pub fn clear_brick_occupancy(&mut self, brick_index: usize, voxel_index: usize, voxel: UVec3) -> Option<bool> {
        let brick = self.bricks.get_mut(brick_index);
        let word = voxel_index >> 6;
        let bit = voxel_index & 63;
        if !brick.test_occupancy(word, bit) {
            return None; // already empty
        }
        let is_only_voxel = brick.occupied_voxel_count() == 1;
        brick.clear_occupancy(word, bit);
        brick.clear_lod(voxel);
        Some( is_only_voxel )
    }
    pub fn get_voxel(&self, brick_index: usize, voxel_index: usize) -> usize {
        let brick = self.bricks.get(brick_index);
        brick.get_raw_index().value() as usize + voxel_index
    }
}

pub struct BrickIndexGrid {
    index: index::IndexGrid,
    status: brick::BrickStatusGrid,
}

impl Default for BrickIndexGrid {
    fn default() -> Self {
        let brick_capacity = BRICKS_PER_BRICKMAP;
        let index = index::IndexGrid::default();
        let status = brick::BrickStatusGrid::new(brick_capacity);
        Self { index, status }
    }
}

#[derive(Default)]
pub struct BrickMap {
    storage: BrickMapStorage,
    grid: BrickIndexGrid,
    active_bricks: u32,
}

impl BrickMap {
    pub fn get_brick(&self, range: Range<usize>)-> &[brick::Brick]{
        self.storage.bricks.slice_range(range.start(), range.end())
    }
    pub fn get_material(&self, range: Range<usize>)-> &[u16] {
        self.storage.materials.slice_bucket(range.start(), range.end())
    }
    pub fn get_indices(&self, range: Range<usize>)-> &[u32] {
        self.grid.index.slice_range(range.start(), range.end())
    }


    pub fn insert(&mut self, voxel: BrickID, material: MaterialID) -> SmallVec<[BrickEditEvent; 4]> {
        let mut events = SmallVec::new();
        let flipped_voxel = self.flip_y(voxel);
        let brick_coord = flipped_voxel / BRICK_SIZE as u32;

        let (brick_index, _was_new) = self.get_or_alloc_brick(brick_coord, &mut events);

        let brick = self.storage.bricks.get_mut(brick_index);
        if brick.is_empty() {
            self.active_bricks += 1;
        }
    
        self.storage
            .allocate_material::<index::VoxelKind>(brick_index, &mut events);
        
        let voxel_index = super::voxel_in_brick(flipped_voxel) as usize;

        let voxel_offset = self.storage.get_voxel(brick_index, voxel_index);
        

        let material_id: u16 = self.storage.materials.write_shared_mat(material);
        self.storage
            .materials
            .write_bucket_index(voxel_offset, material_id);

        events.push(BrickEditEvent::VoxelMaterialChanged {
            index: voxel_offset,
        });

        self.storage.add_brick_occupancy(brick_index, voxel, voxel_index);

        events
    }

    pub fn remove(&mut self, voxel: BrickID) -> SmallVec<[BrickEditEvent;4]> {
        let mut events = SmallVec::new();
        let flipped_voxel = self.flip_y(voxel);

        let brick_coord = flipped_voxel / BRICK_SIZE as u32;
        let brick_index = self.grid.index.get(brick_coord).unwrap() as usize;

        let brick = self.storage.bricks.get_mut(brick_index);
        let voxel_index = super::voxel_in_brick(flipped_voxel) as usize;
       
        let material_index = brick.get_raw_index().value() as usize + voxel_index;
 
        let Some(was_only_voxel) = self.storage.clear_brick_occupancy(brick_index,voxel_index,voxel) else {
            return events;        
        };
        self.storage.materials.remove_bucket_index(material_index);
        events.push(BrickEditEvent::VoxelMaterialChanged {
            index: material_index,
        });

        if was_only_voxel {
            self.grid.index.clear(brick_coord);
            self.grid.status.set_inactive(brick_index);
            events.push(BrickEditEvent::BrickBecameEmpty { brick_index });
            self.active_bricks -= 1;
        }
    
        events
    }

    pub fn get_or_alloc_brick(
        &mut self,
        brick_coord: BrickID,
        events: &mut SmallVec<[BrickEditEvent;4]>,
    ) -> (usize, bool) {
        match self.grid.index.get(brick_coord) {
            Some(idx) => (idx as usize, false),
            None => {
                let idx = self.storage.bricks.allocate(brick::Brick::default());
                events.push(BrickEditEvent::BrickAllocated { brick_index: idx });
                self.grid.index.set(brick_coord, idx as u32);
                self.grid.status.set_active(idx);
                (idx, true)
            }
        }
    }

    pub fn clear(&mut self) {
        self.storage = BrickMapStorage::default();
        self.grid = BrickIndexGrid::default();
    }

    fn flip_y(&self, voxel: BrickID) -> BrickID {
        let flipped_y = (BRICKMAP_SIZE * BRICK_SIZE) as u32 - 1 - voxel.y;
        UVec3::new(voxel.x, flipped_y, voxel.z)
    }
}

pub enum BrickEditEvent {
    BrickAllocated { brick_index: usize },
    BrickBecameEmpty { brick_index: usize },
    MaterialBlockAllocated { base: u32 },
    VoxelMaterialChanged { index: usize },
}
