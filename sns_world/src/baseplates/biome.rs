use bevy_math::*;

use crate::prelude::ClimateField;

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
        climate_field: ClimateField,
        noise_fields: &NoiseFields,
        seed: i32,
    ) -> Vec<ActiveBiome> {
        // 1. Compute biome weights
        let (mut active_biomes, sum) = self.compute_weights(climate_field);
        if sum == 0.0 {
            return Vec::new();
        }

        // 2. For each biome with weight > ε
        for active_biome in &mut active_biomes {
            active_biome.weight /= sum;
            // 3. Evaluate sub-biomes
            for sub in self.sub_biomes.values() {
                if sub.get_parent_id() != active_biome.get_biome_id() {
                    continue;
                }
                let Some(noise) = noise_fields.get(sub.get_noise_id()) else {
                    continue;
                };

                if let Some(active_sub) =
                    sub.evaluate(coord, seed, active_biome.get_weight(), noise)
                {
                    active_biome.sub_biomes.push(active_sub);
                    // 4. Sub-biome active
                    // 5. Evaluate feature rules (later)
                    //    active.strength is what FeatureRules read
                }
            }
        }
        active_biomes
    }

    pub fn compute_weights(&self, climate_field: ClimateField) -> (Vec<ActiveBiome>, f32) {
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
                    biome_modifier_ids: biome.biome_modifier_ids.clone(),
                    feature_rule_ids: biome.feature_rule_ids.clone(),
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
    center: ClimateField,
    width: ClimateField,

    // What terrain this biome produces
    terrain_profile_id: String,
    biome_modifier_ids: Vec<String>,
    feature_rule_ids: Vec<String>,
}
//Add localized character to each biome so there's variation based on noise + parent weights
#[derive(Default)]
pub struct SubBiome {
    parent_id: String,

    mask_noise_id: String, // or reference/id
    threshold: f32,        // activation threshold
    strength: f32,
    terrain_profile_id: Option<String>,
    subbiome_modifier_ids: Vec<String>,
    feature_rule_ids: Vec<String>,
}

impl SubBiome {
    pub fn get_parent_id(&self) -> String {
        self.parent_id.clone()
    }
    pub fn get_noise_id(&self) -> String {
        self.mask_noise_id.clone()
    }
    pub fn _get_opt_terrain_profile_id(&self) -> &Option<String> {
        &self.terrain_profile_id
    }
    pub fn _get_subbiome_modifier_ids(&self) -> &Vec<String> {
        &self.subbiome_modifier_ids
    }
    pub fn _get_feature_rule_ids(&self) -> &Vec<String> {
        &self.feature_rule_ids
    }
    pub fn evaluate(
        &self,
        coord: Vec2,
        seed: i32,
        parent_weight: f32,
        mask_noise: &NoiseField,
    ) -> Option<ActiveSubBiome> {
        let mask = mask_noise.sample_fbm_2d(coord, seed);

        if mask < self.threshold {
            return None;
        }

        Some(ActiveSubBiome {
            biome_id: self.parent_id.clone(),
            strength: parent_weight * self.strength * mask,
            subbiome_modifier_ids: self.subbiome_modifier_ids.clone(),
            terrain_profile_id: self.terrain_profile_id.clone(),
            feature_rule_ids: self.feature_rule_ids.clone(),
        })
    }
}

#[derive(Default, Clone)]
pub struct ActiveBiome {
    biome_id: String,
    weight: f32,
    terrain_profile_id: String,
    biome_modifier_ids: Vec<String>,
    feature_rule_ids: Vec<String>,
    sub_biomes: Vec<ActiveSubBiome>,
}

impl ActiveBiome {
    pub fn get_biome_id(&self) -> String {
        self.biome_id.clone()
    }
    pub fn get_terrain_profile_id(&self) -> String {
        self.terrain_profile_id.clone()
    }
    pub fn get_biome_modifier_ids(&self) -> &Vec<String> {
        &self.biome_modifier_ids
    }
    pub fn get_rule_ids(&self) -> &Vec<String> {
        &self.feature_rule_ids
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
                let (strength, source) = match b_opt {
                    Some(b) => (lerp(a.strength, b.strength, t), a),
                    None => (a.strength * (1.0 - t), a),
                };

                (strength > 0.001).then_some(ActiveSubBiome {
                    biome_id: source.biome_id.clone(),
                    strength,
                    subbiome_modifier_ids: source.subbiome_modifier_ids.clone(),
                    feature_rule_ids: source.feature_rule_ids.clone(),
                    terrain_profile_id: source.terrain_profile_id.clone(),
                })
            })
            .collect();

        Self {
            biome_id: self.biome_id.clone(),
            terrain_profile_id: self.terrain_profile_id.clone(),
            weight: lerp(self.weight, other.weight, t),
            sub_biomes,
            biome_modifier_ids: self.biome_modifier_ids.clone(),
            feature_rule_ids: self.feature_rule_ids.clone(),
        }
    }
}

#[derive(Default, Clone)]
pub struct ActiveSubBiome {
    biome_id: String,
    strength: f32,
    terrain_profile_id: Option<String>,
    subbiome_modifier_ids: Vec<String>,
    feature_rule_ids: Vec<String>,
}
impl ActiveSubBiome {
    pub fn get_biome_id(&self) -> String {
        self.biome_id.clone()
    }

    pub fn get_strength(&self) -> f32 {
        self.strength
    }
    pub fn get_subbiome_modifier_ids(&self) -> &Vec<String> {
        &self.subbiome_modifier_ids 
    }
    pub fn get_rule_ids(&self) -> &Vec<String> {
        &self.feature_rule_ids
    }
    pub fn get_opt_terrain_profile_id(&self) -> &Option<String> {
        &self.terrain_profile_id
    }
}
