use bevy_math::*;

use crate::{SUBBIOME_OFFSET_SCALE, WEIGHT_EPSILON};

use sns_assets::{
    baseplates::{Biome, BiomeID, PartialClimateField, SubBiome, SubBiomeID},
    worldshared::{BaseRuleID, NoiseField, TerrainProfileID},
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
impl WeightedSample for Sample {
    type Id = BiomeID;

    fn id(&self) -> Self::Id {
        self.id
    }
    fn weight(&self) -> f32 {
        self.weight
    }
    fn set_weight(&mut self, w: f32) {
        self.weight = w;
    }
    fn lerp(&self, other: &Self, t: f32) -> Self {
        let weight = lerp(self.weight, other.weight, t);

        let sub_sample = merge_lerp(&self.sub_sample, &other.sub_sample, t);

        Sample {
            id: self.id,
            weight: weight,
            terrain_profile_id: self.terrain_profile_id,
            base_rule_ids: self.base_rule_ids.clone(),
            sub_sample,
        }
    }
    fn scaled(&self, w: f32) -> Self {
        Sample {
            id: self.id,
            weight: w,
            terrain_profile_id: self.terrain_profile_id,
            base_rule_ids: self.base_rule_ids.clone(),
            sub_sample: self.sub_sample.clone(),
        }
    }
}

impl Sample {
    pub fn terrain_profile_id(&self) -> TerrainProfileID {
        self.terrain_profile_id
    }
    pub fn base_rule_ids(&self) -> &Vec<BaseRuleID> {
        &self.base_rule_ids
    }
    pub fn mut_rule_ids(&mut self) -> &mut Vec<BaseRuleID> {
        &mut self.base_rule_ids
    }

    pub fn sub_sample(&self) -> &Vec<SubSample> {
        &self.sub_sample
    }
    pub fn mut_sub_sample(&mut self) -> &mut Vec<SubSample> {
        &mut self.sub_sample
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

impl WeightedSample for SubSample {
    type Id = SubBiomeID;

    fn id(&self) -> Self::Id {
        self.id
    }
    fn weight(&self) -> f32 {
        self.weight
    }
    fn set_weight(&mut self, w: f32) {
        self.weight = w;
    }
    fn lerp(&self, other: &Self, t: f32) -> Self {
        let weight = lerp(self.weight, other.weight, t);

        Self {
            id: self.id,
            parent_id: self.parent_id, // assume same parent
            weight,
            terrain_profile_id: self.terrain_profile_id, // or resolve conflict
            base_rule_ids: self.base_rule_ids.clone(),
        }
    }
    fn scaled(&self, w: f32) -> Self {
        Self {
            id: self.id,
            parent_id: self.parent_id,
            weight: w,
            terrain_profile_id: self.terrain_profile_id.clone(),
            base_rule_ids: self.base_rule_ids.clone(),
        }
    }
}

impl SubSample {
    pub fn parent_id(&self) -> BiomeID {
        self.parent_id
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
}

pub trait WeightedSample: Sized + Clone {
    type Id: Ord + Copy;

    fn id(&self) -> Self::Id;
    fn weight(&self) -> f32;
    fn set_weight(&mut self, w: f32);

    fn lerp(&self, other: &Self, t: f32) -> Self;
    fn scaled(&self, w: f32) -> Self;
}

pub fn merge_lerp<T: WeightedSample>(a: &[T], b: &[T], t: f32) -> Vec<T> {
    let mut out = Vec::with_capacity(a.len() + b.len());

    let mut i = 0;
    let mut j = 0;

    while i < a.len() && j < b.len() {
        match a[i].id().cmp(&b[j].id()) {
            Ordering::Equal => {
                let item = a[i].lerp(&b[j], t);
                if item.weight() > WEIGHT_EPSILON {
                    out.push(item);
                }
                i += 1;
                j += 1;
            }
            Ordering::Less => {
                let item = a[i].scaled(a[i].weight() * (1.0 - t));
                if item.weight() > WEIGHT_EPSILON {
                    out.push(item);
                }
                i += 1;
            }
            Ordering::Greater => {
                let item = b[j].scaled(b[j].weight() * t);
                if item.weight() > WEIGHT_EPSILON {
                    out.push(item);
                }
                j += 1;
            }
        }
    }

    while i < a.len() {
        let item = a[i].scaled(a[i].weight() * (1.0 - t));
        if item.weight() > WEIGHT_EPSILON {
            out.push(item);
        }
        i += 1;
    }

    while j < b.len() {
        let item = b[j].scaled(b[j].weight() * t);
        if item.weight() > WEIGHT_EPSILON {
            out.push(item);
        }
        j += 1;
    }

    out
}
