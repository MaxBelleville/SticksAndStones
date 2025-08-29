use bevy::prelude::*;
use super::contree;

pub mod components;
pub mod maths;
pub mod res;

//Constants
pub const RAYMARCH_PATH: &str = "shaders/raymarch.wgsl";
pub const RASTERIZE_PATH: &str = "shaders/rasterize.wgsl";
pub const LIGHTING_PATH: &str = "shaders/lighting.wgsl";
pub const PREFILL_PATH: &str = "shaders/prefill_depth.wgsl";
#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
pub enum GameState {
    #[default]
    None,
    Splash,
    InMenu,
    StartGame,
    InGame,
    Paused
}

// Generic system that takes a component as a parameter, and will despawn all entities with that component
pub fn despawn<T: Component>(to_despawn: Query<Entity, With<T>>, mut commands: Commands) {
    for entity in &to_despawn {
        commands.entity(entity).despawn();
    }
}
