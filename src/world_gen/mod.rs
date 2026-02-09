
use bevy::prelude::*;

pub struct World {
    seed: u64,
    version: f32,
}

pub struct WorldGenPlugin;
impl Plugin for WorldGenPlugin {
    fn build(&self, app: &mut App) {

    }
}