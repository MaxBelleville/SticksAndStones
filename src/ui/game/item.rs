use bevy::prelude::*;

use crate::ui;
pub struct ItemPlugin;

impl Plugin for ItemPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ui::GameState::Inventory), setup);
    }
}

#[derive(Component)]
struct OnItem;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
}