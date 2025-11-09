
use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*};

mod contree;
mod shared;
mod window_manager; 
mod player;
mod enviroment;
mod world_builder;
// mod ui;
// mod rendering; 

fn main() {
    //Initalized the nishita sky model
    App::new()
        .insert_resource(shared::res::ExtractedGameState(shared::GameState::default()))
        .insert_resource(shared::res::WorldData::default())
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            FrameTimeDiagnosticsPlugin::default(),
            window_manager::WindowManagerPlugin,
            enviroment::EnviromentPlugin,
          //  ui::UIPlugins,
            world_builder::WorldBuilderPlugins,
           // rendering::RenderSystemPlugin
        ))
        .init_state::<shared::GameState>()
        .add_systems(Update, sync_game_state)
        .run();
}


pub fn sync_game_state(state: Res<State<shared::GameState>>, mut extracted: ResMut<shared::res::ExtractedGameState>) {
    extracted.0 = *state.get();
}
