use core::f32;
use std::collections::HashMap;

use sns_core::prelude::*;
use crate::prelude::ClimateField;
use bevy_math::*;

pub type RegionCoord = IVec2;
pub struct RegionPlate {
    //Node id: node
    archetypes: HashMap<String, RegionArchetype>,
    archetypes_ids: Vec<String>, //Collect and sort keys when load in archetypes from file.
    region_size: f32,
}
impl Default for RegionPlate {
    fn default() -> Self { 
        Self {
            archetypes: HashMap::new(),
            archetypes_ids: Vec::new(),
            region_size: 1024.0
        }
    }
}

impl RegionPlate {
    pub fn get_archetype(&self, id: String) -> Option<&RegionArchetype> {
        self.archetypes.get(id.as_str())
    }

    pub fn sample_region(&self, coord: Vec2, seed: i32)-> RegionSample {
        let v = self.voronoi_query(coord, seed);
        let archetype_id = self.get_archetype_id(v.closest, seed);

        let edge_factor = ((v.d2.sqrt() - v.d1.sqrt()) / self.region_size)
            .clamp(0.0, 1.0);

        RegionSample {
            coord: v.closest,
            archetype_id,
            edge_factor
        }
    }

    pub fn voronoi_query(&self, pos: Vec2, seed: i32) -> VoronoiResult {
        let base = self.pos_to_region_cell(pos);

        let mut closest = base;
        let mut d1 = f32::INFINITY;
        let mut d2 = f32::INFINITY;

        for dy in -1..=1 {
            for dx in -1..=1 {
                let c = base + RegionCoord::new(dx, dy);
                let jitter_center = hash_vec2(c, seed) * self.region_size;
                let d = jitter_center.distance_squared(pos);

                if d < d1 {
                    d2 = d1;
                    d1 = d;
                    closest = c;
                } else if d < d2 {
                    d2 = d;
                }
            }
        }

        VoronoiResult { closest, d1, d2 }
    }

    fn get_archetype_id(&self, coord: RegionCoord,  seed: i32) -> String {
        let h = hash2(coord,seed ^ 0xA53C9E);
        let idx = (h as usize) % self.archetypes_ids.len();
        self.archetypes_ids[idx].clone()
    }

    pub fn pos_to_region_cell(&self, pos: Vec2) -> RegionCoord {
        RegionCoord::new(
            (pos.x / self.region_size).floor() as i32,
            (pos.y / self.region_size).floor() as i32,
        )
    }

}

#[derive(Default)]
pub struct RegionArchetype {
    bias: ClimateField,
    max_climate_fields: ClimateField,
    terrain_profile_id: Option<String>,
    region_modifier_ids: Vec<String>,
    feature_rule_ids: Vec<String>,
}
impl RegionArchetype {
   pub fn get_bias(&self) -> ClimateField {
        self.bias
    }
    pub fn _get_max_climate(&self) -> ClimateField {
        self.max_climate_fields
    } 
    pub fn get_opt_terrain_profile_id(&self) -> &Option<String> {
        &self.terrain_profile_id
    }
    pub fn get_region_modifier_ids(&self) -> &Vec<String> {
        &self.region_modifier_ids
    }
    pub fn get_rule_ids(&self) -> &Vec<String> {
        &self.feature_rule_ids
    }
}

#[derive(Default)]
pub struct VoronoiResult {
    closest: RegionCoord,
    d1: f32,
    d2: f32,
}

#[derive(Default)]
pub struct RegionSample {
    coord: RegionCoord,
    archetype_id: String,
    edge_factor: f32,
}
impl RegionSample {
    pub fn get_coord(&self) -> RegionCoord {
        self.coord
    }
    pub fn get_archetype_id(&self) -> String {
        self.archetype_id.clone()
    }
    pub fn get_edge_factor(&self) -> f32 {
        self.edge_factor
    }
}