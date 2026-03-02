use bevy::prelude::*;

use crate::ui;
pub struct QuickSelectPlugin;

impl Plugin for QuickSelectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ui::GameState::QuickSelect), setup);
    }
}

#[derive(Component)]
struct OnQuickSelect;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
}