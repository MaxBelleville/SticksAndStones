use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, pbr::DefaultOpaqueRendererMethod, prelude::*};
use bevy_skein::SkeinPlugin;
use iyes_progress::ProgressPlugin;

mod ui;
mod window;
mod ambassadors;
mod net;

mod renderer;

fn main() {
    App::new()
        .insert_resource(ui::ExtractedManagerState(ui::ManagerState::default()))
        .insert_resource(DefaultOpaqueRendererMethod::deferred())
        .insert_resource(GlobalAmbientLight::NONE)
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            FrameTimeDiagnosticsPlugin::default(),
            window::WindowPlugin,
            SkeinPlugin::default(),
            ui::UIPlugins,
            ProgressPlugin::<ui::ManagerState>::new()
                .with_state_transition(ui::ManagerState::LoadingGame, ui::ManagerState::InGame),
            net::NetPlugin,
            ambassadors::AmbassorPlugins,
            renderer::RendererPlugin
        ))
        .add_systems(Update, ui::sync_manager_state)
        .run();
}

