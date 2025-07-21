use bevy::prelude::*;

pub mod components;
pub mod maths;
pub mod res;

//Constants
pub const COMPUTE_SHADER_PATH: &str = "shaders/raymarch.wgsl";

pub const FRAG_SHADER_PATH: &str ="shaders/fullquad.wgsl";

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
pub enum GameState {
    #[default]
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
