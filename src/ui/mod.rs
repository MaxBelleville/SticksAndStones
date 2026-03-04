use bevy::{app::PluginGroupBuilder, camera::visibility::RenderLayers, prelude::*, render::extract_resource::ExtractResource};

use crate::renderer;

mod components;
mod game;
mod load;
mod menu;
mod splash;


pub const MARGIN: Val = Val::Px(12.);

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
pub enum ManagerState {
    #[default]
    Splash,
    InMenu,
    LoadingGame,
    InGame,
}

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
pub enum MenuState {
    #[default]
    Disabled,
    Main,
    Settings,
    Multiplayer,
    LoadSave,
    SaveGame,
    NewGame,
    Pause,
}

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, SubStates)]
#[source(ManagerState = ManagerState::InGame)]
pub enum GameState {
    #[default]
    Normal,
    Inventory,
    QuickSelect,
    Chat,
    Interaction,
    PlayerList,
    Map,
    Console,
    Quest,
    Paused,
}

#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Hash, ExtractResource)]
pub struct ExtractedManagerState(pub ManagerState);

#[derive(Component)]
pub struct ButtonLabel(pub &'static str);


pub fn sync_manager_state(
    state: Res<State<ManagerState>>,
    mut extracted: ResMut<ExtractedManagerState>,
) {
    extracted.0 = *state.get();
}

pub struct UIPlugins;

impl PluginGroup for UIPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(UIPlugin)
            .add(splash::SplashPlugin)
            .add(load::LoadPlugin)
            .add_group(menu::MenuPlugins)
            .add_group(game::GamePlugins)
    }
}

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<ManagerState>()
            .init_state::<MenuState>()
            .add_sub_state::<GameState>()
            .add_systems(Startup, setup);
    }
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        IsDefaultUiCamera,
        Camera {
            order: renderer::CameraOrder::Ui.into(), // Render second (on top)
            ..default()
        },
        RenderLayers::from(renderer::RenderLayer::UI),//Probably not needed but just to be safe
        renderer::UIModelCamera,
    ));
}
