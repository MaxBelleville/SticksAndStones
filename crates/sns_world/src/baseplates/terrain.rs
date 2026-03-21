use crate::BIOME_BASE_WEIGHT;
use crate::SUBBIOME_BASE_WEIGHT;
use crate::prelude::{ClimateSample, LocalPlateSample};

use bevy_math::*;
use sns_assets::prelude::{NoiseField, NoiseFieldID, TerrainProfile, TerrainProfileID};
use sns_core::prelude::sample_fbm_2d;
use std::collections::HashMap;

#[derive(Default)]
pub struct Plate {
    terrain_profiles: HashMap<TerrainProfileID, TerrainProfile>,
}
impl Plate {
    pub fn blend(
        &self,
        coord: Vec2,
        seed: i32,
        climate: ClimateSample,
        noise_fields: &HashMap<NoiseFieldID, NoiseField>,
    ) -> LocalPlateSample {
        let mut local = LocalPlateSample::default();
        local.set_field(climate.field());

        local.set_base_rule_ids(climate.base_rule_ids().clone());

        for biome_samples in climate.biome_samples() {
            self.apply(
                &biome_samples.terrain_profile_id(),
                coord,
                seed,
                biome_samples.weight() * BIOME_BASE_WEIGHT,
                &mut local,
                noise_fields,
            );
            for sub in biome_samples.sub_sample() {
                let Some(terrain_profile_id) = sub.terrain_profile_id() else {
                    continue;
                };
       
                self.apply(
                    terrain_profile_id,
                    coord,
                    seed,
                    sub.weight() * SUBBIOME_BASE_WEIGHT,
                    &mut local,
                    noise_fields,
                );
            }
        }

        local.normalize();

        local
    }

    pub fn apply(
        &self,
        terrain_profile_id: &TerrainProfileID,
        coord: Vec2,
        seed: i32,
        weight: f32,
        local: &mut LocalPlateSample,
        noise_fields: &HashMap<NoiseFieldID, NoiseField>,
    ) {
        let Some(tp) = self.terrain_profiles.get(terrain_profile_id) else {
            return;
        };

        let noise = noise_fields
            .get(tp.base_noise())
            .map(|n| sample_fbm_2d(n,coord, seed))
            .unwrap_or(0.0);

        // Height is stochastic
        let final_weight = weight * noise;

        let height_mean = final_weight * tp.height_offset();
        let height_variance = final_weight * tp.height_variance();
        let roughness = final_weight * tp.roughness();
        let erosion_bias = final_weight * tp.erosion_strength();

        local.insert_first_density_id(tp.density_curve_id());

        local.add_terrain(
            height_mean,
            height_variance,
            roughness,
            erosion_bias,
            weight,
        );
    }
}
