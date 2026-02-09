use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, pbr::DefaultOpaqueRendererMethod, prelude::*};
use bevy_skein::SkeinPlugin;

mod ui;
mod shared;
mod window;
mod world_gen;
mod net;
mod hdr;


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
            world_gen::WorldGenPlugin,
            net::NetPlugin,
            hdr::HdrPlugin,
        ))
        .add_systems(Update, ui::sync_manager_state)
        .run();
}

