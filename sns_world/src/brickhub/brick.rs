pub struct Brick {
    occupancy_mask: u64,      // 1 bit per voxel (optional but recommended)
    material_base: u32,        // Base index into material bucket
}