mod biome;
mod region;
mod terrain;

use super::structs::{GridSampleCache, GridSampler};
use crate::prelude::{ClimateField, NoiseIds, LocalPlateSample};
use bevy_math::*;
use sns_macros::Iterable;
use std::collections::HashMap;

use sns_core::prelude::*;

pub struct ClimatePlate {
    region_plate: region::RegionPlate,
    biome_plate: biome::BiomePlate,
    terrain_plate: terrain::TerrainPlate,
    noise_plate: NoisePlate,
    noise_fields: NoiseFields,

    climate_grid_cache: GridSampleCache<ClimateSample>,
    local_grid_cache: GridSampleCache<LocalPlateSample>,
}
impl Default for ClimatePlate {
    fn default() -> Self {
        Self {
            region_plate: region::RegionPlate::default(),
            biome_plate: biome::BiomePlate::default(),
            terrain_plate: terrain::TerrainPlate::default(),
            noise_plate: NoisePlate::default(),
            noise_fields: NoiseFields::default(),
           
            climate_grid_cache: GridSampleCache::new(2, 512.0),
            local_grid_cache: GridSampleCache::new(64, 4.0),
        }
    }
}

impl ClimatePlate {
    pub fn sample_base_noise(&self, coord: Vec2, seed: i32) -> ClimateField {
        let mut field = ClimateField::default();

        let noise_part_iter = self.noise_plate.iter();
        let mut noise_sums: HashMap<String, f32> = HashMap::new();

        for (_name, noise_ids_any) in noise_part_iter {
            //Loop through each noise part
            if let Some(noise_ids) = noise_ids_any.downcast_ref::<NoiseIds>() {
                //Loop through each noise id in the noise part
                for (noise_name, noise_id) in noise_ids.iter() {
                    //Get noise field from id
                    let Some(noise) = self.noise_fields.get(noise_id.to_string())
                    else {
                        return field;
                    };
                    //add sample_fbm to a hashmap of noise_name(ex temp,roughness): sum
                    let noise_val = noise.sample_fbm_2d(coord, seed);
                    if let Some(sum_val) = noise_sums.get_mut(noise_name) {
                        *sum_val += noise_val;
                    } else {
                        noise_sums.insert(noise_name.to_string(), noise_val);
                    }
                }
            }
        }

        let field_iter = field.iter_mut_val();
        //zip field_iter with noise_sums
        for ((name, sum_val), field_value) in std::iter::zip(noise_sums.iter(), field_iter) {
            let mut final_sum = sum_val.clone();
            final_sum = final_sum.clamp(0.0, 1.0);

            // Optional per-field post-processing
            if name == "ruggedness" {
                final_sum = final_sum.abs();
            }

            if let Some(v) = field_value.downcast_mut::<f32>() {
                *v = final_sum;
            }
        }
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

        field_data = field_data + archetype.get_bias();

        let active_biomes =
            self.biome_plate
                .evaluate_biomes(world_pos, field_data, &self.noise_fields, seed);

        ClimateSample {
            field: field_data,
            active_biomes,
            archetype_id: region_sample.get_archetype_id(),
        }
    }
}

impl GridSampler<LocalPlateSample> for ClimatePlate {
    fn sample_cell(&mut self, grid_coord: IVec2, seed: i32) -> LocalPlateSample {
        //get world pos + jitter from grid_coord
        let base_pos = grid_coord.as_vec2() * self.local_grid_cache.get_grid_size();

        let jitter = (hash_vec2(grid_coord, seed) - 0.5) * self.local_grid_cache.get_grid_size();
        let world_pos = base_pos + jitter;

        let final_interp = self.interpolation(world_pos);
        let archetype = self
            .region_plate
            .get_archetype(final_interp.archetype_id.clone())
            .expect("Missing archetype");

        //Do other stuff
        self.terrain_plate.blend(world_pos,seed,final_interp, archetype,&self.noise_fields)
        
    }
}

#[derive(Default, Iterable)]
#[iterable(std::any::Any)]
pub struct NoisePlate {
    climate: NoiseIds,
    region: NoiseIds,
    local: NoiseIds,
}

#[derive(Default,Clone)]
pub struct ClimateSample {
    field: ClimateField,
    active_biomes: Vec<biome::ActiveBiome>,
    archetype_id: String,
}

impl ClimateSample {
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        let field = self.field.lerp(&other.field, t);
        let curr_active_biomes = self.lerp_biomes(&other.active_biomes, t);
        Self {
            field,
            active_biomes: curr_active_biomes,
            archetype_id: self.archetype_id.clone(),
        }
    }

    pub fn lerp_biomes(&self, other: &Vec<biome::ActiveBiome>, t: f32) -> Vec<biome::ActiveBiome> {
        //as other and self.active_biomes lists might be different sizes we have do a hashmap

        let mut map = HashMap::new();

        for biome in self.active_biomes.iter() {
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


