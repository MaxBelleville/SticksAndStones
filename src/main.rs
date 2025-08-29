
use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*};
use bevy_enhanced_input::prelude::*;
use bevy_lunex::UiLunexPlugins;


mod shared;
mod window_manager; 
mod ui;
mod player;
mod rendering; 
mod contree;
mod enviroment;
mod world_builder;


fn main() {
    //Initalized the nishita sky model
    App::new()
        .insert_resource(shared::res::ExtractedGameState(shared::GameState::default()))
        .insert_resource(shared::res::WorldData::default())
        .insert_resource(shared::components::DisplayQuality::Medium)
        .insert_resource(shared::components::Volume(7))
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            FrameTimeDiagnosticsPlugin::default(),
            window_manager::WindowManagerPlugin,
            EnhancedInputPlugin,
            player::PlayerPlugin,
            UiLunexPlugins,
            enviroment::EnviromentPlugin,
            ui::UIPlugins,
            world_builder::WorldBuilderPlugins,
            rendering::RenderSystemPlugin
        ))
        .init_state::<shared::GameState>()
        .add_systems(Update, sync_game_state)
        .run();
}


pub fn sync_game_state(state: Res<State<shared::GameState>>, mut extracted: ResMut<shared::res::ExtractedGameState>) {
    extracted.0 = *state.get();
}
