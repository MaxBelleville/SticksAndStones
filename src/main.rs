use bevy::{
    diagnostic::FrameTimeDiagnosticsPlugin,
    pbr::DefaultOpaqueRendererMethod,
    prelude::*
};
use bevy_skein::SkeinPlugin;
use iyes_progress::ProgressPlugin;
use leafwing_input_manager::prelude::*;

mod ambassadors;
mod net;
mod ui;
mod window;
mod player;

mod renderer;

fn main() {
    App::new()
        .insert_resource(ui::ExtractedManagerState(ui::ManagerState::default()))
        .insert_resource(DefaultOpaqueRendererMethod::deferred())
        .insert_resource(GlobalAmbientLight::NONE)
        .init_resource::<ActionState<player::Input>>()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            FrameTimeDiagnosticsPlugin::default(),
            window::WindowPlugin,
            SkeinPlugin::default(),
             InputManagerPlugin::<player::Input>::default(),
            ui::UIPlugins,
            ProgressPlugin::<ui::ManagerState>::new()
                .with_state_transition(ui::ManagerState::LoadingGame, ui::ManagerState::InGame),
            net::client::ClientPlugin,
            ambassadors::AmbassorPlugins,
            player::PlayerPlugin,
            renderer::RendererPlugin,
           
        ))
        .add_systems(Update, ui::sync_manager_state)
        .run();
}
//Simplify accessing larger path namespaces
pub mod sns_prelude {
    pub use crate::ambassadors::prelude::*;
}
