use std::collections::{HashMap, HashSet};

use bevy_math::*;

use crate::prelude::biome;
use sns_assets::prelude::{
    BaseRule, BaseRuleID, ClimateField, ClimateFieldID, NoiseBasis, NoiseFieldID,
    RegionArchetypeID, DensityCurveID,  
};

#[derive(Default)]
pub struct NoiseBasisPlate {
    climate: NoiseBasis,
    region: NoiseBasis,
    local: NoiseBasis,
}
impl NoiseBasisPlate {
    pub fn climate(&self) -> &NoiseBasis {
        &self.climate
    }
    pub fn region(&self) -> &NoiseBasis {
        &self.region
    }
    pub fn local(&self) -> &NoiseBasis {
        &self.local
    }
    pub fn climate_id(&self) -> &NoiseFieldID {
        self.climate.noise_id()
    }
    pub fn region_id(&self) -> &NoiseFieldID {
        self.region.noise_id()
    }
    pub fn local_id(&self) -> &NoiseFieldID {
        self.local.noise_id()
    }
    pub fn climate_weight_id(&self) -> &ClimateFieldID {
        self.climate.weight_id()
    }
    pub fn region_weight_id(&self) -> &ClimateFieldID {
        self.region.weight_id()
    }
    pub fn local_weight_id(&self) -> &ClimateFieldID {
        self.local.weight_id()
    }
}

#[derive(Default)]
pub struct BaseRules {
    rules: HashMap<BaseRuleID, BaseRule>,
}
impl BaseRules {
    pub fn rules(&self) -> &HashMap<BaseRuleID, BaseRule> {
        &self.rules
    }
    pub fn rule(&self, id: BaseRuleID) -> Option<&BaseRule> {
        self.rules.get(&id)
    }

    pub fn apply_rules(&self, samples: &mut Vec<biome::Sample>) {
        for sample in samples {
            let weight = sample.weight();
            self.recompute_rules(sample.mut_rule_ids(), weight);
            for sub in sample.mut_sub_sample() {
                let sub_weight = sub.weight();
                self.recompute_rules(sub.mut_rule_ids(), sub_weight);
            }
        }
    }
    pub fn recompute_rules(&self, rule_ids: &mut Vec<BaseRuleID>, weight: f32) {
        rule_ids.retain(|&rule_id| {
            if let Some(rule) = self.rules.get(&rule_id) {
                weight >= rule.min_weight()
            } else {
                false
            }
        });
    }
}

#[derive(Default)]
pub struct LocalPlateSample {
    field: ClimateField,

    height_mean: f32,
    height_variance: f32,
    roughness: f32,
    erosion_bias: f32,
    density_curve_id: Option<DensityCurveID>,
    weight_sum: f32,

    base_rule_ids: Vec<BaseRuleID>,
}
impl LocalPlateSample {
    pub fn insert_first_density_id(&mut self, density_curve_id: &Option<DensityCurveID>) {
        //If self.density_curve_id doesn't exists and parameter density_curve_id exists add
        if self.density_curve_id.is_none() && density_curve_id.is_some() {
            self.density_curve_id = density_curve_id.clone();
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
    pub fn set_base_rule_ids(&mut self, base_rule_ids: Vec<BaseRuleID>) {
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

    pub fn base_rules(&self) -> &Vec<BaseRuleID> {
        &self.base_rule_ids
    }

    pub fn height_mean(&self) -> f32 {
        self.height_mean
    }
    pub fn height_variance(&self) -> f32 {
        self.height_variance
    }
    pub fn roughness(&self) -> f32 {
        self.roughness
    }
    pub fn erosion_bias(&self) -> f32 {
        self.erosion_bias
    }
    pub fn density_curve_id(&self) -> Option<DensityCurveID> {
        self.density_curve_id
    }
    pub fn weight_sum(&self) -> f32 {
        self.weight_sum
    }
    pub fn field(&self) -> ClimateField {
        self.field
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
    pub fn grid_size(&self) -> f32 {
        self.grid_size
    }
    pub fn cell_res(&self) -> i32 {
        self.cell_res
    }
    pub fn cell(&self, grid_coord: IVec2) -> &T {
        self.grid.get(&grid_coord).unwrap()
    }
    pub fn mut_values(&mut self) -> Vec<&mut T> {
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
    archetype_id: RegionArchetypeID,
    base_rule_ids: Vec<BaseRuleID>,
}

impl ClimateSample {
    pub fn new(
        field: ClimateField,
        biome_samples: Vec<biome::Sample>,
        archetype_id: RegionArchetypeID,
        base_rule_ids: Vec<BaseRuleID>,
    ) -> Self {
        Self {
            field,
            biome_samples,
            archetype_id,
            base_rule_ids,
        }
    }

    pub fn field(&self) -> ClimateField {
        self.field
    }
    pub fn set_field(&mut self, field: ClimateField) {
        self.field = field;
    }

    pub fn biome_samples(&self) -> &Vec<biome::Sample> {
        &self.biome_samples
    }
    pub fn mut_biome_samples(&mut self) -> &mut Vec<biome::Sample> {
        &mut self.biome_samples
    }
    pub fn set_biomes_samples(&mut self, biome_samples: Vec<biome::Sample>) {
        self.biome_samples = biome_samples;
    }

    pub fn archetype_id(&self) -> RegionArchetypeID {
        self.archetype_id
    }

    pub fn base_rule_ids(&self) -> &Vec<BaseRuleID> {
        &self.base_rule_ids
    }
}
