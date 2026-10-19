use super::{AssetRef, StaticAsset, metadata, registry};
use bevy_math::*;

use sns_macros::MakeAsset;

#[derive(Default, MakeAsset, Clone)]
#[asset(namespace = "environment")]
pub struct Curve {
    values: Vec<f32>,
}
#[derive(Default, MakeAsset, Clone, Copy)]
#[asset(namespace = "environment")]
pub struct Preset {
    sun_color: Vec3,
    alpha_moon_color: Vec3,
    beta_moon_color: Vec3,

    horizon_color: Vec3,
    mid_color: Vec3,
    zenith_color: Vec3,
}
impl Preset {
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

#[derive(Default, MakeAsset)]
#[asset(namespace = "environment")]
pub struct PresetSegement {
    #[asset_ref(Curve)]
    curve_id: CurveID,
    #[asset_ref(Preset)]
    start_id: PresetID,
    #[asset_ref(Preset)]
    end_id: PresetID,
    duration: Vec2,
}
