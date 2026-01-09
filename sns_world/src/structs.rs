use std::collections::{HashMap, HashSet};

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


#[derive(Default, Iterable)]
#[iterable(ToString)]

pub struct NoiseIds {
    temperature: String,
    moisture: String,
    continentalness: String,
    ruggedness: String,
}

#[derive(Default)]
pub struct LocalPlateSample {
    field: ClimateField,
    region_modifier_ids: Vec<String>,
    biome_modifier_ids: Vec<String>,
    subbiome_modifier_ids: Vec<String>,
    feature_rule_ids: Vec<String>,

    height_mean: f32,
    height_variance: f32,
    roughness: f32,
    erosion_bias: f32,
    density_curve_id: Option<String>,

    weight_sum: f32,
}
impl LocalPlateSample {
    pub fn append_feature_rules(&mut self, other: &Vec<String>) {
         let mut tmp_feature_rules = other.clone();
         self.feature_rule_ids.append(&mut tmp_feature_rules);
    }
    pub fn set_region_modifier_ids(&mut self, ids: &Vec<String>) {
        self.region_modifier_ids = ids.clone();
    }
    pub fn set_biome_modifier_ids(&mut self, ids: &Vec<String>) {
        self.biome_modifier_ids = ids.clone();
    }
    pub fn set_subbiome_modifier_ids(&mut self, ids: &Vec<String>) {
        self.subbiome_modifier_ids =ids.clone();
    }
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
    pub fn add_terrain(&mut self, height_mean: f32, height_variance: f32, roughness: f32, erosion_bias: f32, weight: f32) {
        self.height_mean += height_mean;
        self.height_variance += height_variance;
        self.roughness += roughness;
        self.erosion_bias += erosion_bias;
        self.weight_sum += weight;
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
    pub fn set_field(&mut self, field: ClimateField) {
        self.field = field;
    }
    pub fn get_field(&self) -> ClimateField {
        self.field
    }
    pub fn get_region_modifier_ids(&self) -> &Vec<String> {
        &self.region_modifier_ids
    }
    pub fn get_biome_modifier_ids(&self) -> &Vec<String> {
        &self.biome_modifier_ids 
    }
    pub fn get_subbiome_modifier_ids(&self) -> &Vec<String> {
        &self.subbiome_modifier_ids
    }
    pub fn get_feature_rule_ids(&self) -> &Vec<String> {
        &self.feature_rule_ids 
    }
    
    
}

#[derive(Default)]
pub struct SharedTerrainPlate {
    region_modifier_ids: HashMap<String, TerrainModifier>,
    biome_modifier_ids: HashMap<String, TerrainModifier>,
    subbiome_modifier_ids: HashMap<String, TerrainModifier>,

    feature_rules: HashMap<String, TerrainFeature>,
    denisty_curves: HashMap<String, DensityCurve>,
}
impl SharedTerrainPlate {
    pub fn get_region_modifier(&self, id: String) -> Option<&TerrainModifier> {
        self.region_modifier_ids.get(&id)
    }
    pub fn get_biome_modifier(&self, id: String) -> Option<&TerrainModifier> {
        self.biome_modifier_ids.get(&id)
    }
    pub fn get_subbiome_modifier(&self, id: String) -> Option<&TerrainModifier> {
        self.subbiome_modifier_ids.get(&id)
    }
    pub fn get_feature_rule(&self, id: String) -> Option<&TerrainFeature> {
        self.feature_rules.get(&id)
    }
    pub fn get_density_curve(&self, id: String) -> Option<&DensityCurve> {
        self.denisty_curves.get(&id)
    }
}
#[derive(Default)]
pub struct DensityPoint {
    height: f32,
    density: f32,
}

#[derive(Default)]
pub struct DensityCurve {
    points: Vec<DensityPoint> 
}
impl DensityCurve {
    pub fn sample(&self, h: f32) -> f32 {
        // Piecewise linear interpolation
        return 0.0;
    }
}

#[derive(Default)]
pub struct TerrainModifier {}

#[derive(Default)]
pub struct TerrainFeature {
    placement_radius: f32,
    density: f32,

    min_height: f32,
    max_height: f32,
    min_slope: f32,
    max_slope: f32,
}

#[derive(Default)]
pub struct FeatureRule {
    feature_id: String,
    min_biome_weight: f32,
    min_subbiome_strength: f32,
    spawn_probability: f32,
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
    pub fn required_cells(
        &self,
        center: IVec2,
    ) -> HashSet<IVec2> {
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
        Vec2::new(
            pos.x / self.grid_size,
            pos.y / self.grid_size,
        )
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
      
    pub fn update<S: GridSampler<T>>(
        &mut self,
        sampler: &mut S,
        world_pos: Vec2,
        seed: i32,
    ) {
        let center = self.pos_to_cell(world_pos);
        let required = self.required_cells(center);

        self.grid.retain(|grid_coord, _| required.contains(grid_coord));

        for grid_coord in required {
            if !self.grid.contains_key(&grid_coord) {
                let value = sampler.sample_cell(grid_coord, seed);
                self.grid.insert(grid_coord, value);
            }
        }
    }
}