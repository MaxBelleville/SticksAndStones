use bevy::prelude::*;
use bevy_spacetimedb::ReadStdbConnectedMessage;
use bevy_spacetimedb::StdbConnection;

use bevy_spacetimedb::StdbPlugin;
use bevy_spacetimedb::connect_with_token;
use super::stdb_bindings::DbConnection;
use super::stdb_bindings::message_table::MessageTableAccess;   
use super::stdb_bindings::user_table::UserTableAccess;                                                                                                 
use super::stdb_bindings::{RemoteModule,RemoteTables};
use crate::sns_prelude::*;
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
            .with_delayed_connect(true) 
            .add_table(RemoteTables::message)
            .add_table(RemoteTables::user),
        ).add_systems(OnEnter(LoadState::Connecting), connect_to_db)
        .add_systems(Update, on_connected.run_if(in_state(LoadState::Connecting)));
    }
}

fn connect_to_db(world: &mut World) {
    info!("Connecting to SpacetimeDB…");
    connect_with_token::<DbConnection, RemoteModule>(world, None);
}

fn on_connected(
    mut messages: ReadStdbConnectedMessage,
    mut next: ResMut<NextState<LoadState>>,
) {
    for _ev in messages.read() {
        info!("Connected to SpacetimeDB");
        next.set(LoadState::LoadAssets);
    }
}