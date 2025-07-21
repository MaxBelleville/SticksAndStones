use bevy::color::palettes::css::CRIMSON;
use bevy::prelude::*;

use super::ButtonColors;
use super::MenuButtonAction;
use super::MenuState;
use super::TEXT_COLOR;
use super::shared;

#[derive(Component)]
struct OnMainMenuScreen;

pub struct MenuPlugin;
// This plugin manages the menu, with 3 different screens:
// - a main menu with "New Game", "Settings", "Quit"
// - a settings menu with two submenus and a back button(handled in Settings plugin)
// - two settings screen with a setting that can be set and a back button(handled in Settings plugin)
impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<MenuState>()
            .add_systems(OnEnter(shared::GameState::InMenu), menu_setup)
            .add_systems(OnEnter(shared::GameState::Paused), menu_setup)
            .add_systems(OnEnter(MenuState::Main), main_menu_setup)
            .add_systems(
                OnExit(MenuState::Main),
                shared::despawn::<OnMainMenuScreen>,
            );
    }
}

fn menu_setup(mut menu_state: ResMut<NextState<MenuState>>) {
    menu_state.set(MenuState::Main);
}

fn main_menu_setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    game_state: Res<State<shared::GameState>>,
) {
    // Common style for all buttons on the screen
    let button_node = Node {
        width: Val::Px(300.0),
        height: Val::Px(65.0),
        margin: UiRect::all(Val::Px(20.0)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    };
    let button_icon_node = Node {
        width: Val::Px(30.0),
        // This takes the icons out of the flexbox flow, to be positioned exactly
        position_type: PositionType::Absolute,
        // The icon will be close to the left border of the button
        left: Val::Px(10.0),
        ..default()
    };
    let button_text_font = TextFont {
        font_size: 33.0,
        ..default()
    };

    let right_icon = asset_server.load("images/right.png");
    let wrench_icon = asset_server.load("images/wrench.png");
    let exit_icon = asset_server.load("images/exitRight.png");
    let label = if game_state.get() == &shared::GameState::Paused {
        "Continue"
    } else {
        "New Game"
    };

    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        OnMainMenuScreen,
        children![(
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(CRIMSON.into()),
            children![
                // Display the game name
                (
                    Text::new("Bevy Game Menu UI"),
                    TextFont {
                        font_size: 67.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                    Node {
                        margin: UiRect::all(Val::Px(50.0)),
                        ..default()
                    },
                ),
                // Display three buttons for each action available from the main menu:
                // - new game
                // - settings
                // - quit
                (
                    Button,
                    button_node.clone(),
                    BackgroundColor(ButtonColors::NORMAL),
                    MenuButtonAction::Play,
                    children![
                        (ImageNode::new(right_icon), button_icon_node.clone()),
                        (
                            Text::new(label),
                            button_text_font.clone(),
                            TextColor(TEXT_COLOR),
                        ),
                    ]
                ),
                (
                    Button,
                    button_node.clone(),
                    BackgroundColor(ButtonColors::NORMAL),
                    MenuButtonAction::Settings,
                    children![
                        (ImageNode::new(wrench_icon), button_icon_node.clone()),
                        (
                            Text::new("Settings"),
                            button_text_font.clone(),
                            TextColor(TEXT_COLOR),
                        ),
                    ]
                ),
                (
                    Button,
                    button_node,
                    BackgroundColor(ButtonColors::NORMAL),
                    MenuButtonAction::Quit,
                    children![
                        (ImageNode::new(exit_icon), button_icon_node),
                        (Text::new("Quit"), button_text_font, TextColor(TEXT_COLOR),),
                    ]
                ),
            ]
        )],
    ));
}


