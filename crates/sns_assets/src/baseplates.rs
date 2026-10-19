use super::{AssetRef, StaticAsset, metadata, registry, worldshared};
use bevy_math::*;
use serde::Deserialize;
use sns_macros::{FieldMath, MakeAsset};

#[derive(Default, MakeAsset)]
#[asset(namespace = "baseplates")]
pub struct NoiseBasis {
    #[asset_ref(worldshared::NoiseField)]
    noise_id: worldshared::NoiseFieldID,
    #[asset_ref(worldshared::ClimateField)]
    weight_id: worldshared::ClimateFieldID,
}

#[derive(Default, MakeAsset)]
#[asset(namespace = "baseplates")]
pub struct RegionArchetype {
    #[asset_ref(worldshared::ClimateField)]
    bias_id: worldshared::ClimateFieldID,
    #[asset_ref(worldshared::ClimateField)]
    max_delta_id: Option<worldshared::ClimateFieldID>,
    #[asset_ref(worldshared::BaseRule)]
    base_rule_ids: Vec<worldshared::BaseRuleID>,
}

#[derive(Default, MakeAsset)]
#[asset(namespace = "baseplates")]
pub struct Biome {
    // Climate preference
    center: PartialClimateField,
    width: PartialClimateField,
    #[asset_ref(SubBiome)]
    sub_ids: Vec<SubBiomeID>,

    // What terrain this biome produces
    #[asset_ref(worldshared::TerrainProfile)]
    terrain_profile_id: worldshared::TerrainProfileID,
    #[asset_ref(worldshared::BaseRule)]
    base_rule_ids: Vec<worldshared::BaseRuleID>,
}

#[derive(Default, MakeAsset)]
#[asset(namespace = "baseplates")]
pub struct SubBiome {
    #[asset_ref(Biome)]
    parent_id: BiomeID,
    threshold: f32,
    weight: f32,
    #[asset_ref(worldshared::TerrainProfile)]
    terrain_profile_id: Option<worldshared::TerrainProfileID>,
    #[asset_ref(worldshared::BaseRule)]
    base_rule_ids: Vec<worldshared::BaseRuleID>,
}

#[derive(Default, Copy, Clone, FieldMath, Deserialize)]

pub struct PartialClimateField {
    temperature: f32,
    moisture: f32,
}
impl PartialClimateField {
    pub fn new(temperature: f32, moisture: f32) -> Self {
        Self {
            temperature,
            moisture,
        }
    }
    pub fn get_temperature(&self) -> f32 {
        self.temperature
    }
    pub fn get_moisture(&self) -> f32 {
        self.moisture
    }
}
impl From<worldshared::ClimateField> for PartialClimateField {
    fn from(field: worldshared::ClimateField) -> Self {
        Self {
            temperature: field.temperature().clone(),
            moisture: field.moisture().clone(),
        }
    }
}
