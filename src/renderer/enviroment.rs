

use sns_assets::enviroment::Preset;
use sns_core::prelude::{dir_from_angle};

use bevy::{
    prelude::*,
    render::{extract_resource::ExtractResource, render_resource::ShaderType},
};
use bytemuck::{Pod, Zeroable};
#[repr(C)]
#[derive(Resource, Default, ExtractResource, Clone, ShaderType, Copy, Pod, Zeroable)]
pub struct EnvironmentUniform {
    dynamic: EnviromentInterpolated,
    constants: EnviromentConsts,
}
impl EnvironmentUniform {
    pub fn set_interpulated(&mut self, preset: EnviromentInterpolated) {
        self.dynamic = preset;
    }
}

#[repr(C)]
#[derive(Clone, Copy, ShaderType, Pod, Zeroable)]
pub struct EnviromentConsts {
    sky_mid_height: f32,
    sky_mid_softness: f32,

    sun_size: f32,
    alpha_moon_size: f32,
    beta_moon_size: f32,
    sun_softness: f32,

    gradient_exponent: f32,

    fog_height_falloff: f32,
    sky_quantization_levels: f32,
}
impl Default for EnviromentConsts {
    fn default() -> Self {
        Self {
            sky_mid_height: 0.35,
            sky_mid_softness: 0.25,
            sun_size: 0.007,
            sun_softness: 0.0003,
            alpha_moon_size: 0.003,
            beta_moon_size: 0.0005,
            gradient_exponent: 0.5,
            fog_height_falloff: 0.01,
            sky_quantization_levels: 64.0,
        }
    }
}

#[repr(C)]
#[derive(Default, Clone, Copy, ShaderType, Pod, Zeroable)]
pub struct EnviromentInterpolated {
    sun_direction: Vec3,
    sun_intensity: f32,
    sun_color: Vec3,

    alpha_moon_direction: Vec3,
    alpha_moon_intensity: f32,
    alpha_moon_color: Vec3,

    beta_moon_direction: Vec3,
    beta_moon_intensity: f32,
    beta_moon_color: Vec3,

    zenith_color: Vec3,
    mid_color: Vec3,
    horizon_color: Vec3,

    fog_density: f32,
}
impl From<Preset> for EnviromentInterpolated {
    fn from(value: Preset) -> Self {
        Self {
            sun_color: value.sun_color(),
            alpha_moon_color: value.alpha_moon_color(),
            beta_moon_color: value.beta_moon_color(),

            zenith_color: value.zenith_color(),
            mid_color: value.mid_color(),
            horizon_color: value.horizon_color(),
            ..default()
        }
    }
}  
impl EnviromentInterpolated {
    pub fn set_sun(&mut self, angle: f32, intensity: f32) {
        let sun_dir = dir_from_angle(angle);
        self.sun_intensity = intensity;
        self.sun_direction = sun_dir;
    }
    pub fn set_moons(&mut self, angle: f32, intensity: f32) {
        let mut alpha_moon_dir = dir_from_angle(angle);
        let mut beta_moon_dir = dir_from_angle(angle);

        beta_moon_dir.y *= 0.8;
        beta_moon_dir = Quat::from_rotation_y(0.7) * beta_moon_dir;

        alpha_moon_dir.y *= 1.2;

        self.alpha_moon_intensity = intensity;
        self.alpha_moon_direction = alpha_moon_dir;

        self.beta_moon_intensity = intensity;
        self.beta_moon_direction = beta_moon_dir;
    }
    pub fn set_fog(&mut self, density: f32) {
        self.fog_density = density;
    }
}
