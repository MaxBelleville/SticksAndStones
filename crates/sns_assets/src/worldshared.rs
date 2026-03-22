use bevy_math::*;
use serde::Deserialize;
use sns_macros::{AssetDef, FieldMath};
use super::{DefinedAsset, AssetRef, registry, metadata,IterableMut,IterableRef};


#[derive(Default, AssetDef)]
#[asset(namespace = "worldshared")]
pub struct NoiseField {
    frequency: f32,
    octaves: u8,
    gain: f32,
    lacunarity: f32,
    mult: f32,
}


#[derive(Default, Copy, Clone, FieldMath, AssetDef)]
#[asset(namespace = "worldshared")]
pub struct ClimateField {
    temperature: f32,
    moisture: f32,
    continentalness: f32,
    ruggedness: f32,
}
impl std::ops::Mul<f32> for ClimateField {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self {
        Self {
            temperature: self.temperature * rhs,
            moisture: self.moisture * rhs,
            continentalness: self.continentalness * rhs,
            ruggedness: self.ruggedness * rhs,
        }
    }
}
impl ClimateField {
    pub fn abs_rugedness(&mut self) {
        self.ruggedness = self.ruggedness.abs();
    }
}


#[derive(Default, AssetDef)]
#[asset(namespace = "worldshared")]
pub struct BaseRule {
    min_weight: f32,
    is_feature: bool,
    scope: String,       //regional,biome,subbiome
    rule_type: String,   //Ex village,enemey_nest, dugeon, city,etc.
    #[asset_ref(BaseRule)]
    child_id: Option<BaseRuleID>, //Specific modifier or structure ex elven tree / goblin camp.
}

#[derive(Default, AssetDef)]
#[asset(namespace = "worldshared")]
pub struct TerrainProfile {
    #[asset_ref(NoiseField)]
    base_noise: NoiseFieldID,
    height_offset: f32,   // mean elevation contribution
    height_variance: f32, // amplitude of deviation
    roughness: f32,
    erosion_strength: f32,
    #[asset_ref(DensityCurve)]
    density_curve_id: Option<DensityCurveID>,
}

#[derive(Default,Deserialize)]

pub struct DensityPoint {
    height: f32,
    density: f32,
}

#[derive(Default, AssetDef)]
#[asset(namespace = "worldshared")]
pub struct DensityCurve {
    points: Vec<DensityPoint>,
}
