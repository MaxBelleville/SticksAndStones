pub mod biome;
mod region;
mod terrain;

use crate::prelude::{
    BaseRules, ClimateField, ClimateSample, GridSampleCache, GridSampler, LocalPlateSample,
    NoiseBasis,
};
use bevy_math::*;
use sns_macros::Iterable;
use std::collections::HashMap;

use sns_core::prelude::*;

/*
    What is a baseplate?
    The foundation of the world generation, which everything else stems off of.
    Using biome, subbiome, region and climate baseplates we can sample a large area around the camera
    Which will be used to determine the aproximate world geometry, as well as determine structures/features.
    Baseplates are completely 2D, and using an ambassor link up to 3d brickhub/brick map terrain.
*/

pub struct ClimatePlate {
    region_plate: region::RegionPlate,
    biome_plate: biome::BiomePlate,
    terrain_plate: terrain::TerrainPlate,
    noise_plate: NoiseBasisPlate,
    noise_fields: NoiseFields,

    base_rules: BaseRules,
    climate_grid_cache: GridSampleCache<ClimateSample>,
    local_grid_cache: GridSampleCache<LocalPlateSample>,
}
impl Default for ClimatePlate {
    fn default() -> Self {
        Self {
            region_plate: region::RegionPlate::default(),
            biome_plate: biome::BiomePlate::default(),
            terrain_plate: terrain::TerrainPlate::default(),
            noise_plate: NoiseBasisPlate::default(),
            noise_fields: NoiseFields::default(),

            base_rules: BaseRules::default(),
            climate_grid_cache: GridSampleCache::new(6, 512.0),
            local_grid_cache: GridSampleCache::new(128, 3.0),
        }
    }
}

impl ClimatePlate {
    pub fn sample_base_noise(&self, coord: Vec2, seed: i32) -> ClimateField {
        let mut field = ClimateField::default();

        let noise_part_iter = self.noise_plate.iter();

        for (_name, noise_basis_any) in noise_part_iter {
            //Loop through each noise part
            if let Some(noise_basis) = noise_basis_any.downcast_ref::<NoiseBasis>() {
                //Get noise field from basis
                let Some(noise) = self.noise_fields.get(noise_basis.get_noise_id()) else {
                    return field;
                };
                //Get noise value from noise field and mutiply weight
                let noise_val = noise.sample_fbm_2d(coord, seed);
                field = field + (noise_basis.get_weight() * noise_val);
            }
        }
        field.abs_rugedness();
        field
    }
    pub fn interpolation(&self, coord: Vec2) -> ClimateSample {
        //Covert to climate grid coords and shift to be in the middle mutiple cells.
        let climate_grid_coord = self.climate_grid_cache.pos_reduce_to_cell(coord) - 0.5;
        let val0 = climate_grid_coord.floor().as_ivec2();
        let val1 = val0 + 1;
        let tval = climate_grid_coord - val0.as_vec2();

        let cell00 = self.climate_grid_cache.get_cell(val0);
        let cell01 = self.climate_grid_cache.get_cell(IVec2::new(val0.x, val1.y));
        let cell10 = self.climate_grid_cache.get_cell(IVec2::new(val1.x, val0.y));
        let cell11 = self.climate_grid_cache.get_cell(val1);

        let interp_cell0 = cell00.lerp(cell10, tval.x);
        let interp_cell1 = cell01.lerp(cell11, tval.x);
        interp_cell0.lerp(&interp_cell1, tval.y)
    }
}

impl GridSampler<ClimateSample> for ClimatePlate {
    fn sample_cell(&mut self, grid_coord: IVec2, seed: i32) -> ClimateSample {
        let world_pos = grid_coord.as_vec2() * self.climate_grid_cache.get_grid_size();

        let mut field_data = self.sample_base_noise(world_pos, seed);

        let region_sample = self.region_plate.sample_region(world_pos, seed);
        let archetype = self
            .region_plate
            .get_archetype(region_sample.get_archetype_id())
            .expect("Missing archetype");

        let mut bias = archetype.get_bias() * region_sample.get_edge_factor();

        if let Some(max_delta) = archetype.get_max_climate_delta() {
            bias = bias.clamp(max_delta);
        }

        field_data = field_data + bias;

        let local_noise_id = self.noise_plate.local.get_noise_id();
        let local_noise = self.noise_fields.get(local_noise_id).unwrap();

        let active_biomes = self.biome_plate.evaluate_biomes(
            world_pos,
            seed,
            field_data.to_partial(),
            &self.base_rules,
            local_noise,
        );

        let mut base_rule_ids = archetype.get_base_rules().clone();

        for biome in active_biomes.iter() {
            base_rule_ids.extend(biome.get_base_rule_ids().iter().cloned());
            for sub in biome.get_sub_biomes() {
                base_rule_ids.extend(sub.get_base_rule_ids().iter().cloned());
            }
        }

        ClimateSample::new(
            field_data,
            active_biomes,
            region_sample.get_archetype_id(),
            base_rule_ids,
        )
    }
}

impl GridSampler<LocalPlateSample> for ClimatePlate {
    fn sample_cell(&mut self, grid_coord: IVec2, seed: i32) -> LocalPlateSample {
        //get world pos + jitter from grid_coord
        let base_pos = grid_coord.as_vec2() * self.local_grid_cache.get_grid_size();

        let jitter = (hash_vec2(grid_coord, seed) - 0.5) * self.local_grid_cache.get_grid_size();
        let world_pos = base_pos + jitter;

        let mut final_interp = self.interpolation(world_pos);
        final_interp.recalculate_rules(&self.base_rules);

        //Do other stuff
        self.terrain_plate
            .blend(world_pos, seed, final_interp, &self.noise_fields)
    }
}

#[derive(Default, Iterable)]
#[iterable(std::any::Any)]
pub struct NoiseBasisPlate {
    climate: NoiseBasis,
    region: NoiseBasis,
    local: NoiseBasis,
}

pub trait ClimateInterpolator {
    fn lerp(&self, other: &Self, t: f32) -> Self;
    fn lerp_biomes(&self, other: &Vec<biome::ActiveBiome>, t: f32) -> Vec<biome::ActiveBiome>;
}

impl ClimateInterpolator for ClimateSample {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        let field = self.get_field().lerp(&other.get_field(), t);
        let curr_active_biomes = self.lerp_biomes(other.get_active_biomes(), t);
        Self::new(
            field,
            curr_active_biomes,
            self.get_archetype_id(),
            self.get_base_rule_ids().clone(),
        )
    }

    fn lerp_biomes(&self, other: &Vec<biome::ActiveBiome>, t: f32) -> Vec<biome::ActiveBiome> {
        //as other and self.active_biomes lists might be different sizes we have do a hashmap

        let mut map = HashMap::new();

        for biome in self.get_active_biomes().iter() {
            map.insert(biome.get_biome_id(), (Some(biome), None));
        }

        for biome in other.iter() {
            map.entry(biome.get_biome_id())
                .and_modify(|v| v.1 = Some(biome))
                .or_insert((None, Some(biome)));
        }

        map.into_iter()
            .filter_map(|(_, (a_opt, b_opt))| {
                let biome = match (a_opt, b_opt) {
                    (Some(a), Some(b)) => a.lerp(&b, t),
                    (Some(a), None) => {
                        let mut faded = a.clone();
                        faded.set_weight(faded.get_weight() * (1.0 - t));
                        faded
                    }
                    (None, Some(b)) => {
                        let mut faded = b.clone();
                        faded.set_weight(faded.get_weight() * (1.0 - t));
                        faded
                    }
                    (None, None) => unreachable!(),
                };

                (biome.get_weight() > 0.001).then_some(biome)
            })
            .collect()
    }
}
