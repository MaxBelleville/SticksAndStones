use super::{handler, shared};
use bevy::math::*;

///Small helper that proccesses per 64x64x64 areas of voxels and helps proccess lod.
#[derive(Debug)]
pub struct Chunk {
    handler: handler::Handler,
    extent: usize,    // usually 64
    base_root: usize, // index into NodePool
}

impl Chunk {
    pub fn new(voxel_size: usize) -> Self {
        let est_cap = shared::maths::estimate_nodes(voxel_size, 64);
        let (handler, base_root) = handler::Handler::new(est_cap, 3);
        Self {
            handler,
            extent: 64,
            base_root,
        }
    }

    pub fn get_material(&self, local_point: IVec3) -> u8 {
        self.handler.get_material(local_point, self.base_root, 3)
    }

    //Uses tree handler
    pub fn build_voxel(&mut self, voxel: shared::res::Voxel) {
        self.handler.build_voxel(voxel, self.base_root, 0,3);
    }

    //As needs chunk data can only happen here.
    pub fn build_region(&mut self, voxels: Vec<shared::res::Voxel>, min_level: usize) {
        self.handler
            .build_region(voxels, self.base_root, 64, ivec3(0, 0, 0), min_level);
    }
}
