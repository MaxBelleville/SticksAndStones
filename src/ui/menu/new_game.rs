use bevy::prelude::*;

use crate::ui;

pub struct NewGamePlugin;

impl Plugin for NewGamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ui::MenuState::NewGame), setup);
    }
}

#[derive(Component)]
struct OnNewGameMenu;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
      let main_bg = asset_server.load("images/menu/main_bg.png");
       commands
        .spawn(ui::components::menu_bg(
            main_bg,
            ui::MenuState::NewGame,
            OnNewGameMenu,
            JustifyContent::FlexStart,
            AlignItems::Baseline,
        ));
}
