
use crate::BIOME_BASE_WEIGHT;
use crate::SUBBIOME_BASE_WEIGHT;
use crate::prelude::{LocalPlateSample,ClimateSample};
use bevy_math::*;
use sns_core::prelude::NoiseFields;
use std::collections::HashMap;
#[derive(Default)]
pub struct TerrainPlate {
    terrain_profiles: HashMap<String, TerrainProfile>,
}
impl TerrainPlate {
    pub fn blend(
        &self,
        coord: Vec2,
        seed: i32,
        climate: ClimateSample,
        noise_fields: &NoiseFields,
    ) -> LocalPlateSample {
        let mut local = LocalPlateSample::default();
        local.set_field(climate.get_field());

        local.set_base_rule_ids(climate.get_base_rule_ids().clone());

        for active in climate.get_active_biomes() {
            let Some(biome_profile) = self.terrain_profiles.get(&active.get_terrain_profile_id())
            else {
                continue;
            };

            biome_profile.apply(coord, seed, active.get_weight() * BIOME_BASE_WEIGHT, &mut local, noise_fields);
            for sub in active.get_sub_biomes() {
                let Some(terrain_profile_id) = sub.get_opt_terrain_profile_id() else {
                    continue;
                };
                let subbiome_profile = self.terrain_profiles.get(terrain_profile_id).unwrap();

                subbiome_profile.apply(coord, seed, sub.get_weight() * SUBBIOME_BASE_WEIGHT, &mut local, noise_fields);
            }
        }

        local.normalize();

        local
    }
}

#[derive(Default)]
pub struct TerrainProfile {
    base_noise: String,
    height_offset: f32,   // mean elevation contribution
    height_variance: f32, // amplitude of deviation
    roughness: f32,
    erosion_strength: f32,
    density_curve_id: Option<String>,
}
impl TerrainProfile {
    pub fn apply(
        &self,
        coord: Vec2,
        seed: i32,
        weight: f32,
        local: &mut LocalPlateSample,
        noise_fields: &NoiseFields,
    ) {
        let noise = noise_fields
            .get(self.base_noise.clone())
            .map(|n| n.sample_fbm_2d(coord, seed))
            .unwrap_or(0.0);

        // Height is stochastic
        let final_weight = weight * noise;

        let height_mean   = final_weight * self.height_offset;
        let height_variance = final_weight * self.height_variance;
        let roughness     = final_weight * self.roughness;
        let erosion_bias  = final_weight * self.erosion_strength;

        local.insert_first_density_id(self.density_curve_id.clone());

        local.add_terrain(
            height_mean,
            height_variance,
            roughness,
            erosion_bias,
            weight,
        );
    }
}
