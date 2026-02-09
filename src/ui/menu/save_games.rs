use bevy::prelude::*;

use crate::ui;

pub struct SaveGamePlugin;

impl Plugin for SaveGamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ui::MenuState::SaveGame), setup);
    }
}

#[derive(Component)]
struct OnSaveGameMenu;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
      let main_bg = asset_server.load("images/menu/main_bg.png");
       commands
        .spawn(ui::components::menu_bg(
            main_bg,
            ui::MenuState::SaveGame,
            OnSaveGameMenu,
            JustifyContent::FlexStart,
            AlignItems::Baseline,
        ));
}
