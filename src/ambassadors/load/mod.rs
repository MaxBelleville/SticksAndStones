use bevy::{app::PluginGroupBuilder, prelude::*};
mod server;
#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
pub enum LoadState {
    #[default]
    Boot,   //App start
    ServerStarting,
    Connecting,
    WorldGen2D,
    SpawnAssignment,
    WorldGen3D,
    Streaming,
    InGame,
}

pub struct LoadPlugins;
impl PluginGroup for LoadPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
        .add(LoadPlugin)
        .add(server::ServerPlugin)
    }
}

pub struct LoadPlugin;
impl Plugin for LoadPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<LoadState>();
    }
}