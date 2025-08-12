
use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*};
use bevy_enhanced_input::prelude::*;
use bevy_rand::prelude::{ChaCha12Rng, EntropyPlugin};
use noisy_bevy::NoisyShaderPlugin;


mod shared;
mod window_manager; 
mod ui;
mod flycam;
mod compute_system; 
mod contree;
mod world_builder;


// One of the two settings that can be set through the menu. It will be a resource in the app
#[derive(Resource, Debug, Component, PartialEq, Eq, Clone, Copy)]
enum DisplayQuality {
    Low,
    Medium,
    High,
}

// One of the two settings that can be set through the menu. It will be a resource in the app
#[derive(Resource, Debug, Component, PartialEq, Eq, Clone, Copy)]
struct Volume(u32);


fn main() {
    //Initalized the nishita sky model

    App::new()
        
        .insert_resource(DisplayQuality::Medium)
        .insert_resource(Volume(7))
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            FrameTimeDiagnosticsPlugin::default(),
            window_manager::WindowManagerPlugin,
            EntropyPlugin::<ChaCha12Rng>::default(),
            NoisyShaderPlugin,
            EnhancedInputPlugin,
            ui::UIPlugins,
            world_builder::WorldBuilderPlugins,
        ))
        .init_state::<shared::GameState>()
        .insert_resource(shared::res::ExtractedGameState(shared::GameState::default()))
        .add_systems(Update, sync_game_state)
        .run();
}


pub fn sync_game_state(state: Res<State<shared::GameState>>, mut extracted: ResMut<shared::res::ExtractedGameState>) {
    extracted.0 = *state.get();
}
