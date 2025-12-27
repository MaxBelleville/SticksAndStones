use bevy::prelude::*;
use bevy_lunex::prelude::*;
use super::shared;
use super::{Background, MenuState, UICanvas,SettingsBackground};

// Tag component used to tag entities added on the settings menu screen
#[derive(Component)]
struct SettingsMenu;


pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(MenuState::Settings), settings_menu_setup)
            .add_systems(OnExit(MenuState::Settings), shared::despawn::<SettingsMenu>);
    }
}

fn settings_menu_setup(
    mut commands: Commands,
    mut menu_state: ResMut<NextState<MenuState>>,
    asset_server: Res<AssetServer>,
    container_query: Query<Entity, With<UICanvas>>,
    background_query: Query<Entity, (With<Background>,With<SettingsBackground>)>,
) {
    if let Ok(container) = container_query.single() {
        commands.entity(container).with_children(|ui| {
            if background_query.single().is_err() {
                ui.spawn((
                    SettingsBackground,
                    // You can name your entites for easier debug
                    Name::new("Background"),
                    UiLayout::solid()
                        .size((640.0, 360.0))
                        .scaling(Scaling::Fill)
                        .pack(),
                    Sprite::from_image(asset_server.load("images/menu/main_bg.png")),
                ));
            }
        });
    }
}
