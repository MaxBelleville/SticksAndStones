use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*};
use bevy_lunex::UiLunexPlugins;
use bevy_skein::SkeinPlugin;

mod player;
mod ui;
mod shared;
mod window_manager;
mod world_gen;

fn main() {
    //Initalized the nishita sky model
    App::new()
        .insert_resource(shared::res::ExtractedGameState(shared::GameState::default()))
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            FrameTimeDiagnosticsPlugin::default(),
            window_manager::WindowManagerPlugin,
            UiLunexPlugins,
            SkeinPlugin::default(),
            ui::UIPlugins,
            player::PlayerPlugin,
            world_gen::WorldGenPlugin,
        ))
        .init_state::<shared::GameState>()
        .add_systems(Update, sync_game_state)
        .run();
}

pub fn sync_game_state(
    state: Res<State<shared::GameState>>,
    mut extracted: ResMut<shared::res::ExtractedGameState>,
) {
    extracted.0 = *state.get();
}
