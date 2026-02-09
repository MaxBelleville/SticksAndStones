use bevy::{
    prelude::*,
    render::extract_resource::ExtractResource,
    window::{PresentMode, WindowMode},
};


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

#[derive(Resource, Clone, Copy, ExtractResource, Default)]
pub struct AtmosphereData {
    pub sun_dir: Vec3,
    pub time: f32,
}