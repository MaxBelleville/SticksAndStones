use super::LoadState;
use crate::net::{
    server::{DatabaseBuilt, LocalServer},
};
use bevy::prelude::*;
pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(LoadState::ServerStarting), start_local_server)
            .add_systems(
                Update,
                (
                    wait_for_server_ready.run_if(in_state(LoadState::ServerStarting)),
                    //Mointoring
                ),
            );
    }
}

fn start_local_server(mut commands: Commands) {
    let server = LocalServer::start(true);
    commands.insert_resource(server);
}

fn wait_for_server_ready(
    server: Option<Res<LocalServer>>,
    mut next: ResMut<NextState<LoadState>>,
    mut val: Local<i32>,
) {
    let Some(server) = server else {
        return;
    };
    if !server.is_ready() {
        return;
    }

    //timeout
    if *val >= 10 {
        panic!("Took too long to boot server");
    }
    *val += 1;
    info!("Server is live!");
    next.set(LoadState::Connecting);
}

fn monitor_server_health(mut server: ResMut<LocalServer>, mut next: ResMut<NextState<LoadState>>) {
    if !server.is_alive() {
        error!("Server process died unexpectedly!");
        next.set(LoadState::Boot);
    }
}

fn build_dbs_once(server: Res<LocalServer>, mut built: ResMut<DatabaseBuilt>) {
    if built.get() {
        return;
    }
    server.build_dbs();
    built.set(true);
}
