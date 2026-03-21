use bevy_math::*;
use sns_macros::AssetDef;
use super::{DefinedAsset,registry,metadata, AssetRef};

#[derive(Default,AssetDef)]
#[asset(type = "Curve", namespace = "enviroment")]
pub struct Curve {
    values: Vec<f32>,
}
#[derive(Default,AssetDef)]
#[asset(type = "Preset", namespace = "enviroment")]
pub struct Preset {
    sun_color: Vec3,
    alpha_moon_color: Vec3,
    beta_moon_color: Vec3,

    horizon_color: Vec3,
    mid_color: Vec3,
    zenith_color: Vec3,
}
#[derive(Default,AssetDef)]
#[asset(type = "PresetSegement", namespace = "enviroment")]
pub struct PresetSegement {
    #[asset_ref(Curve)]
    curve_id: CurveID,
    #[asset_ref(Preset)]
    start_id: PresetID,
    #[asset_ref(Preset)]
    end_id: PresetID,
    is_first: bool
}
