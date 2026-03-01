use std::collections::{HashMap, HashSet};

use bevy_math::*;
use sns_core::prelude::*;
use sns_macros::*;

use crate::prelude::biome;

#[derive(Default, Copy, Clone, FieldMath)]

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
    pub fn to_partial(&self) -> PartialClimateField {
        PartialClimateField {
            temperature: self.temperature,
            moisture: self.moisture,
        }
    }
}

#[derive(Default, Copy, Clone, FieldMath)]

pub struct PartialClimateField {
    temperature: f32,
    moisture: f32,
}
#[derive(Default)]
pub struct NoiseBasis {
    noise_id: NoiseId,
    weight: ClimateField,
}
impl NoiseBasis {
    pub fn get_noise_id(&self) -> NoiseId {
        self.noise_id
    }
    pub fn get_weight(&self) -> ClimateField {
        self.weight
    }
}
#[derive(Default)]
pub struct NoiseBasisPlate {
    climate: NoiseBasis,
    region: NoiseBasis,
    local: NoiseBasis,
}
impl NoiseBasisPlate {
    pub fn get_climate(&self) -> &NoiseBasis {
        &self.climate
    }
    pub fn get_region(&self) -> &NoiseBasis {
        &self.region
    }
    pub fn get_local(&self) -> &NoiseBasis {
        &self.local
    }
    pub fn get_climate_id(&self) -> NoiseId {
        self.climate.get_noise_id()
    }
    pub fn get_region_id(&self) -> NoiseId {
        self.region.get_noise_id()
    }
    pub fn get_local_id(&self) -> NoiseId {
        self.local.get_noise_id()
    }
    pub fn get_climate_weight(&self) -> ClimateField {
        self.climate.get_weight()
    }
    pub fn get_region_weight(&self) -> ClimateField {
        self.region.get_weight()
    }
    pub fn get_local_weight(&self) -> ClimateField {
        self.local.get_weight()
    }
}


#[derive(Default)]
pub struct BaseRules {
    rules: HashMap<BaseRuleId, BaseRule>,
}
impl BaseRules {
    pub fn get_rules(&self) -> &HashMap<BaseRuleId, BaseRule> {
        &self.rules
    }
    pub fn get_rule(&self, id: BaseRuleId) -> Option<&BaseRule> {
        self.rules.get(&id)
    }

    pub fn apply_rules(&self, samples: &mut Vec<biome::Sample>) {
        for sample in samples {
            let weight = sample.get_weight();
            self.recompute_rules(sample.get_mut_rule_ids(), weight);
            for sub in sample.get_mut_sub_sample() {
                let sub_weight = sub.get_weight();
                self.recompute_rules(sub.get_mut_rule_ids(), sub_weight);
            }
        }
    }
    pub fn recompute_rules(&self, rule_ids: &mut Vec<BaseRuleId>, weight: f32) {
        rule_ids.retain(|&rule_id| {
            if let Some(rule) = self.rules.get(&rule_id) {
                weight >= rule.get_min_weight()
            } else {
                false
            }
        });
    }
}

#[derive(Default)]
pub struct BaseRule {
    min_weight: f32,
    is_feature: bool,
    scope: String,       //regional,biome,subbiome
    rule_type: String,   //Ex village,enemey_nest, dugeon, city,etc.
    rule_id: BaseRuleId, //Specific modifier or structure ex elven tree / goblin camp.
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
    pub fn get_rule_id(&self) -> BaseRuleId {
        self.rule_id
    }
}

#[derive(Default)]
pub struct LocalPlateSample {
    field: ClimateField,

    height_mean: f32,
    height_variance: f32,
    roughness: f32,
    erosion_bias: f32,
    density_curve_id: Option<DensityCurveId>,
    weight_sum: f32,

    base_rule_ids: Vec<BaseRuleId>,
}
impl LocalPlateSample {
    pub fn insert_first_density_id(&mut self, density_curve_id: Option<DensityCurveId>) {
        //If self.density_curve_id doesn't exists and parameter density_curve_id exists add
        if self.density_curve_id.is_none() && density_curve_id.is_some() {
            self.density_curve_id = density_curve_id;
        }
    }
    pub fn normalize(&mut self) {
        //normalize height_mean, height_variance, roughness, erosion_bias using weight sum
        self.height_mean /= self.weight_sum;
        self.height_variance /= self.weight_sum;
        self.roughness /= self.weight_sum;
        self.erosion_bias /= self.weight_sum;
        self.weight_sum = 1.0; // Just to make sure it's impossible to normalize again without changing values.
    }
    pub fn set_base_rule_ids(&mut self, base_rule_ids: Vec<BaseRuleId>) {
        self.base_rule_ids = base_rule_ids;
    }
    pub fn set_field(&mut self, field: ClimateField) {
        self.field = field;
    }
    pub fn add_terrain(
        &mut self,
        height_mean: f32,
        height_variance: f32,
        roughness: f32,
        erosion_bias: f32,
        weight: f32,
    ) {
        self.height_mean += height_mean;
        self.height_variance += height_variance;
        self.roughness += roughness;
        self.erosion_bias += erosion_bias;
        self.weight_sum += weight;
    }

    pub fn get_base_rules(&self) -> &Vec<BaseRuleId> {
        &self.base_rule_ids
    }

    pub fn get_height_mean(&self) -> f32 {
        self.height_mean
    }
    pub fn get_height_variance(&self) -> f32 {
        self.height_variance
    }
    pub fn get_roughness(&self) -> f32 {
        self.roughness
    }
    pub fn get_erosion_bias(&self) -> f32 {
        self.erosion_bias
    }
    pub fn get_density_curve_id(&self) -> Option<DensityCurveId> {
        self.density_curve_id
    }
    pub fn get_weight_sum(&self) -> f32 {
        self.weight_sum
    }
    pub fn get_field(&self) -> ClimateField {
        self.field
    }
}

#[derive(Default)]
pub struct DensityPoint {
    height: f32,
    density: f32,
}

#[derive(Default)]
pub struct DensityCurve {
    points: Vec<DensityPoint>,
}
impl DensityCurve {
    pub fn sample(&self, _h: f32) -> f32 {
        // Piecewise linear interpolation
        return 0.0;
    }
}

pub trait GridSampler<T> {
    fn sample_cell(&mut self, grid_coord: IVec2, seed: i32) -> T;
}
#[derive(Default)]
pub struct GridSampleCache<T> {
    cell_res: i32,
    grid_size: f32,
    grid: HashMap<IVec2, T>,
}

impl<T> GridSampleCache<T> {
    pub fn new(cell_res: i32, grid_size: f32) -> Self {
        Self {
            cell_res,
            grid_size,
            grid: HashMap::new(),
        }
    }
    pub fn required_cells(&self, center: IVec2) -> HashSet<IVec2> {
        let mut set = HashSet::new();

        for y in -self.cell_res..=self.cell_res {
            for x in -self.cell_res..=self.cell_res {
                set.insert(center + IVec2::new(x, y));
            }
        }

        set
    }
    pub fn pos_to_cell(&self, pos: Vec2) -> IVec2 {
        IVec2::new(
            (pos.x / self.grid_size).floor() as i32,
            (pos.y / self.grid_size).floor() as i32,
        )
    }
    pub fn pos_reduce_to_cell(&self, pos: Vec2) -> Vec2 {
        Vec2::new(pos.x / self.grid_size, pos.y / self.grid_size)
    }
    pub fn get_grid_size(&self) -> f32 {
        self.grid_size
    }
    pub fn get_cell_res(&self) -> i32 {
        self.cell_res
    }
    pub fn get_cell(&self, grid_coord: IVec2) -> &T {
        self.grid.get(&grid_coord).unwrap()
    }
    pub fn get_mut_values(&mut self) -> Vec<&mut T> {
        self.grid.values_mut().collect()
    }

    pub fn update<S: GridSampler<T>>(&mut self, sampler: &mut S, world_pos: Vec2, seed: i32) {
        let center = self.pos_to_cell(world_pos);
        let required = self.required_cells(center);

        self.grid
            .retain(|grid_coord, _| required.contains(grid_coord));

        for grid_coord in required {
            if !self.grid.contains_key(&grid_coord) {
                let value = sampler.sample_cell(grid_coord, seed);
                self.grid.insert(grid_coord, value);
            }
        }
    }
}

#[derive(Default, Clone)]
pub struct ClimateSample {
    field: ClimateField,
    biome_samples: Vec<biome::Sample>,
    archetype_id: RegionArchetypeId,
    base_rule_ids: Vec<BaseRuleId>,
}

impl ClimateSample {
    pub fn new(
        field: ClimateField,
        biome_samples: Vec<biome::Sample>,
        archetype_id: RegionArchetypeId,
        base_rule_ids: Vec<BaseRuleId>,
    ) -> Self {
        Self {
            field,
            biome_samples,
            archetype_id,
            base_rule_ids,
        }
    }

    pub fn get_field(&self) -> ClimateField {
        self.field
    }
    pub fn set_field(&mut self, field: ClimateField) {
        self.field = field;
    }

    pub fn get_biome_samples(&self) -> &Vec<biome::Sample> {
        &self.biome_samples
    }
     pub fn get_mut_biome_samples(&mut self) -> &mut Vec<biome::Sample> {
        &mut self.biome_samples
    }
    pub fn set_biomes_samples(&mut self, biome_samples: Vec<biome::Sample>) {
        self.biome_samples = biome_samples;
    }

    pub fn get_archetype_id(&self) -> RegionArchetypeId {
        self.archetype_id
    }

    pub fn get_base_rule_ids(&self) -> &Vec<BaseRuleId> {
        &self.base_rule_ids
    }
  
}

#[derive(Default, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TerrainProfileId {
    pub val: u32,
}

#[derive(Default, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct NoiseId {
    pub val: u32,
}

#[derive(Default, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BiomeId {
    pub val: u32,
}

#[derive(Default, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SubBiomeId {
    pub val: u32,
}

#[derive(Default, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BaseRuleId {
    pub val: u32,
}

#[derive(Default, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RegionArchetypeId {
    pub val: u32,
}

#[derive(Default, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct DensityCurveId {
    pub val: u32,
}

pub struct WorldGlossary {
    terrain_profiles: IdVolume<TerrainProfileId>,
    noise_fields: IdVolume<NoiseId>,
    biomes: IdVolume<BiomeId>,
    sub_biomes: IdVolume<SubBiomeId>,
    region_archetypes: IdVolume<RegionArchetypeId>,
    base_rules: IdVolume<BaseRuleId>,
    density_curves: IdVolume<DensityCurveId>,
}
