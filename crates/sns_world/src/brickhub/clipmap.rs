use bevy_math::*;
use std::{collections::HashMap, num::NonZeroUsize};
use lru::LruCache;
use super::CellID;


#[derive(Clone, Copy)]
pub struct ClipmapRing {
    pub level: u8,
    pub radius_cells: i32,
    pub max_cells: usize,
}

pub const CLIPMAP_RINGS: &[ClipmapRing] = &[
    ClipmapRing { level: 0, radius_cells: 4,  max_cells: 9 * 9 * 9 },
    ClipmapRing { level: 1, radius_cells: 8,  max_cells: 17 * 17 * 17 },
    ClipmapRing { level: 2, radius_cells: 16, max_cells: 33 * 33 * 33 },
];


pub fn classify_ring(cell: IVec3, view_cell: IVec3) -> Option<u8> {
    let delta = (cell - view_cell).abs();
    let dist = delta.x.max(delta.y).max(delta.z);

    CLIPMAP_RINGS
        .iter()
        .find(|ring| dist <= ring.radius_cells)
        .map(|ring| ring.level)
}

pub struct ResidencyManager {
    budgets: HashMap<u8, usize>,            // ring → max allowed
    lru_per_ring: HashMap<u8, LruCache<CellID,u64>>,
}
impl Default for ResidencyManager {
    fn default() -> Self {
        let mut budgets = HashMap::new();

        for ring in CLIPMAP_RINGS {
            budgets.insert(ring.level, ring.max_cells);
        }

        Self {
            budgets,
            lru_per_ring: HashMap::new(),
        }
    }
}

#[derive(Clone, Copy)]
pub enum ResidencyDecision {
    Admit,
    EvictAndAdmit(CellID),
    Reject,
}

impl ResidencyManager {
    pub fn admit(
        &mut self,
        id: CellID,
        ring: u8,
        frame: u64,
    ) -> ResidencyDecision {
        let budget = self.budgets[&ring];
        let default = LruCache::new(NonZeroUsize::new(budget).unwrap());
        let lru = self.lru_per_ring.entry(ring).or_insert(default);

        if lru.len() >= budget {
            if let Some((evict_id,_evict_frame)) = lru.pop_lru() {
                lru.push(id, frame);
                return ResidencyDecision::EvictAndAdmit(evict_id);
            }
            return ResidencyDecision::Reject;
        }

        lru.push(id, frame);
        ResidencyDecision::Admit
    }

    pub fn touch(&mut self, id: CellID, ring: u8, frame: u64) {
        if let Some(lru) = self.lru_per_ring.get_mut(&ring) {
            lru.push(id, frame);
        }
    }

    pub fn remove(&mut self, id: CellID, ring: u8) {
        if let Some(lru) = self.lru_per_ring.get_mut(&ring) {
            lru.pop_entry(&id);
        }
    }
    pub fn ring_full(&self, ring: u8) -> bool {
        let max = match self.budgets.get(&ring) {
            Some(b) => *b,
            None => return false,
        };

        let current = self.lru_per_ring
            .get(&ring)
            .map(|lru| lru.len())
            .unwrap_or(0);

        current >= max
    }


    /// Active count for debugging/metrics
    pub fn active_count(&self, ring: u8) -> usize {
        self.lru_per_ring
            .get(&ring)
            .map(|lru| lru.len())
            .unwrap_or(0)
    }

}