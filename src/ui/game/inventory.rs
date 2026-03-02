use bevy::prelude::*;

use crate::ui;
pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ui::GameState::Inventory), setup);
    }
}

#[derive(Component)]
struct OnInventory;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
}