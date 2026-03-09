use crate::sns_prelude::*;
use crate::ui;
use bevy::app::AppExit;
use bevy::{app::PluginGroupBuilder, prelude::*};
pub struct MenuPlugins;

mod load_save;
mod multiplayer;
mod new_game;
mod save_games;
mod settings;

impl PluginGroup for MenuPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
        .add(MenuPlugin)
        .add(load_save::LoadSavePlugin)
        .add(multiplayer::MultiplayerPlugin)
        .add(new_game::NewGamePlugin)
        .add(save_games::SaveGamePlugin)
        .add(settings::SettingsPlugin)
    }
}

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ui::MenuState::Main), main_menu_setup);
        app.add_systems(OnEnter(ui::MenuState::Pause), pause_menu_setup);
    }
}

#[derive(Component)]
struct OnPauseMenu;

#[derive(Component)]
struct OnMainMenu;

fn main_menu_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let main_bg = asset_server.load("images/menu/main_bg.png");
    let button_img = asset_server.load("images/menu/button.png");
    let button_font = asset_server.load("fonts/monogram.ttf");

    let menu_buttons = Vec::from([
        "Continue",
        "New Game",
        "Load Game",
        "Multiplayer",
        "Settings",
        "Quit Game",
    ]);

    //Check if you have saves

    commands
        .spawn(ui::components::menu_bg(
            main_bg,
            ui::MenuState::Main,
            OnMainMenu,
            JustifyContent::FlexStart,
            AlignItems::Baseline,
        ))
        .with_children(|parent| {
            parent.spawn(ui::components::title(
                asset_server,
                vec2(520.0, 64.0),
            ));
            parent.spawn(Node {
                margin: UiRect::vertical(ui::MARGIN),
                ..Default::default()
            });
            for button in menu_buttons {
                parent
                    .spawn(ui::components::button(
                        button,
                        button_img.clone(),
                        button_font.clone(),
                        64.0,
                        vec2(300.0, 64.0),
                    ))
                    .observe(on_over)
                    .observe(on_out)
                    .observe(on_click);
            }
        });
}

fn pause_menu_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let pause_img = asset_server.load("images/menu/pause_bg.png");
    let button_img = asset_server.load("images/menu/button.png");
    let button_font = asset_server.load("fonts/monogram.ttf");

    let menu_buttons = Vec::from([
        "Continue",
        "Save Game",
        "Load Game",
        "Settings",
        "Quit Game",
    ]);

    commands
        .spawn(ui::components::menu_bg(
            pause_img,
            ui::MenuState::Pause,
            OnPauseMenu,
            JustifyContent::FlexStart,
            AlignItems::Baseline,
        ))
        .with_children(|parent| {
            parent.spawn(ui::components::title(
                asset_server,
                vec2(520.0, 64.0),
            ));
            parent.spawn(Node {
                margin: UiRect::vertical(ui::MARGIN),
                ..Default::default()
            });
            for button in menu_buttons {
                parent
                    .spawn(ui::components::button(
                        button,
                        button_img.clone(),
                        button_font.clone(),
                        64.0,
                        vec2(300.0, 64.0),
                    ))
                    .observe(on_over)
                    .observe(on_out)
                    .observe(on_click);
            }
        });
}

fn on_over(
    over: On<Pointer<Over>>,
    asset_server: Res<AssetServer>,
    mut query: Query<(&mut ImageNode, &mut Node)>,
) {
    let hover_img = asset_server.load("images/menu/hover_button.png");

    if let Ok((mut image, mut node)) = query.get_mut(over.entity) {
        image.image = hover_img;
        node.top = px(3.0);
    }
}

fn on_out(
    out: On<Pointer<Out>>,
    asset_server: Res<AssetServer>,
    mut query: Query<(&mut ImageNode, &mut Node)>,
) {
    let button_img = asset_server.load("images/menu/button.png");

    if let Ok((mut image, mut node)) = query.get_mut(out.entity) {
        image.image = button_img;
        node.top = px(0.0);
    }
}

fn on_click(
    click: On<Pointer<Click>>,
    query: Query<&ui::ButtonLabel>,
    mut next_manager: ResMut<NextState<ui::ManagerState>>,
    mut next_menu: ResMut<NextState<ui::MenuState>>,
    mut next_load: ResMut<NextState<LoadState>>,
    next_game: Option<ResMut<NextState<ui::GameState>>>,
    game_state: Option<Res<State<ui::GameState>>>,
    mut exit: MessageWriter<AppExit>,
) {
    let Ok(label) = query.get(click.entity) else {
        return;
    };
    //Should check for pause menu first
    match label.0 {
        "Continue" => {
            if let Some(_game) = game_state {
                next_game.unwrap().set(ui::GameState::Normal);
                next_menu.set(ui::MenuState::Disabled);
            } else {
                next_manager.set(ui::ManagerState::LoadingGame);
                next_load.set(LoadState::ServerStarting);
                next_menu.set(ui::MenuState::Disabled);
            }
        }
        "New Game" => {
            next_menu.set(ui::MenuState::NewGame);
        }
        "Load Save" => {
            next_menu.set(ui::MenuState::LoadSave);
        }
        "Multiplayer" => {
            next_menu.set(ui::MenuState::Multiplayer);
        }
        "Settings" => {
            next_menu.set(ui::MenuState::Settings);
        }
        "Quit Game" => {
            exit.write(AppExit::Success);
        }
        _ => info!("Clicked: {}", label.0),
    }
}
