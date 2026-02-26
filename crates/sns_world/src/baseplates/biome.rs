use bevy_math::*;

use crate::{
    SUBBIOME_OFFSET_SCALE, WEIGHT_EPSILON,
    prelude::PartialClimateField,
    structs::{BaseRuleId, BiomeId, SubBiomeId, TerrainProfileId},
};

use sns_core::prelude::*;
use std::{cmp::Ordering, collections::HashMap};
#[derive(Default)]
pub struct Plate {
    biomes: HashMap<BiomeId, Biome>,
    sub_biomes: HashMap<SubBiomeId, SubBiome>,
}


impl Plate {
    pub fn _get_biome(&self, id: BiomeId) -> Option<&Biome> {
        self.biomes.get(&id)
    }

    pub fn sample(
        &self,
        climate: PartialClimateField,
        coord: Vec2,
        seed: i32,
        local_noise: &NoiseField,
    ) -> Vec<Sample> {
        let mut out = Vec::new();
        let mut sum = 0.0;

        for biome in self.biomes.values() {
            let weight = climate.gaussian(biome.center, biome.width);

            if weight <= WEIGHT_EPSILON {
                continue;
            }
            sum += weight;

            let sample = Sample {
                id: biome.id,
                weight,
                sub_sample: Vec::new(),
                terrain_profile_id: biome.terrain_profile_id,
                base_rule_ids: biome.base_rule_ids.clone(),
            };
            out.push(sample);
        }
        //Normalize values and get sub biomes
        for sample in &mut out {
            sample.weight /= sum;
            let biome = self.biomes.get(&sample.id).unwrap();
            for sub_ids in &biome.sub_ids {
                let sub = self.sub_biomes.get(sub_ids).unwrap();
                if let Some(evaluated_sub) = sub.sample(coord, seed, sample.weight, local_noise) {
                    sample.sub_sample.push(evaluated_sub);
                }
            }
        }   

        out
    }
}

//Details what kinda terrain emergies under the climate conditions(will likely be a blend between mutiple biomes).
#[derive(Default)]
pub struct Biome {
    id: BiomeId,
    // Climate preference
    center: PartialClimateField,
    width: PartialClimateField,
    sub_ids: Vec<SubBiomeId>,

    // What terrain this biome produces
    terrain_profile_id: TerrainProfileId,
    base_rule_ids: Vec<BaseRuleId>,
}

//Add localized character to each biome so there's variation based on noise + parent weights
#[derive(Default)]
pub struct SubBiome {
    parent_id: BiomeId,
    id: SubBiomeId,
    threshold: f32,
    weight: f32,
    terrain_profile_id: Option<TerrainProfileId>,
    base_rule_ids: Vec<BaseRuleId>,
}

impl SubBiome {
    pub fn get_parent_id(&self) -> BiomeId {
        self.parent_id
    }

    pub fn _get_opt_terrain_profile_id(&self) -> &Option<TerrainProfileId> {
        &self.terrain_profile_id
    }

    pub fn _get_base_rule_ids(&self) -> &Vec<BaseRuleId> {
        &self.base_rule_ids
    }

    pub fn sample(
        &self,
        coord: Vec2,
        seed: i32,
        parent_weight: f32,
        local_noise: &NoiseField,
    ) -> Option<SubSample> {
        let hash = hash_id_vec2(self.id.val, seed);

        // Centered, small, frequency-aware offset
        let offset = (hash - Vec2::splat(0.5)) * SUBBIOME_OFFSET_SCALE;

        let mask = local_noise.sample_fbm_2d(coord + offset, seed);

        if mask < self.threshold {
            return None;
        }

        let weight = parent_weight * self.weight * mask;

        Some(SubSample {
            parent_id: self.parent_id,
            id: self.id,
            weight,
            terrain_profile_id: self.terrain_profile_id,
            base_rule_ids: self.base_rule_ids.clone(),
        })
    }
}

#[derive(Default, Clone)]
pub struct Sample {
    id: BiomeId,
    weight: f32,
    terrain_profile_id: TerrainProfileId,
    base_rule_ids: Vec<BaseRuleId>,
    sub_sample: Vec<SubSample>,
}

impl Sample {
    pub fn get_id(&self) -> BiomeId {
        self.id
    }
    pub fn get_terrain_profile_id(&self) -> TerrainProfileId {
        self.terrain_profile_id
    }
    pub fn get_base_rule_ids(&self) -> &Vec<BaseRuleId> {
        &self.base_rule_ids
    }
    pub fn get_mut_rule_ids(&mut self) -> &mut Vec<BaseRuleId> {
        &mut self.base_rule_ids
    }

    pub fn get_weight(&self) -> f32 {
        self.weight
    }
    pub fn get_sub_sample(&self) -> &Vec<SubSample> {
        &self.sub_sample
    }
     pub fn get_mut_sub_sample(&mut self) -> &mut Vec<SubSample> {
        &mut self.sub_sample
    }

    pub fn set_weight(&mut self, weight: f32) {
        self.weight = weight;
    }
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        let sub_sample = SubSample::lerp(&self.sub_sample, &other.sub_sample, t);
        Self {
            id: self.id,
            terrain_profile_id: self.terrain_profile_id,
            weight: lerp(self.weight, other.weight, t),
            sub_sample,
            base_rule_ids: self.base_rule_ids.clone(),
        }
    }
}

#[derive(Default, Clone)]
pub struct SubSample {
    id: SubBiomeId,
    parent_id: BiomeId,
    weight: f32,
    terrain_profile_id: Option<TerrainProfileId>,
    base_rule_ids: Vec<BaseRuleId>,
}
impl SubSample {
    pub fn get_id(&self) -> SubBiomeId {
        self.id
    }
    pub fn get_parent_id(&self) -> BiomeId {
        self.parent_id
    }

    pub fn get_weight(&self) -> f32 {
        self.weight
    }

    pub fn get_base_rule_ids(&self) -> &Vec<BaseRuleId> {
        &self.base_rule_ids
    }
      pub fn get_mut_rule_ids(&mut self) -> &mut Vec<BaseRuleId> {
        &mut self.base_rule_ids
    }
    pub fn get_opt_terrain_profile_id(&self) -> &Option<TerrainProfileId> {
        &self.terrain_profile_id
    }

    pub fn lerp(
        a: &Vec<SubSample>,
        b: &Vec<SubSample>,
        t: f32,
    ) -> Vec<SubSample> {
        let mut out = Vec::new();

        let mut i = 0;
        let mut j = 0;

        while i < a.len() && j < b.len() {
            match a[i].id.val.cmp(&b[j].id.val) {
                Ordering::Equal => {
                    let weight = lerp(a[i].weight, b[j].weight, t);
                    if weight > WEIGHT_EPSILON {
                        out.push(SubSample {
                            id: a[i].id,
                            parent_id: a[i].parent_id,
                            weight,
                            terrain_profile_id: a[i].terrain_profile_id,
                            base_rule_ids: a[i].base_rule_ids.clone(),
                        });
                    }
                    i += 1;
                    j += 1;
                }
                Ordering::Less => {
                    let weight = a[i].weight * (1.0 - t);
                    if weight > WEIGHT_EPSILON {
                        out.push(a[i].scaled(weight));
                    }
                    i += 1;
                }
                Ordering::Greater => {
                    let weight = b[j].weight * t;
                    if weight > WEIGHT_EPSILON {
                        out.push(b[j].scaled(weight));
                    }
                    j += 1;
                }
            }
        }
        out
    }

    pub fn scaled(&self, w: f32) -> SubSample {
        SubSample {
            id: self.id,
            parent_id: self.parent_id,
            weight: w,
            terrain_profile_id: self.terrain_profile_id,
            base_rule_ids: self.base_rule_ids.clone(),
        }
    }
}
