use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, pbr::DefaultOpaqueRendererMethod, prelude::*};
use bevy_skein::SkeinPlugin;
use iyes_progress::ProgressPlugin;

mod ambassadors;
mod net;
mod ui;
mod window;

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
            net::client::ClientPlugin,
            ambassadors::AmbassorPlugins,
            renderer::RendererPlugin,
        ))
        .add_systems(Update, ui::sync_manager_state)
        .run();
}
//Simplify accessing larger path namespaces
pub mod sns_prelude {
    pub use crate::ambassadors::prelude::*;
}
