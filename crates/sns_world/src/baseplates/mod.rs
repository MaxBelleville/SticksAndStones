pub mod biome;
mod region;
mod terrain;

use std::collections::HashMap;

use crate::{
    prelude::{BaseRules, ClimateSample, GridSampleCache, GridSampler},
    structs::{LocalSample, NoiseBasisPlate},
};

use bevy_math::*;

use sns_assets::{
    baseplates::PartialClimateField,
    worldshared::{ClimateField, ClimateFieldID, NoiseField, NoiseFieldID},
};
use sns_core::prelude::*;

/*
    What is a base plate?
    The foundation of the world generation, which everything else stems off of.
    Using biome, sub biome, region and climate base plates we can sample a large area around the camera
    Which will be used to determine the approximate world geometry, as well as determine structures/features.
    Base plates are completely 2D, and using an ambassador link up to 3d brick hub/brick map terrain.
*/

pub struct WorldPlate {
    climate_plate: ClimatePlate,
    local_plate: LocalPlate,
    terrain_plate: terrain::Plate,
    noise_fields: HashMap<NoiseFieldID, NoiseField>,
    climate_fields: HashMap<ClimateFieldID, ClimateField>,
    base_rules: BaseRules,
}
impl Default for WorldPlate {
    fn default() -> Self {
        Self {
            climate_plate: ClimatePlate::default(),
            local_plate: LocalPlate::default(),
            terrain_plate: terrain::Plate::default(),
            noise_fields: HashMap::new(),
            climate_fields: HashMap::new(),
            base_rules: BaseRules::default(),
        }
    }
}

impl WorldPlate {
    pub fn climate_update(&mut self, world_pos: Vec2, seed: i32) {
        self.climate_plate.climate_update(
            world_pos,
            seed,
            &self.climate_fields,
            &self.noise_fields,
            &self.base_rules,
        );
    }

    pub fn local_update(&mut self, world_pos: Vec2, seed: i32) {
        self.local_plate.local_update(
            world_pos,
            seed,
            &self.climate_plate.climate_cache,
            &self.terrain_plate,
            &self.base_rules,
            &self.noise_fields,
        );
    }
}

pub struct LocalPlate {
    local_cache: GridSampleCache<LocalSample>,
}
impl Default for LocalPlate {
    fn default() -> Self {
        Self {
            local_cache: GridSampleCache::new(128, 3.0),
        }
    }
}

impl LocalPlate {
    pub fn local_update(
        &mut self,
        world_pos: Vec2,
        seed: i32,
        climate_cache: &GridSampleCache<ClimateSample>,
        terrain_plate: &terrain::Plate,
        base_rules: &BaseRules,
        noise_fields: &HashMap<NoiseFieldID, NoiseField>,
    ) {
        let mut sampler = LocalPlateSampler {
            climate_cache,
            terrain_plate,
            base_rules,
            noise_fields,
            grid_size: self.local_cache.grid_size(),
        };

        self.local_cache.update(&mut sampler, world_pos, seed);
    }
}

pub struct ClimatePlate {
    pipeline: ClimatePipeline,
    climate_cache: GridSampleCache<ClimateSample>,
}
impl Default for ClimatePlate {
    fn default() -> Self {
        Self {
            pipeline: ClimatePipeline::default(),
            climate_cache: GridSampleCache::new(6, 512.0),
        }
    }
}

impl ClimatePlate {
    pub fn climate_update(
        &mut self,
        world_pos: Vec2,
        seed: i32,
        climate_fields: &HashMap<ClimateFieldID, ClimateField>,
        noise_fields: &HashMap<NoiseFieldID, NoiseField>,
        base_rules: &BaseRules,
    ) {
        let mut sampler = ClimateGridSampler {
            pipeline: &self.pipeline,
            grid_size: self.climate_cache.grid_size(),
            climate_fields,
            noise_fields,
            base_rules,
        };

        self.climate_cache.update(&mut sampler, world_pos, seed);
    }
}
#[derive(Default)]
pub struct ClimatePipeline {
    field_gen: FieldGenerator,
    region_plate: region::Plate,
    biome_plate: biome::Plate,
}

impl ClimatePipeline {
    pub fn sample(
        &self,
        coord: Vec2,
        seed: i32,
        climate_fields: &HashMap<ClimateFieldID, ClimateField>,
        noise_fields: &HashMap<NoiseFieldID, NoiseField>,
    ) -> ClimateSample {
        let field = self
            .field_gen
            .sample(coord, seed, climate_fields, noise_fields);
        let region_sample = self.region_plate.sample(coord, seed);
        let region_id = region_sample.get_archetype_id();
        let region = self.region_plate.get_archetype(region_id).unwrap();

        let mut biased_field = field;
        let region_bias = climate_fields.get(region.bias_id()).unwrap();

        biased_field = biased_field + region_bias.clone();

        let local_id = self.field_gen.local_id();
        let local = noise_fields.get(local_id).unwrap();

        let biomes =
            self.biome_plate
                .sample(PartialClimateField::from(biased_field), coord, seed, local);
        ClimateSample::new(biased_field, biomes, region_id, Vec::new())
    }
}
#[derive(Default)]
pub struct FieldGenerator {
    noise_plate: NoiseBasisPlate,
}

impl FieldGenerator {
    pub fn sample(
        &self,
        coord: Vec2,
        seed: i32,
        climate_fields: &HashMap<ClimateFieldID, ClimateField>,
        noise_fields: &HashMap<NoiseFieldID, NoiseField>,
    ) -> ClimateField {
        let mut field = ClimateField::default();

        let climate = noise_fields
            .get(self.noise_plate.climate_id())
            .expect("climate noise not found");
        let mut noise_val = sample_fbm_2d(&climate, coord, seed);
        let climate_weight = climate_fields
            .get(self.noise_plate.climate_weight_id())
            .unwrap();

        field = field + (climate_weight.clone() * noise_val);

        let region = noise_fields
            .get(self.noise_plate.region_id())
            .expect("region noise not found");
        noise_val = sample_fbm_2d(region, coord, seed);
        let region_weight = climate_fields
            .get(self.noise_plate.region_weight_id())
            .unwrap();

        field = field + (region_weight.clone() * noise_val);

        let local = noise_fields
            .get(self.noise_plate.local_id())
            .expect("local noise not found");
        noise_val = sample_fbm_2d(local, coord, seed);
        let local_weight = climate_fields
            .get(self.noise_plate.local_weight_id())
            .unwrap();

        field = field + (local_weight.clone() * noise_val);

        field.abs_rugedness();

        field
    }
    pub fn local_id(&self) -> &NoiseFieldID {
        self.noise_plate.local_id()
    }
}

pub struct ClimateGridSampler<'a> {
    pipeline: &'a ClimatePipeline,
    climate_fields: &'a HashMap<ClimateFieldID, ClimateField>,
    noise_fields: &'a HashMap<NoiseFieldID, NoiseField>,
    base_rules: &'a BaseRules,
    grid_size: f32,
}

impl<'a> GridSampler<ClimateSample> for ClimateGridSampler<'a> {
    fn sample_cell(&mut self, grid_coord: IVec2, seed: i32) -> ClimateSample {
        let world_pos = Vec2::new(
            grid_coord.x as f32 * self.grid_size,
            grid_coord.y as f32 * self.grid_size,
        );

        let mut samples =
            self.pipeline
                .sample(world_pos, seed, self.climate_fields, self.noise_fields);
        self.base_rules.apply_rules(samples.mut_biome_samples());
        samples
    }
}
pub struct LocalPlateSampler<'a> {
    climate_cache: &'a GridSampleCache<ClimateSample>,
    terrain_plate: &'a terrain::Plate,
    base_rules: &'a BaseRules,
    noise_fields: &'a HashMap<NoiseFieldID, NoiseField>,
    grid_size: f32,
}
impl<'a> LocalPlateSampler<'a> {
    fn interpolate(&self, world_pos: Vec2) -> ClimateSample {
        self.climate_cache
            .interpolate(world_pos, ClimateSample::lerp)
    }
}

impl<'a> GridSampler<LocalSample> for LocalPlateSampler<'a> {
    fn sample_cell(&mut self, grid_coord: IVec2, seed: i32) -> LocalSample {
        let base_pos = grid_coord.as_vec2() * self.grid_size;

        let jitter = (hash_vec2(grid_coord, seed) - 0.5) * self.grid_size;

        let world_pos = base_pos + jitter;

        // interpolate ONLY from already-updated climate cache
        let mut climate = self.interpolate(world_pos);
        self.base_rules.apply_rules(climate.mut_biome_samples());

        self.terrain_plate
            .blend(world_pos, seed, climate, self.noise_fields)
    }
}

pub trait ClimateInterpolator {
    fn lerp(&self, other: &Self, t: f32) -> Self;
}

impl ClimateInterpolator for ClimateSample {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        let field = self.field().lerp(&other.field(), t);

        let biome_samples = biome::merge_lerp(self.biome_samples(), other.biome_samples(), t);

        Self::new(
            field,
            biome_samples,
            self.archetype_id(), // or resolve if needed
            self.base_rule_ids().clone(),
        )
    }
}
