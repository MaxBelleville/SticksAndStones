use bevy_math::*;

use crate::{SUBBIOME_OFFSET_SCALE, WEIGHT_EPSILON};

use sns_assets::prelude::{
    BaseRuleID, Biome, BiomeID, NoiseField, PartialClimateField, SubBiome, SubBiomeID,
    TerrainProfileID,
};
use sns_core::prelude::*;
use std::{cmp::Ordering, collections::HashMap};

#[derive(Default)]
pub struct Plate {
    biomes: HashMap<BiomeID, Biome>,
    sub_biomes: HashMap<SubBiomeID, SubBiome>,
}

impl Plate {
    pub fn sample(
        &self,
        climate: PartialClimateField,
        coord: Vec2,
        seed: i32,
        local_noise: &NoiseField,
    ) -> Vec<Sample> {
        let mut out = Vec::new();
        let mut sum = 0.0;

        for (id, biome) in &self.biomes {
            let weight = climate.gaussian(biome.center(), biome.width());

            if weight <= WEIGHT_EPSILON {
                continue;
            }
            sum += weight;

            let sample = Sample {
                id: id.clone(),
                weight,
                sub_sample: Vec::new(),
                terrain_profile_id: biome.terrain_profile_id().clone(),
                base_rule_ids: biome.base_rule_ids().clone(),
            };
            out.push(sample);
        }
        //Normalize values and get sub biomes
        for sample in &mut out {
            sample.weight /= sum;
            let biome = self.biomes.get(&sample.id).unwrap();
            for sub_ids in biome.sub_ids() {
                if let Some(evaluated_sub) =
                    self.sub_sample(sub_ids, coord, seed, sample.weight, local_noise)
                {
                    sample.sub_sample.push(evaluated_sub);
                }
            }
        }

        out
    }

    pub fn sub_sample(
        &self,
        sub_id: &SubBiomeID,
        coord: Vec2,
        seed: i32,
        parent_weight: f32,
        local_noise: &NoiseField,
    ) -> Option<SubSample> {
        let sub = self.sub_biomes.get(sub_id).unwrap();
        let hash = hash_id_vec2(sub_id.0, seed);

        // Centered, small, frequency-aware offset
        let offset = (hash - Vec2::splat(0.5)) * SUBBIOME_OFFSET_SCALE;

        let mask = sample_fbm_2d(local_noise, coord + offset, seed);

        if mask < sub.threshold() {
            return None;
        }

        let weight = parent_weight * sub.weight() * mask;

        Some(SubSample {
            parent_id: sub.parent_id().clone(),
            id: sub_id.clone(),
            weight,
            terrain_profile_id: sub.terrain_profile_id().clone(),
            base_rule_ids: sub.base_rule_ids().clone(),
        })
    }
}

#[derive(Default, Clone)]
pub struct Sample {
    id: BiomeID,
    weight: f32,
    terrain_profile_id: TerrainProfileID,
    base_rule_ids: Vec<BaseRuleID>,
    sub_sample: Vec<SubSample>,
}

impl Sample {
    pub fn id(&self) -> BiomeID {
        self.id
    }
    pub fn terrain_profile_id(&self) -> TerrainProfileID {
        self.terrain_profile_id
    }
    pub fn base_rule_ids(&self) -> &Vec<BaseRuleID> {
        &self.base_rule_ids
    }
    pub fn mut_rule_ids(&mut self) -> &mut Vec<BaseRuleID> {
        &mut self.base_rule_ids
    }

    pub fn weight(&self) -> f32 {
        self.weight
    }
    pub fn sub_sample(&self) -> &Vec<SubSample> {
        &self.sub_sample
    }
    pub fn mut_sub_sample(&mut self) -> &mut Vec<SubSample> {
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
    id: SubBiomeID,
    parent_id: BiomeID,
    weight: f32,
    terrain_profile_id: Option<TerrainProfileID>,
    base_rule_ids: Vec<BaseRuleID>,
}
impl SubSample {
    pub fn id(&self) -> SubBiomeID {
        self.id
    }
    pub fn parent_id(&self) -> BiomeID {
        self.parent_id
    }

    pub fn weight(&self) -> f32 {
        self.weight
    }

    pub fn base_rule_ids(&self) -> &Vec<BaseRuleID> {
        &self.base_rule_ids
    }
    pub fn mut_rule_ids(&mut self) -> &mut Vec<BaseRuleID> {
        &mut self.base_rule_ids
    }
    pub fn terrain_profile_id(&self) -> &Option<TerrainProfileID> {
        &self.terrain_profile_id
    }

    pub fn lerp(a: &Vec<SubSample>, b: &Vec<SubSample>, t: f32) -> Vec<SubSample> {
        let mut out = Vec::new();

        let mut i = 0;
        let mut j = 0;

        while i < a.len() && j < b.len() {
            match a[i].id.0.cmp(&b[j].id.0) {
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
        // Remaining elements in a
        while i < a.len() {
            let weight = a[i].weight * (1.0 - t);
            if weight > WEIGHT_EPSILON {
                out.push(a[i].scaled(weight));
            }
            i += 1;
        }

        // Remaining elements in b
        while j < b.len() {
            let weight = b[j].weight * t;
            if weight > WEIGHT_EPSILON {
                out.push(b[j].scaled(weight));
            }
            j += 1;
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
