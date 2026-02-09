use bevy::prelude::*;
use bevy_spacetimedb::StdbConnection;

use bevy_spacetimedb::StdbPlugin;
use super::stdb_bindings::DbConnection;
use super::stdb_bindings::message_table::MessageTableAccess;   
use super::stdb_bindings::user_table::UserTableAccess;                                                                                                 
use super::stdb_bindings::{RemoteModule,RemoteTables};

pub struct ClientPlugin;
pub type SpacetimeDB<'a> = Res<'a, StdbConnection<DbConnection>>;

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        //add spacetime db client
       app.add_plugins(
            StdbPlugin::default()
            .with_uri("localhost:3000")
            .with_module_name("sticks-and-stones")
            .with_run_fn(DbConnection::run_threaded)
            .add_table(RemoteTables::message)
            .add_table(RemoteTables::user),
        );
    }
}
