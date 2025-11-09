use bevy::{
    platform::collections::HashSet,
    prelude::*,
    render::{extract_component::ExtractComponent, extract_resource::ExtractResource},
    window::{PresentMode, WindowMode},
};



use super::{GameState,contree, maths};

#[derive(Resource, Debug, Clone, ExtractResource)]
pub struct WindowManagerSettings {
    pub display_factor: f32,
    pub dim: Vec4,
    pub window_mode: WindowMode,
    pub present_mode: PresentMode,
}

impl Default for WindowManagerSettings {
    fn default() -> Self {
        let display_fact = 2.0;
        let (width, height) = (1366.0 / display_fact, 768.0 / display_fact);
        Self {
            display_factor: display_fact,
            dim: vec4(0.0, 0.0, width, height),
            window_mode: WindowMode::Fullscreen(
                MonitorSelection::Current,
                VideoModeSelection::Current,
            ),
            present_mode: PresentMode::AutoVsync,
        }
    }
}

#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Hash, ExtractResource)]
pub struct ExtractedGameState(pub GameState);

#[derive(Resource, Default, Clone, ExtractResource)]
pub struct WorldData {
    pub contree: contree::Contree,
    pub seed: Option<u32>,
    pub proccessing_chunks: HashSet<IVec3>,
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

#[derive(Resource, Clone, Copy, ExtractResource, Default)]
pub struct AtmosphereData {
    pub sun_dir: Vec3,
    pub time: f32,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Voxel {
    pub pos: IVec3,
    pub mat:u8,
    pub level: u8,
    pub morton: usize,
}

impl Voxel {
    pub fn new(pos: IVec3, mat: u8, level: u8) -> Self {
        Self {
            pos,
            mat,
            level,
            morton: maths::ileavechunk(pos.x as u32, pos.y as u32, pos.z as u32),
        }
    }

    pub fn localize(&mut self, extent: usize) -> IVec3 {
        let chunk_coord = maths::div_floor_vec3(self.pos, extent as i32);
        let local_pos = maths::mod_floor_vec3(self.pos, extent as i32);

        let max_level = (extent as f32).log(4.0) as u8;
        if self.level > max_level {
            self.level = max_level;
        }
        self.pos = local_pos;
        self.update_morton();
        return chunk_coord;
    }

    pub fn volume(&self)->usize {
        let extent = maths::level_extent(self.level as usize);
        extent*extent*extent 
    }

    pub fn normalize(&mut self, size: i32) {
        self.pos += IVec3::splat(size / 2);
        self.update_morton();
    }

    pub fn chunk_to_world(&self, chunk_pos: IVec3) -> IVec3 {
        (chunk_pos * 64) + self.pos
    }

    pub fn update_morton(&mut self) {
        self.morton = maths::ileavechunk(self.pos.x as u32, self.pos.y as u32, self.pos.z as u32);
    }
}
