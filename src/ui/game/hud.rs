use bevy::prelude::*;

use crate::ui;
pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ui::GameState::Normal), setup);
    }
}

#[derive(Component)]
struct OnHud;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
}