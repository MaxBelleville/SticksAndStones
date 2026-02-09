use bevy::prelude::*;

use crate::ui;


pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ui::MenuState::Settings), setup);
    }
}

#[derive(Component)]
struct OnSettingsMenu;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
      let main_bg = asset_server.load("images/menu/main_bg.png");
       commands
        .spawn(ui::components::menu_bg(
            main_bg,
            ui::MenuState::Settings,
            OnSettingsMenu,
            JustifyContent::FlexStart,
            AlignItems::Baseline,
        ));
}
