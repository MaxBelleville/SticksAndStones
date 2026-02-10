use bevy::prelude::*;

use crate::ui;

pub struct LoadSavePlugin;

impl Plugin for LoadSavePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ui::MenuState::LoadSave), setup);
    }
}

#[derive(Component)]
struct OnLoadSaveMenu;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
      let main_bg = asset_server.load("images/menu/main_bg.png");
       commands
        .spawn(ui::components::menu_bg(
            main_bg,
            ui::MenuState::LoadSave,
            OnLoadSaveMenu,
            JustifyContent::FlexStart,
            AlignItems::Baseline,
        ));
}
