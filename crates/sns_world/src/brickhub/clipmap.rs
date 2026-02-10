use bevy_math::*;
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