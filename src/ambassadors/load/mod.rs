use crate::net::ClientState;
use bevy::{app::PluginGroupBuilder, prelude::*};
#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
pub enum LoadState {
    #[default]
    Boot, //App start
    ServerStarting,
    LoadAssets,
    BaseplateGen,
    BrickmapGen,
    InitGPU,
    InGame,
}

pub struct LoadPlugins;
impl PluginGroup for LoadPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>().add(LoadPlugin)
    }
}

pub struct LoadPlugin;
impl Plugin for LoadPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<LoadState>()
            .add_systems(OnEnter(LoadState::ServerStarting), start_local_server)
            .add_systems(OnEnter(ClientState::Connected), load_assets);
    }
}

fn start_local_server(mut next: ResMut<NextState<ClientState>>) {
    next.set(ClientState::StartLocal);
}

fn load_assets(mut next: ResMut<NextState<LoadState>>) {
    next.set(LoadState::LoadAssets);
}
