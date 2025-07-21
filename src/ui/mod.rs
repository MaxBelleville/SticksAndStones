use bevy::{app::PluginGroupBuilder, prelude::*};
mod menu;
mod player;
mod settings;
mod splash;

use super::DisplayQuality;
use super::Volume;
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

// Tag component used to mark which setting is currently selected
#[derive(Component)]
struct SelectedOption;

// All actions that can be triggered from a button click
#[derive(Component)]
enum MenuButtonAction {
    Play,
    Settings,
    SettingsDisplay,
    SettingsSound,
    BackToMainMenu,
    BackToSettings,
    Quit,
}

const TEXT_COLOR: Color = Color::srgb(0.9, 0.9, 0.9);

pub struct ButtonColors;

impl ButtonColors {
    pub const NORMAL: Color = Color::srgb(0.15, 0.15, 0.15);
    pub const HOVER: Color = Color::srgb(0.25, 0.25, 0.25);
    pub const HOVER_PRESSED: Color = Color::srgb(0.25, 0.65, 0.25);
    pub const PRESSED: Color = Color::srgb(0.35, 0.75, 0.35);
}

//Setup plugin group and menu resources
pub struct UIPlugins;

impl PluginGroup for UIPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(UIPlugin)
            .add(splash::SplashPlugin)
            .add(menu::MenuPlugin)
            .add(settings::SettingsPlugin)
        //.add( player::PlayerUIPlugin)
    }
}

struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(
                Update,
                (menu_action, button_system).run_if(
                    in_state(shared::GameState::InMenu).or(in_state(shared::GameState::Paused)),
                ),
            )
             .add_systems(
                Update,
                (pause_on_escape).run_if(
                    in_state(shared::GameState::InGame).or(in_state(shared::GameState::Paused)),
                ),
            )
            .add_systems(
                OnExit(shared::GameState::InMenu),
                shared::despawn::<Camera2d>
                    .run_if(not(resource_exists::<shared::res::IsGamePrepared>)),
            );
    }
}

fn setup(mut commands: Commands) {
    commands.spawn((Camera2d,));
}

// This system handles changing all buttons color based on mouse interaction
fn button_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, Option<&SelectedOption>),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut background_color, selected) in &mut interaction_query {
        *background_color = match (*interaction, selected) {
            (Interaction::Pressed, _) | (Interaction::None, Some(_)) => {
                ButtonColors::PRESSED.into()
            }
            (Interaction::Hovered, Some(_)) => ButtonColors::HOVER_PRESSED.into(),
            (Interaction::Hovered, None) => ButtonColors::HOVER.into(),
            (Interaction::None, None) => ButtonColors::NORMAL.into(),
        }
    }
}

fn menu_action(
    interaction_query: Query<
        (&Interaction, &MenuButtonAction),
        (Changed<Interaction>, With<Button>),
    >,
    mut app_exit_events: EventWriter<AppExit>,
    mut menu_state: ResMut<NextState<MenuState>>,
    curr_game_state: Res<State<shared::GameState>>,
    mut game_state: ResMut<NextState<shared::GameState>>,
) {
    let state = if curr_game_state.get() == &shared::GameState::Paused {
        shared::GameState::InGame
    } else {
        shared::GameState::InGame
    };

    for (interaction, menu_button_action) in &interaction_query {
        if *interaction == Interaction::Pressed {
            match menu_button_action {
                MenuButtonAction::Quit => {
                    app_exit_events.write(AppExit::Success);
                }
                MenuButtonAction::Play => {
                    game_state.set(state);
                    menu_state.set(MenuState::Disabled);
                }
                MenuButtonAction::Settings => menu_state.set(MenuState::Settings),
                MenuButtonAction::SettingsDisplay => {
                    menu_state.set(MenuState::SettingsDisplay);
                }
                MenuButtonAction::SettingsSound => {
                    menu_state.set(MenuState::SettingsSound);
                }
                MenuButtonAction::BackToMainMenu => menu_state.set(MenuState::Main),
                MenuButtonAction::BackToSettings => {
                    menu_state.set(MenuState::Settings);
                }
            }
        }
    }
}

fn pause_on_escape(keyboard_input: Res<ButtonInput<KeyCode>>, 
    curr_game_state: Res<State<shared::GameState>>,
    mut game_state: ResMut<NextState<shared::GameState>>,
    mut menu_state: ResMut<NextState<MenuState>>) {
    let (change_game,change_menu) = if curr_game_state.get() == &shared::GameState::Paused {
        (shared::GameState::InGame,MenuState::Disabled)
    } else {
        (shared::GameState::Paused,MenuState::Main)
    };
    

    if keyboard_input.just_pressed(KeyCode::Escape) {
         game_state.set(change_game);
         menu_state.set(change_menu);
    }
}