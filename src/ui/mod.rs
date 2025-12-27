use bevy::{app::PluginGroupBuilder, camera::visibility::RenderLayers, prelude::*};
use bevy_lunex::prelude::*;

mod load;
mod menu;
mod player;
mod settings;
mod splash;

use super::shared;

// State used for the current menu screen
#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
enum MenuState {
    Main,
    Settings,
    SettingsDisplay,
    SettingsSound,
    #[default]
    Disabled,
}
#[derive(Component)]
pub struct UICanvas;

#[derive(Component)]
pub struct Background;

#[derive(Component)]
pub struct PauseBackground;

#[derive(Component)]
pub struct SettingsBackground;

//Setup plugin group and menu resources
pub struct UIPlugins;

impl PluginGroup for UIPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(UIPlugin)
            .add(splash::SplashPlugin)
            .add(menu::MenuPlugin)
            .add(load::LoadPlugin)
            .add(settings::SettingsPlugin)
            .add(player::PlayerUIPlugin)
    }
}

struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(
                Update,
                (on_escape).run_if(
                    in_state(shared::GameState::InGame)
                        .or(in_state(shared::GameState::Paused))
                        .or(in_state(shared::GameState::InMenu)),
                ),
            )
            .add_systems(
                OnExit(shared::GameState::StartGame),
                shared::despawn::<Background>,
            );
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,

    mut game_state: ResMut<NextState<shared::GameState>>,
) {
    commands.spawn((
        // This camera will become the source for all UI paired to index 0.
        Camera2d,
        UiSourceCamera::<0>,
        // Ui nodes start at 0 and move + on the Z axis with each depth layer.
        // This will ensure you will see up to 1000 nested children.
        Transform::from_translation(Vec3::Z * 1000.0),
        RenderLayers::from_layers(&[0, 1]),
    ));

    commands
        .spawn((
            Name::new("Canvas Comp"),
            UiLayoutRoot::new_2d(),
            UICanvas,
            UiFetchFromCamera::<0>,
        ))
        .with_children(|ui| {
      

             ui.spawn((
                Name::new("CanvasRender"),
                UiLayout::solid()
                    .size((16.0, 9.0))
                    .scaling(Scaling::Fill)
                    .pack(),
              //  Sprite::from_image(world.render.clone()),
                UiEmbedding,
                RenderLayers::from_layers(&[0, 1]),
            ));

            ui.spawn((
                Background,
                // You can name your entites for easier debug
                Name::new("Background"),
                UiLayout::solid()
                    .size((640.0, 360.0))
                    .scaling(Scaling::Fill)
                    .pack(),
                Sprite::from_image(asset_server.load("images/menu/main_bg.png")),
            ));
        });

    game_state.set(shared::GameState::Splash);
}

fn on_escape(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    curr_game_state: Res<State<shared::GameState>>,
    curr_menu_state: Res<State<MenuState>>,
    mut game_state: ResMut<NextState<shared::GameState>>,
    mut menu_state: ResMut<NextState<MenuState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        //If in some other menu go to the main screen
        let (change_game, change_menu) = if curr_menu_state.get() != &MenuState::Main {
            //Check if paused or main menu.
            if curr_game_state.get() == &shared::GameState::InMenu {
                (shared::GameState::InMenu, MenuState::Main)
            } else {
                (shared::GameState::Paused, MenuState::Main)
            }
        }
        //Otherwise switch from ingame to paused
        else {
            if curr_game_state.get() == &shared::GameState::Paused {
                (shared::GameState::InGame, MenuState::Disabled)
            } else {
                (shared::GameState::Paused, MenuState::Main)
            }
        };

        game_state.set(change_game);
        menu_state.set(change_menu);
    }
}
