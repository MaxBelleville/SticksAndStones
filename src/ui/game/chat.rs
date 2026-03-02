use bevy::prelude::*;

use crate::ui;
pub struct ChatPlugin;

impl Plugin for ChatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ui::GameState::Chat), setup);
    }
}

#[derive(Component)]
struct OnChat;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
}