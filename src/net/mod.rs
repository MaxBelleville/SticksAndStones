use bevy::prelude::*;
mod server;
mod client;
mod stdb_bindings;

pub struct NetPlugin;
impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        let server = server::LocalServer::start(true);
        server.build_dbs();
        app.add_plugins(client::ClientPlugin);
    }
}