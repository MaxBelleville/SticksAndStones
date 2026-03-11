use bevy_math::*;
use serde::Deserialize;
use sns_macros::{AssetDef, FieldMath};
use sns_core::prelude::*;
use super::{DefinedAsset, AssetRef, registry, schema};



#[derive(Default, AssetDef)]
pub struct NoiseField {
    frequency: f32,
    octaves: u8,
    gain: f32,
    lacunarity: f32,
    mult: f32,
}

#[derive(Default, AssetDef)]
pub struct BaseRule {
    min_weight: f32,
    is_feature: bool,
    scope: String,       //regional,biome,subbiome
    rule_type: String,   //Ex village,enemey_nest, dugeon, city,etc.
    #[asset_ref(BaseRule)]
    child_id: Option<BaseRuleID>, //Specific modifier or structure ex elven tree / goblin camp.
}

impl BaseRule {
    pub fn get_min_weight(&self) -> f32 {
        self.min_weight
    }
    pub fn is_feature(&self) -> bool {
        self.is_feature
    }
    pub fn get_scope(&self) -> String {
        self.scope.clone()
    }
    pub fn get_rule_type(&self) -> String {
        self.rule_type.clone()
    }
    pub fn get_child_id(&self) -> Option<BaseRuleID> {
        self.child_id
    }
}
#[derive(Default, AssetDef)]
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
pub struct DensityCurve {
    points: Vec<DensityPoint>,
}

#[derive(Default, Copy, Clone, FieldMath, AssetDef)]
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
    pub fn get_temperature(&self) -> f32 {
        self.temperature
    }
    pub fn get_moisture(&self) -> f32 {
        self.moisture
    }
}


#[derive(Default, AssetDef)]
pub struct NoiseBasis {
    #[asset_ref(NoiseField)]
    noise_id: NoiseFieldID,
    #[asset_ref(ClimateField)]
    weight_id: ClimateFieldID,
}
impl NoiseBasis {
    pub fn get_noise_id(&self) -> NoiseFieldID {
        self.noise_id
    }
    pub fn get_weight_id(&self) -> ClimateFieldID {
        self.weight_id
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
impl From<ClimateField> for PartialClimateField {
    fn from(field: ClimateField) -> Self {
        Self {
            temperature: field.get_temperature(),
            moisture: field.get_moisture(),
        }
    }
}


#[derive(Default,  AssetDef)]
pub struct RegionArchetype {
    #[asset_ref(ClimateField)]
    bias_id:  ClimateFieldID,
    #[asset_ref(ClimateField)]
    max_delta_id: Option<ClimateFieldID>,
    #[asset_ref(BaseRule)]
    base_rules: Vec<BaseRuleID>,
}


impl RegionArchetype {
   pub fn get_bias(&self) -> ClimateFieldID {
        self.bias_id
    }
    pub fn get_base_rules(&self) -> &Vec<BaseRuleID> {
        &self.base_rules
    }
    pub fn get_max_delta_id(&self) -> Option<ClimateFieldID> {
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
    #[asset_ref(TerrainProfile)]
    terrain_profile_id: TerrainProfileID,
    #[asset_ref(BaseRule)]
    base_rule_ids: Vec<BaseRuleID>,
}

#[derive(Default,  AssetDef)]
pub struct SubBiome {
    #[asset_ref(Biome)]
    parent_id: BiomeID,
    threshold: f32,
    weight: f32,
    #[asset_ref(TerrainProfile)]
    terrain_profile_id: Option<TerrainProfileID>,
    #[asset_ref(BaseRule)]
    base_rule_ids: Vec<BaseRuleID>,
}
impl SubBiome {
    pub fn get_parent_id(&self) -> BiomeID {
        self.parent_id
    }

    pub fn _get_opt_terrain_profile_id(&self) -> &Option<TerrainProfileID> {
        &self.terrain_profile_id
    }

    pub fn _get_base_rule_ids(&self) -> &Vec<BaseRuleID> {
        &self.base_rule_ids
    }
}

