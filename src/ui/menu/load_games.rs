use bevy::prelude::*;

use crate::ui;

pub struct LoadGamePlugin;

impl Plugin for LoadGamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ui::MenuState::LoadGame), setup);
    }
}

#[derive(Component)]
struct OnLoadGameMenu;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
      let main_bg = asset_server.load("images/menu/main_bg.png");
       commands
        .spawn(ui::components::menu_bg(
            main_bg,
            ui::MenuState::LoadGame,
            OnLoadGameMenu,
            JustifyContent::FlexStart,
            AlignItems::Baseline,
        ));
}
