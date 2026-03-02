use bevy::prelude::*;

use crate::ui;
pub struct PlayerListPlugin;

impl Plugin for PlayerListPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ui::GameState::PlayerList), setup);
    }
}

#[derive(Component)]
struct OnPlayerList;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
}