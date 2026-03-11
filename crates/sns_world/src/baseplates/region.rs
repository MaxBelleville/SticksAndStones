use core::f32;
use std::collections::HashMap;

use sns_core::prelude::*;

use bevy_math::*;

pub type Coord = IVec2;


pub struct Plate {
    //Node id: node
    archetypes: HashMap<RegionArchetypeId, Archetype>,
    archetypes_ids: Vec<RegionArchetypeId>, //Collect and sort keys when load in archetypes from file.
    region_size: f32,
}
impl Default for Plate {
    fn default() -> Self { 
        Self {
            archetypes: HashMap::new(),
            archetypes_ids: Vec::new(),
            region_size: 4096.0
        }
    }
}

impl Plate {
    pub fn get_archetype(&self, id: RegionArchetypeId) -> Option<&Archetype> {
        self.archetypes.get(&id)
    }

    pub fn sample(&self, coord: Vec2, seed: i32)-> Sample {
        let v = self.voronoi_query(coord, seed);
        let archetype_id = self.get_archetype_id(v.closest, seed);

        let edge_factor = ((v.d2.sqrt() - v.d1.sqrt()) / self.region_size)
            .clamp(0.0, 1.0);

        Sample {
            _coord: v.closest,
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
                let c = base + Coord::new(dx, dy);
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

    fn get_archetype_id(&self, coord: Coord,  seed: i32) -> RegionArchetypeId {
        let h = hash2(coord,seed ^ 0xA53C9E);
        let idx = (h as usize) % self.archetypes_ids.len();
        self.archetypes_ids[idx]
    }

    pub fn pos_to_region_cell(&self, pos: Vec2) -> Coord {
        Coord::new(
            (pos.x / self.region_size).floor() as i32,
            (pos.y / self.region_size).floor() as i32,
        )
    }

}

#[derive(Default)]
pub struct VoronoiResult {
    closest: Coord,
    d1: f32,
    d2: f32,
}

#[derive(Default)]
pub struct Sample {
    _coord: Coord,
    archetype_id: RegionArchetypeId,
    edge_factor: f32,
}
impl Sample {
    pub fn _get_coord(&self) -> Coord {
        self._coord
    }
    pub fn get_archetype_id(&self) -> RegionArchetypeId {
        self.archetype_id
    }
    pub fn get_edge_factor(&self) -> f32 {
        self.edge_factor
    }
}