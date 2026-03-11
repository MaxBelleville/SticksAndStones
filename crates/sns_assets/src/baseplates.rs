use bevy_math::*;
use serde::Deserialize;
use sns_macros::{AssetDef, FieldMath};
use sns_core::prelude::*;
use super::{DefinedAsset, AssetRef, registry, schema, worldshared};


#[derive(Default, AssetDef)]
pub struct NoiseBasis {
    #[asset_ref(worldshared::NoiseField)]
    noise_id: worldshared::NoiseFieldID,
    #[asset_ref(worldshared::ClimateField)]
    weight_id: worldshared::ClimateFieldID,
}
impl NoiseBasis {
    pub fn get_noise_id(&self) -> worldshared::NoiseFieldID {
        self.noise_id
    }
    pub fn get_weight_id(&self) -> worldshared::ClimateFieldID {
        self.weight_id
    }
}

#[derive(Default,  AssetDef)]
pub struct RegionArchetype {
    #[asset_ref(worldshared::ClimateField)]
    bias_id:  worldshared::ClimateFieldID,
    #[asset_ref(worldshared::ClimateField)]
    max_delta_id: Option<worldshared::ClimateFieldID>,
    #[asset_ref(worldshared::BaseRule)]
    base_rule_ids: Vec<worldshared::BaseRuleID>,
}


impl RegionArchetype {
   pub fn get_bias(&self) -> worldshared::ClimateFieldID {
        self.bias_id
    }
    pub fn get_rule_ids(&self) -> &Vec<worldshared::BaseRuleID> {
        &self.base_rule_ids
    }
    pub fn get_max_delta_id(&self) -> Option<worldshared::ClimateFieldID> {
        self.max_delta_id
    }
}

#[derive(Default,  AssetDef)]
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

#[derive(Default,  AssetDef)]
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
impl SubBiome {
    pub fn get_parent_id(&self) -> BiomeID {
        self.parent_id
    }

    pub fn _get_opt_terrain_profile_id(&self) -> &Option<worldshared::TerrainProfileID> {
        &self.terrain_profile_id
    }

    pub fn _get_rule_ids(&self) -> &Vec<worldshared::BaseRuleID> {
        &self.base_rule_ids
    }
}

#[derive(Default, Copy, Clone, FieldMath, Deserialize)]

pub struct PartialClimateField {
   temperature: f32,
   moisture: f32,
}
impl PartialClimateField {
    pub fn new(temperature: f32, moisture: f32) -> Self {
        Self { temperature, moisture }
    }
}
impl From<worldshared::ClimateField> for PartialClimateField {
    fn from(field: worldshared::ClimateField) -> Self {
        Self {
            temperature: field.get_temperature(),
            moisture: field.get_moisture(),
        }
    }
}

