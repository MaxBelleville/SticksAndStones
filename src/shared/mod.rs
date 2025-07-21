use bevy::state::state::States;

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

