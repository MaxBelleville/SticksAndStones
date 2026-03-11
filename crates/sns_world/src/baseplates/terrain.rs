use crate::BIOME_BASE_WEIGHT;
use crate::SUBBIOME_BASE_WEIGHT;
use crate::prelude::{ClimateSample, LocalPlateSample};

use bevy_math::*;
use std::collections::HashMap;

#[derive(Default)]
pub struct Plate {
    terrain_profiles: HashMap<TerrainProfileId, TerrainProfile>,
}
impl Plate {
    pub fn blend(
        &self,
        coord: Vec2,
        seed: i32,
        climate: ClimateSample,
        noise_fields: &NoiseFields<NoiseId>,
    ) -> LocalPlateSample {
        let mut local = LocalPlateSample::default();
        local.set_field(climate.get_field());

        local.set_base_rule_ids(climate.get_base_rule_ids().clone());

        for biome_samples in climate.get_biome_samples() {
            let Some(biome_profile) = self
                .terrain_profiles
                .get(&biome_samples.get_terrain_profile_id())
            else {
                continue;
            };

            biome_profile.apply(
                coord,
                seed,
                biome_samples.get_weight() * BIOME_BASE_WEIGHT,
                &mut local,
                noise_fields,
            );
            for sub in biome_samples.get_sub_sample() {
                let Some(terrain_profile_id) = sub.get_opt_terrain_profile_id() else {
                    continue;
                };
                let subbiome_profile = self.terrain_profiles.get(terrain_profile_id).unwrap();

                subbiome_profile.apply(
                    coord,
                    seed,
                    sub.get_weight() * SUBBIOME_BASE_WEIGHT,
                    &mut local,
                    noise_fields,
                );
            }
        }

        local.normalize();

        local
    }
}

impl TerrainProfile {
    pub fn apply(
        &self,
        coord: Vec2,
        seed: i32,
        weight: f32,
        local: &mut LocalPlateSample,
        noise_fields: &NoiseFields<NoiseId>,
    ) {
        let noise = noise_fields
            .get(self.base_noise)
            .map(|n| n.sample_fbm_2d(coord, seed))
            .unwrap_or(0.0);

        // Height is stochastic
        let final_weight = weight * noise;

        let height_mean = final_weight * self.height_offset;
        let height_variance = final_weight * self.height_variance;
        let roughness = final_weight * self.roughness;
        let erosion_bias = final_weight * self.erosion_strength;

        local.insert_first_density_id(self.density_curve_id);

        local.add_terrain(
            height_mean,
            height_variance,
            roughness,
            erosion_bias,
            weight,
        );
    }
}
