
use std::f32::consts::{PI, TAU};

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
impl From<EnviromentPreset> for EnviromentInterpolated {
    fn from(value: EnviromentPreset) -> Self {
        Self {
            sun_color: value.sun_color,
            alpha_moon_color: value.alpha_moon_color,
            beta_moon_color: value.beta_moon_color,

            zenith_color: value.zenith_color,
            mid_color: value.mid_color,
            horizon_color: value.horizon_color,
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
#[derive(Clone, Copy)]
pub struct EnviromentPreset {
    sun_color: Vec3,
    alpha_moon_color: Vec3,
    beta_moon_color: Vec3,

    zenith_color: Vec3,
    mid_color: Vec3,
    horizon_color: Vec3,
}

impl EnviromentPreset {
    pub fn dawn_preset() -> Self {
        Self {
            sun_color: vec3(1.0, 0.75, 0.55),
            alpha_moon_color: vec3(0.739, 0.716, 0.676),
            beta_moon_color: vec3(0.608, 0.529, 0.792),

            zenith_color: vec3(0.2, 0.3, 0.6),
            mid_color: vec3(0.6, 0.4, 0.7),
            horizon_color: vec3(1.0, 0.45, 0.3),
        }
    }

    pub fn noon_preset() -> Self {
        Self {
            sun_color: vec3(1.0, 1.0, 1.0),
            alpha_moon_color: vec3(0.688, 0.704, 0.716),
            beta_moon_color: vec3(0.749, 0.769, 0.788),

            zenith_color: vec3(0.1, 0.35, 0.9),
            mid_color: vec3(0.3, 0.55, 1.0),
            horizon_color: vec3(0.6, 0.8, 1.0),
        }
    }

    pub fn dusk_preset() -> Self {
        Self {
            sun_color: vec3(1.0, 0.65, 0.5),
            alpha_moon_color: vec3(0.382, 0.333, 0.731),
            beta_moon_color: vec3(0.886, 0.784, 0.710),

            zenith_color: vec3(0.25, 0.2, 0.45),
            mid_color: vec3(0.8, 0.3, 0.4),
            horizon_color: vec3(1.0, 0.4, 0.3),
        }
    }

    pub fn midnight_preset() -> Self {
        Self {
            sun_color: vec3(0.7, 0.7, 0.8),
            alpha_moon_color: vec3(0.688, 0.704, 0.743),
            beta_moon_color: vec3(0.447, 0.388, 0.851),

            zenith_color: vec3(0.02, 0.03, 0.07),
            mid_color: vec3(0.04, 0.04, 0.09),
            horizon_color: vec3(0.05, 0.06, 0.1),
        }
    }
    pub fn lerp(&self, other: &Self, lerp_t: f32) -> Self {
        Self {
            sun_color: self.sun_color.lerp(other.sun_color, lerp_t),
            alpha_moon_color: self.alpha_moon_color.lerp(other.alpha_moon_color, lerp_t),
            beta_moon_color: self.beta_moon_color.lerp(other.beta_moon_color, lerp_t),

            zenith_color: self.zenith_color.lerp(other.zenith_color, lerp_t),
            mid_color: self.mid_color.lerp(other.mid_color, lerp_t),
            horizon_color: self.horizon_color.lerp(other.horizon_color, lerp_t),
        }
    }
}
