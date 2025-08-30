

use bevy::{platform::collections::HashSet, prelude::*, render::{extract_component::ExtractComponent, extract_resource::ExtractResource}, window::{PresentMode, WindowMode}};
use noiz::prelude::*;


use crate::shared;

use super::{maths,contree,GameState};





#[derive(Resource, Debug, Clone,ExtractResource)]
pub struct WindowManagerSettings {
    pub display_factor: u32,
    pub workgroup_size: u32,
    pub dim: Vec4,
    pub window_mode: WindowMode,
    pub present_mode: PresentMode,
}

impl Default for WindowManagerSettings {
    fn default() -> Self {
        let display_fact = 2;
        let (width, height) =  (1280 / display_fact, 720 / display_fact);
        Self {
            display_factor: display_fact,
            workgroup_size: 8,
            dim: vec4(0.0,0.0, width as f32, height as f32),
            window_mode: WindowMode::Fullscreen(MonitorSelection::Current, VideoModeSelection::Current),
            present_mode: PresentMode::AutoVsync,
        }
    }
}

#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Hash,ExtractResource)]
pub struct ExtractedGameState(pub GameState);

#[derive(Resource,Default,Clone,ExtractResource)]
pub struct WorldData {
    pub contree: contree::Contree,
    pub seed: Option<u32>,
    pub proccessing_chunks: HashSet<IVec3>,
    pub noise: Noise<common_noise::Fbm<MixCellGradients<OrthoGrid, Smoothstep, QuickGradients>>>,
    pub gpu_data: Vec<i32>,
    pub file_data: Vec<u8>,
    pub root_size: f32,
    pub root_min: Vec3,
    pub camera: Handle<Image>,
    pub render: Handle<Image>,
}

#[derive(Component, Clone, ExtractComponent)]
#[extract_component_filter(With<Camera>)]
pub struct ComputeCamera;

#[derive(Resource, Clone, Copy,ExtractResource,Default)]
pub struct AtmosphereData {
    pub sun_dir: Vec3,
    pub time: f32,
}


#[derive(Clone, Copy, Default, Debug)]
pub struct VoxelVec {
    pub pos: IVec3,     
    pub mat: Option<u8>,
    pub level: u8, 
    pub morton: usize
} 

impl VoxelVec {
     pub fn new(pos:IVec3,mat:Option<u8>,level:u8)->Self {
        Self {
            pos,
            mat,
            level,
            morton: shared::maths::ileavechunk(
                pos.x as u32,
                pos.y as u32,
                pos.z as u32,
            )
        }
    }

    pub fn localize(&mut self) -> IVec3 {
        let chunk_coord = maths::div_floor_vec3(self.pos, 64);
        let local_pos = maths::mod_floor_vec3(self.pos, 64);

        let max_level = (64 as f32).log(4.0) as u8;
        if self.level > max_level {
            self.level = max_level;
        }
        self.pos = local_pos;
        self.update_morton();
        return chunk_coord;
    }

    pub fn normalize(&mut self, size: i32) {
        self.pos+= IVec3::splat(size/ 2);
        self.update_morton();
    }

    pub fn chunk_to_world(&self, chunk_pos:IVec3) -> IVec3 {
        (chunk_pos * 64) + self.pos
    }

    pub fn update_morton(&mut self) {
        self.morton = shared::maths::ileavechunk(
            self.pos.x as u32,
            self.pos.y as u32,
            self.pos.z as u32,
        );
    }
}

impl PartialEq<Option<u8>> for VoxelVec {
    fn eq(&self, other: &Option<u8>) -> bool {
        self.mat == *other
    }
}