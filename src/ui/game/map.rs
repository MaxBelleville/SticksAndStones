use bevy::prelude::*;

use crate::ui;
pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ui::GameState::Map), setup);
    }
}

#[derive(Component)]
struct OnMap;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
}