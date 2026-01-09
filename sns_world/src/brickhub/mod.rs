use bevy_math::*;

use std::collections::HashMap;
use sns_core::prelude::LinearAllocator;

// --- Constants (Adjust these to match your headers) ---
pub const BRICK_SIZE: usize = 8;
pub const VOXELS_PER_BRICK: usize = BRICK_SIZE * BRICK_SIZE * BRICK_SIZE; // 512
pub const MATERIAL_BUCKET_SIZE: usize = VOXELS_PER_BRICK;

pub const BRICKHUB_CELL_SIZE: usize = 8; // bricks per axis
pub const BRICKS_PER_CELL: usize = BRICKHUB_CELL_SIZE * BRICKHUB_CELL_SIZE * BRICKHUB_CELL_SIZE;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Brick {
    /// 1 bit per voxel (512 bits)
    pub occupancy: [u64; 8], // 8 * 64 = 512 bits

    /// Index into the material bucket allocator
    /// Valid only if occupancy != 0
    pub material_bucket: u32,
}
impl Brick {
    pub fn is_empty(&self) -> bool {
        self.occupancy.iter().all(|&m| m == 0)
    }
    
}

#[repr(C)]
#[derive(Clone)]
pub struct MaterialBucket {
    pub materials: [u16; MATERIAL_BUCKET_SIZE],
}

impl Default for MaterialBucket {
    fn default() -> Self {
        Self {
            materials: [0; MATERIAL_BUCKET_SIZE],
        }
    }
}

#[repr(C)]
pub struct IndexGrid {
    indices: [u32; BRICKS_PER_CELL],
}

impl IndexGrid {
    pub fn new() -> Self {
        Self {
            indices: [0; BRICKS_PER_CELL],
        }
    }

    #[inline]
    pub fn index(coord: UVec3) -> usize {
        let cell_size = BRICKHUB_CELL_SIZE as u32;
        (coord.x + coord.y * cell_size  + coord.z * cell_size * cell_size) as usize
    }

    #[inline]
    pub fn get(&self, coord:UVec3) -> u32 {
        self.indices[Self::index(coord)]
    }

    #[inline]
    pub fn set(&mut self, coord:UVec3, value: u32) {
        let idx = Self::index(coord);
        self.indices[idx] = value;
    }

    pub fn raw(&self) -> &[u32] {
        &self.indices
    }
}
// --- Core Structures ---

pub struct BrickHubCell {
    /// Local brick storage
    pub brick_allocator: LinearAllocator<Brick>,

    /// Local material storage
    pub material_allocator: LinearAllocator<MaterialBucket>,

    /// Extended brickmap index grid
    pub index_grid: IndexGrid,

    /// Dirty flag for GPU upload
    pub dirty: bool,
}
impl BrickHubCell {
    pub fn new() -> Self {
        Self {
            brick_allocator: LinearAllocator::new(BRICKS_PER_CELL),
            material_allocator: LinearAllocator::new(BRICKS_PER_CELL),
            index_grid: IndexGrid::new(),
            dirty: true,
        }
    }

    pub fn clear(&mut self) {
        self.brick_allocator.clear();
        self.material_allocator.clear();
        self.index_grid = IndexGrid::new();
        self.dirty = true;
    }
}
pub struct BrickHub {
    pub cells: HashMap<IVec3, BrickHubCell>,
}

impl BrickHub {
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
        }
    }

    pub fn get_or_create_cell(&mut self, coord: IVec3) -> &mut BrickHubCell {
        self.cells
            .entry(coord)
            .or_insert_with(|| BrickHubCell::new())
    }

    pub fn unload_cell(&mut self, coord: IVec3) {
        self.cells.remove(&coord);
    }
}

// impl Supercell {
//     pub fn new() -> Self {
//         Self {
//             bricks: Vec::new(),
//             indices: vec![BRICK_UNLOADED_BIT; (SUPERGRID_CELL_SIZE.pow(3)) as usize],
//         }
//     }
// }
// pub struct Scene {
//     pub supergrid: Vec<Option<Box<Supercell>>>,
//     pub bricks_to_load: Vec<IVec3>,
//     pub brick_staging: Vec<Brick>,
//     pub indices_staging: Vec<u32>,
// }

// impl Scene {
//     pub fn new() -> Self {
//         Self {
//             supergrid: Vec::new(),
//             bricks_to_load: Vec::with_capacity(BRICK_LOAD_QUEUE_SIZE),
//             brick_staging: Vec::with_capacity(BRICK_LOAD_QUEUE_SIZE),
//             indices_staging: Vec::with_capacity(BRICK_LOAD_QUEUE_SIZE),
//         }
//     }

//     pub fn generate_supercell(&self, start: IVec3) -> Supercell {
//         let mut supercell = Supercell::new();

//         // 1. Precompute 2D heightmap for this supercell footprint (8x8 bricks * 8 voxels = 64x64)
//         let side_len = (SUPERGRID_CELL_SIZE * BRICK_SIZE) as usize;
//         let mut heightmap = vec![0.0f32; side_len * side_len];

//         for y in 0..side_len {
//             for x in 0..side_len {
//                 let world_x = (start.x * SUPERGRID_CELL_SIZE * BRICK_SIZE) + x as i32;
//                 let world_y = (start.y * SUPERGRID_CELL_SIZE * BRICK_SIZE) + y as i32;

//                 // Explicit 2.5D height calculation
//                 let h = 0.0; // TODO: replace with noise height calculation
//                 heightmap[x + y * side_len] = h;
//             }
//         }

//         // 2. Voxelize bricks
//         for z in 0..SUPERGRID_CELL_SIZE {
//             for y in 0..SUPERGRID_CELL_SIZE {
//                 for x in 0..SUPERGRID_CELL_SIZE {
//                     let mut brick = Brick::default();
//                     let mut is_empty = true;
//                     let mut lod_mask = 0u8;

//                     for bz in 0..BRICK_SIZE {
//                         let world_z = (start.z * SUPERGRID_CELL_SIZE + z) * BRICK_SIZE + bz;
//                         for by in 0..BRICK_SIZE {
//                             for bx in 0..BRICK_SIZE {
//                                 let h_val = heightmap[(bx + x * BRICK_SIZE) as usize + (by + y * BRICK_SIZE) as usize * side_len];

//                                 if (world_z as f32) < h_val {
//                                     let (word_idx, bit) = self.get_voxel_bit_index(bx, by, bz);
//                                     brick.data[word_idx] |= bit;
//                                     lod_mask |= self.get_lod_bit_index(bx, by, bz) as u8;
//                                     is_empty = false;
//                                 }
//                             }
//                         }
//                     }

//                     let idx_in_supercell = (x + y * SUPERGRID_CELL_SIZE + z * SUPERGRID_CELL_SIZE * SUPERGRID_CELL_SIZE) as usize;

//                     if !is_empty {
//                         supercell.bricks.push(brick);
//                         let brick_vec_idx = (supercell.bricks.len() - 1) as u32;
//                         supercell.indices[idx_in_supercell] = self.encode_brick_index(brick_vec_idx, lod_mask, true);
//                         // Update CPU-side counter (useful for tracking memory usage before GPU upload)
//                         supercell.gpu_index_highest += 1;
//                     } else {
//                         // Fix: Explicitly mark as unloaded/empty
//                         supercell.indices[idx_in_supercell] = BRICK_UNLOADED_BIT;
//                     }
//                 }
//             }
//         }
//         supercell
//     }

//     pub fn generate(&mut self, supergrid: IVec2) {
//         let start_time = std::time::Instant::now();
//         let supergrid_count = (supergrid.y * supergrid.x * supergrid.x) as usize;
//         //TODO: Would loop through it using multithreading
//         let generated_cells: Vec<Supercell> = (0..supergrid_count as i32)
//             .into_iter()
//             .map(|index| {
//                 let x = (index as i32) % supergrid.x;
//                 let y = ((index as i32) / supergrid.x) % supergrid.x;
//                 let z = (index as i32) / (supergrid.x * supergrid.x);
//                 self.generate_supercell(IVec3::new(x, y, z))
//             })
//             .collect();

//         self.supergrid = generated_cells.into_iter().map(|c| Some(Box::new(c))).collect();

//         println!("Generation took: {}ms", start_time.elapsed().as_millis());

//         // In Bevy, here you would create your WGPU buffers and upload
//         // the initial state (similar to your cudaMalloc/Memcpy block)
//     }

//     fn get_voxel_bit_index(&self, x: i32, y: i32, z: i32) -> (usize, u32) {
//         let bit_idx = (x + y * BRICK_SIZE + z * BRICK_SIZE * BRICK_SIZE) as usize;
//         (bit_idx / 32, 1 << (bit_idx % 32))
//     }

//     fn get_lod_bit_index(&self, x: i32, y: i32, z: i32) -> u32 {
//         // Each bit in the 8-bit mask represents a 4x4x4 quadrant of the 8x8x8 brick
//         let lod_x = (x >= 4) as u32;
//         let lod_y = (y >= 4) as u32;
//         let lod_z = (z >= 4) as u32;
//         1 << (lod_x | (lod_y << 1) | (lod_z << 2))
//     }

//     fn encode_brick_index(&self, brick_idx: u32, lod_mask: u8, is_loaded: bool) -> u32 {
//         let mut encoded = brick_idx & BRICK_INDEX_BITS;
//         if is_loaded {
//             encoded |= BRICK_LOADED_BIT;
//         } else {
//             encoded |= BRICK_UNLOADED_BIT;
//         }
//         encoded | ((lod_mask as u32) << 12)
//     }

//     pub fn dump(&self) {
//         if let Ok(mut file) = File::create("dump.txt") {
//             for cell in self.supergrid.iter().flatten() {
//                 let _ = writeln!(file, "{}", cell.gpu_index_highest);
//             }
//         }
//     }
// }
