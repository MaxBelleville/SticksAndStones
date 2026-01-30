use bevy_math::*;

use crate::{
    SUBBIOME_OFFSET_SCALE,
    prelude::{BaseRules, PartialClimateField},
};

use sns_core::prelude::*;
use std::collections::HashMap;

pub struct BiomePlate {
    biomes: HashMap<String, Biome>,
    sub_biomes: HashMap<String, SubBiome>,
    weight_epsilon: f32,
}
impl Default for BiomePlate {
    fn default() -> Self {
        Self {
            biomes: HashMap::new(),
            sub_biomes: HashMap::new(),
            weight_epsilon: 0.01,
        }
    }
}

impl BiomePlate {
    pub fn _get_biome(&self, id: String) -> Option<&Biome> {
        self.biomes.get(&id)
    }
    pub fn _get_sub_biome(&self, id: String) -> Option<&SubBiome> {
        self.sub_biomes.get(&id)
    }

    pub fn evaluate_biomes(
        &self,
        coord: Vec2,
        seed: i32,
        climate_field: PartialClimateField,
        base_rules: &BaseRules,
        local_noise: &NoiseField,
    ) -> Vec<ActiveBiome> {
        // 1. Compute biome weights
        let (mut active_biomes, sum) = self.compute_weights(climate_field);
        if sum == 0.0 {
            return Vec::new();
        }

        // 2. For each biome with weight > ε
        for active_biome in &mut active_biomes {
            active_biome.weight /= sum;

            let cloned_rule_ids = active_biome.base_rule_ids.clone();
            active_biome.base_rule_ids.clear();
            //Evaluate biome rules
            for rule_id in &cloned_rule_ids {
                if let Some(rule) = base_rules.get_rule(rule_id.clone()) {
                    if active_biome.weight >= rule.get_min_weight() {
                        active_biome.base_rule_ids.push(rule.get_rule_id());
                    }
                }
            }

            // 3. Evaluate sub-biomes
            for sub in self.sub_biomes.values() {
                if sub.get_parent_id() != active_biome.get_biome_id() {
                    continue;
                }
                let active_sub_wrapped = sub.evaluate(
                    coord,
                    seed,
                    active_biome.get_weight(),
                    base_rules,
                    local_noise,
                );

                if let Some(active_sub) = active_sub_wrapped {
                    active_biome.sub_biomes.push(active_sub);
                }
            }
        }
        active_biomes
    }

    pub fn compute_weights(&self, climate_field: PartialClimateField) -> (Vec<ActiveBiome>, f32) {
        let mut out = Vec::new();
        let mut sum = 0.0;

        for (id, biome) in &self.biomes {
            let w = climate_field.gaussian(biome.center, biome.width);
            if w >= self.weight_epsilon {
                sum += w;
                out.push(ActiveBiome {
                    biome_id: id.clone(),
                    weight: w,
                    sub_biomes: Vec::new(),
                    terrain_profile_id: biome.terrain_profile_id.clone(),
                    base_rule_ids: biome.base_rule_ids.clone(),
                });
            }
        }
        (out, sum)
    }
}

//Details what kinda terrain emergies under the climate conditions(will likely be a blend between mutiple biomes).
#[derive(Default)]
pub struct Biome {
    // Climate preference
    center: PartialClimateField,
    width: PartialClimateField,

    // What terrain this biome produces
    terrain_profile_id: String,
    base_rule_ids: Vec<String>,
}
//Add localized character to each biome so there's variation based on noise + parent weights
#[derive(Default)]
pub struct SubBiome {
    parent_id: String,
    subbiome_id: String,
    threshold: f32, // activation threshold
    weight: f32,
    terrain_profile_id: Option<String>,
    base_rule_ids: Vec<String>,
}

impl SubBiome {
    pub fn get_parent_id(&self) -> String {
        self.parent_id.clone()
    }

    pub fn _get_opt_terrain_profile_id(&self) -> &Option<String> {
        &self.terrain_profile_id
    }

    pub fn _get_base_rule_ids(&self) -> &Vec<String> {
        &self.base_rule_ids
    }

    pub fn evaluate(
        &self,
        coord: Vec2,
        seed: i32,
        parent_weight: f32,
        base_rules: &BaseRules,
        local_noise: &NoiseField,
    ) -> Option<ActiveSubBiome> {
        let hash = hash_str_vec2(self.subbiome_id.as_str(), seed);

        // Centered, small, frequency-aware offset
        let offset = (hash - Vec2::splat(0.5)) * SUBBIOME_OFFSET_SCALE;

        let mask = local_noise.sample_fbm_2d(coord + offset, seed);

        if mask < self.threshold {
            return None;
        }

        let mut base_rule_ids = Vec::new();
        let weight = parent_weight * self.weight * mask;

        for rule_id in &self.base_rule_ids {
            if let Some(rule) = base_rules.get_rule(rule_id.clone()) {
                if weight >= rule.get_min_weight() {
                    base_rule_ids.push(rule.get_rule_id());
                }
            }
        }

        Some(ActiveSubBiome {
            biome_id: self.parent_id.clone(),
            weight,
            terrain_profile_id: self.terrain_profile_id.clone(),
            base_rule_ids,
        })
    }
}

#[derive(Default, Clone)]
pub struct ActiveBiome {
    biome_id: String,
    weight: f32,
    terrain_profile_id: String,
    base_rule_ids: Vec<String>,
    sub_biomes: Vec<ActiveSubBiome>,
}

impl ActiveBiome {
    pub fn get_biome_id(&self) -> String {
        self.biome_id.clone()
    }
    pub fn get_terrain_profile_id(&self) -> String {
        self.terrain_profile_id.clone()
    }
    pub fn get_base_rule_ids(&self) -> &Vec<String> {
        &self.base_rule_ids
    }

    pub fn get_weight(&self) -> f32 {
        self.weight
    }
    pub fn get_sub_biomes(&self) -> &Vec<ActiveSubBiome> {
        &self.sub_biomes
    }
    pub fn set_weight(&mut self, weight: f32) {
        self.weight = weight;
    }
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        let mut subs: HashMap<String, (&ActiveSubBiome, Option<&ActiveSubBiome>)> = HashMap::new();

        for s in &self.sub_biomes {
            subs.insert(s.biome_id.clone(), (s, None));
        }

        for s in &other.sub_biomes {
            subs.entry(s.biome_id.clone())
                .and_modify(|v| v.1 = Some(s))
                .or_insert((s, None));
        }

        let sub_biomes = subs
            .into_iter()
            .filter_map(|(_, (a, b_opt))| {
                let (weight, source) = match b_opt {
                    Some(b) => (lerp(a.weight, b.weight, t), a),
                    None => (a.weight * (1.0 - t), a),
                };

                (weight > 0.001).then_some(ActiveSubBiome {
                    biome_id: source.biome_id.clone(),
                    weight,
                    base_rule_ids: source.base_rule_ids.clone(),
                    terrain_profile_id: source.terrain_profile_id.clone(),
                })
            })
            .collect();

        Self {
            biome_id: self.biome_id.clone(),
            terrain_profile_id: self.terrain_profile_id.clone(),
            weight: lerp(self.weight, other.weight, t),
            sub_biomes,
            base_rule_ids: self.base_rule_ids.clone(),
        }
    }

    pub fn recompute_rules(&mut self, base_rules: &BaseRules) {
        let cloned_rules = self.base_rule_ids.clone();

        self.base_rule_ids.clear();
        for rule_id in cloned_rules {
            if let Some(rule) = base_rules.get_rule(rule_id.clone()) {
                if self.weight >= rule.get_min_weight() {
                    self.base_rule_ids.push(rule.get_rule_id());
                }
            }
        }

        for sub in &mut self.sub_biomes {
            let cloned_rules = self.base_rule_ids.clone();
            sub.base_rule_ids.clear();
            for rule_id in cloned_rules {
                if let Some(rule) = base_rules.get_rule(rule_id.clone()) {
                    if sub.weight >= rule.get_min_weight() {
                        sub.base_rule_ids.push(rule.get_rule_id());
                    }
                }
            }
        }
    }
}

#[derive(Default, Clone)]
pub struct ActiveSubBiome {
    biome_id: String,
    weight: f32,
    terrain_profile_id: Option<String>,
    base_rule_ids: Vec<String>,
}
impl ActiveSubBiome {
    pub fn get_biome_id(&self) -> String {
        self.biome_id.clone()
    }

    pub fn get_weight(&self) -> f32 {
        self.weight
    }

    pub fn get_base_rule_ids(&self) -> &Vec<String> {
        &self.base_rule_ids
    }
    pub fn get_opt_terrain_profile_id(&self) -> &Option<String> {
        &self.terrain_profile_id
    }
}
