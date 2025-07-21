

use bevy::{prelude::*, render::extract_resource::ExtractResource, window::{PresentMode, WindowMode}};


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

#[derive(Resource, Clone, ExtractResource, Default)]
pub struct SVTPushConsts { //Theoretically should be moved to the contree and renamed and loaded like window_manager.
    pub root_size: f32,
    pub root_min: Vec3,
}
#[derive(Resource, Clone, Copy,ExtractResource,Default)]
pub struct AtmosphereUniformData {
    pub sun_dir: Vec3,
    pub time: f32,
}

