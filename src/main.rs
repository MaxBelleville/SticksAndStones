use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*};
use bevy_lunex::UiLunexPlugins;
use bevy_skein::SkeinPlugin;

mod player;
mod shared;
mod ui;
mod window_manager;

fn main() {
    //Initalized the nishita sky model
    App::new()
        .insert_resource(shared::res::ExtractedGameState(shared::GameState::default()))
        .add_plugins((
            DefaultPlugins
                .set(AssetPlugin {
                    mode: AssetMode::Processed,
                    ..Default::default()
                })
                .set(ImagePlugin::default_nearest()),
            FrameTimeDiagnosticsPlugin::default(),
            window_manager::WindowManagerPlugin,
            UiLunexPlugins,
            SkeinPlugin::default(),
            ui::UIPlugins,
            player::PlayerPlugin,
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
