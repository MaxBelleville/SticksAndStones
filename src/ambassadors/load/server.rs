use core::panic;

use super::LoadState;
use crate::net::server::{DatabaseBuilt, LocalServer};
use bevy::prelude::*;
pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(LoadState::ServerStarting), start_local_server)
            .add_systems(OnEnter(LoadState::ServerLoadDatabase), build_local_db)
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
    commands.insert_resource(DatabaseBuilt::default());
}

fn wait_for_server_ready(
    opt_server: Option<Res<LocalServer>>,
    mut next: ResMut<NextState<LoadState>>,
    mut val: Local<i32>,
) {
    if let Some(server) = opt_server {
        if server.is_ready() {
            info!("Server is live!");
            next.set(LoadState::ServerLoadDatabase);
            
        }
    }

    //timeout
    if *val >= 100 {
        panic!("Took too long to boot server");
    }
    *val += 1;

}

fn monitor_server_health(mut server: ResMut<LocalServer>, mut next: ResMut<NextState<LoadState>>) {
    if !server.is_alive() {
        error!("Server process died unexpectedly!");
        next.set(LoadState::Boot);
    }
}

fn build_local_db(server: Res<LocalServer>, mut built: ResMut<DatabaseBuilt>, mut next: ResMut<NextState<LoadState>>) {
    if built.get() {
        return;
    }
    if !server.local_db_exists() {
        if !server.build_local_db() {
            panic!("Failed to build local db");
        }
        info!("Local database built!");
    }
    built.set(true);
    next.set(LoadState::Connecting);
}
