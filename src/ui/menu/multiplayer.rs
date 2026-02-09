use bevy::prelude::*;

use crate::ui;

pub struct MultiplayerPlugin;

impl Plugin for MultiplayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ui::MenuState::Multiplayer), setup);
    }
}

#[derive(Component)]
struct OnMutiplayerMenu;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
      let main_bg = asset_server.load("images/menu/main_bg.png");
       commands
        .spawn(ui::components::menu_bg(
            main_bg,
            ui::MenuState::Multiplayer,
            OnMutiplayerMenu,
            JustifyContent::FlexStart,
            AlignItems::Baseline,
        ));
}
