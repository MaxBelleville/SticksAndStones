use bevy::prelude::*;

use crate::ui;
pub struct ConsolePlugin;

impl Plugin for ConsolePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ui::GameState::Console), setup);
    }
}

#[derive(Component)]
struct OnConsole;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
}