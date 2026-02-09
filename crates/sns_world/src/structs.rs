use std::{
    collections::{HashMap, HashSet},
    iter,
};

use crate::baseplates::biome::ActiveBiome;
use bevy_math::*;
use sns_core::prelude::*;
use sns_macros::*;

#[derive(Default, Copy, Clone, FieldMath)]

pub struct ClimateField {
    temperature: f32,
    moisture: f32,
    continentalness: f32,
    ruggedness: f32,
}
impl std::ops::Mul<f32> for ClimateField {
    type Output = Self;
    fn mul(mut self, rhs: f32) -> Self {
        for field in self.iter_mut_val() {
            if let Some(v) = field.downcast_mut::<f32>() {
                *v *= rhs;
            }
        }
        self
    }
}
impl ClimateField {
    pub fn abs_rugedness(&mut self) {
        self.ruggedness = self.ruggedness.abs();
    }
    pub fn to_partial(&self) -> PartialClimateField {
        let mut partial = PartialClimateField::default();
        let partial_mut = partial.iter_mut();
        let full_iter = self.iter();

        for ((full_name, full_val_prim), (part_name, part_val_any)) in
            iter::zip(full_iter, partial_mut)
        {
            if full_name == part_name {
                let full_val = full_val_prim.to_f32().unwrap();
                let Some(part_val) = part_val_any.downcast_mut::<f32>() else {
                    continue;
                };
                *part_val = full_val;
            }
        }

        partial
    }
}

#[derive(Default, Copy, Clone, FieldMath)]

pub struct PartialClimateField {
    temperature: f32,
    moisture: f32,
}
#[derive(Default)]
pub struct NoiseBasis {
    noise_id: String,
    weight: ClimateField,
}
impl NoiseBasis {
    pub fn get_noise_id(&self) -> String {
        self.noise_id.clone()
    }
    pub fn get_weight(&self) -> ClimateField {
        self.weight
    }
}
#[derive(Default)]
pub struct BaseRules {
    rules: HashMap<String,BaseRule>,
}
impl BaseRules {
    pub fn get_rules(&self) -> &HashMap<String,BaseRule> {
        &self.rules
    }
    pub fn get_rule(&self,id: String) -> Option<&BaseRule> {
        self.rules.get(id.as_str())
    }
}

#[derive(Default)]
pub struct BaseRule {
    min_weight: f32,
    is_feature: bool, 
    scope: String, //regional,biome,subbiome
    rule_type: String, //Ex village,enemey_nest, dugeon, city,etc.
    rule_id: String, //Specific modifier or structure ex elven tree / goblin camp.
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
    pub fn get_rule_id(&self) -> String {
        self.rule_id.clone()
    }
}

#[derive(Default)]
pub struct LocalPlateSample {
    field: ClimateField,

    height_mean: f32,
    height_variance: f32,
    roughness: f32,
    erosion_bias: f32,
    density_curve_id: Option<String>,
    weight_sum: f32,

     base_rule_ids: Vec<String>,
}
impl LocalPlateSample {
    pub fn insert_first_density_id(&mut self, density_curve_id: Option<String>) {
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
    pub fn set_base_rule_ids(&mut self, base_rule_ids: Vec<String>) {
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

    pub fn get_base_rules(&self) -> &Vec<String> {
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
    pub fn get_density_curve_id(&self) -> Option<String> {
        self.density_curve_id.clone()
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
    pub fn sample(&self, h: f32) -> f32 {
        // Piecewise linear interpolation
        return 0.0;
    }
}

pub trait GridSampler<T> {
    fn sample_cell(&mut self, grid_coord: IVec2, seed: i32) -> T;
}

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
    active_biomes: Vec<ActiveBiome>,
    archetype_id: String,
    base_rule_ids: Vec<String>,
}

impl ClimateSample {
    pub fn new(
        field: ClimateField,
        active_biomes: Vec<ActiveBiome>,
        archetype_id: String,
        base_rule_ids: Vec<String>,
    ) -> Self {
        Self {
            field,
            active_biomes,
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

    pub fn get_active_biomes(&self) -> &Vec<ActiveBiome> {
        &self.active_biomes
    }
    pub fn set_active_biomes(&mut self, active_biomes: Vec<ActiveBiome>) {
        self.active_biomes = active_biomes;
    }
    pub fn recalculate_rules(&mut self, base_rules: &BaseRules) {
        for biome in self.active_biomes.iter_mut() {
            biome.recompute_rules(base_rules);   
        }
    }


    pub fn get_archetype_id(&self) -> String {
        self.archetype_id.clone()
    }

    pub fn get_base_rule_ids(&self) -> &Vec<String> {
        &self.base_rule_ids
    }
}
