use bevy::{
    asset::AssetMetaCheck,
    diagnostic::FrameTimeDiagnosticsPlugin,
    gltf::{GltfPlugin, convert_coordinates::GltfConvertCoordinates},
    pbr::DefaultOpaqueRendererMethod,
    prelude::*,
};
use bevy_skein::SkeinPlugin;
use iyes_progress::ProgressPlugin;
use leafwing_input_manager::prelude::*;

use crate::renderer::RasterMaterial;


mod ambassadors;
mod net;
mod player;
mod ui;
mod window;

mod renderer;

fn main() {
    App::new()
        .insert_resource(ui::ExtractedManagerState(ui::ManagerState::default()))
        .insert_resource(GlobalAmbientLight::NONE)
        .init_resource::<ActionState<player::Input>>()
        .add_plugins((
            DefaultPlugins
                .set(AssetPlugin {
                    // Wasm builds will check for meta files (that don't exist) if this isn't set.
                    // This causes errors and even panics on web build on itch.
                    // See https://github.com/bevyengine/bevy_github_ci_template/issues/48.
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Window {
                        title: "Sticks_and_Stones".to_string(),
                        fit_canvas_to_parent: true,
                        ..default()
                    }
                    .into(),
                    ..default()
                })
                .set(ImagePlugin::default_nearest())
                .set(GltfPlugin {
                    convert_coordinates: GltfConvertCoordinates {
                        rotate_scene_entity: true,
                        rotate_meshes: true,
                    },
                    ..default()
                }),
            MaterialPlugin::<RasterMaterial>::default(),  
            FrameTimeDiagnosticsPlugin::default(),
            window::WindowPlugin,
            SkeinPlugin::default(),
       
            ui::UIPlugins,
            ProgressPlugin::<ui::ManagerState>::new()
                .with_state_transition(ui::ManagerState::LoadingGame, ui::ManagerState::InGame),

            net::client::ClientPlugin,
            ambassadors::AmbassorPlugins,

            InputManagerPlugin::<player::Input>::default(),
            player::PlayerPlugin,
            
            renderer::RendererPlugins,
        ))
        .add_systems(Update, ui::sync_manager_state)
        .run();
}
//Simplify accessing larger path namespaces
pub mod sns_prelude {
    pub use crate::ambassadors::prelude::*;
}
