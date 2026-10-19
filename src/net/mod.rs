mod server;
mod stdb_bindings;
use bevy::prelude::*;

use bevy_stdb::prelude::*;

use server::{DatabaseProcessor, LocalServer, is_port_open};
use stdb_bindings::{DbConnection, RemoteModule};
use stdb_bindings::{User, UserTableAccess, userQueryTableAccess};

pub struct ClientPlugin;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum SNSSubKey {
    User,
}

pub type StdbConn = StdbConnection<DbConnection>;
pub type StdbSubs = StdbSubscriptions<SNSSubKey, RemoteModule>;

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
pub enum ClientState {
    #[default]
    NotConnected,
    StartLocal,
    ConnectIP,
    BuildBridge,
    Connected,
}

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        //add spacetime db client
        app.init_state::<ClientState>()
            .init_resource::<DatabaseProcessor>()
            .add_systems(OnEnter(ClientState::StartLocal), start_local_server)
            .add_systems(OnEnter(ClientState::BuildBridge), start_bevy_stdb)
            .add_systems(
                Update,
                (
                    wait_for_server_ready.run_if(
                        in_state(ClientState::StartLocal).or(in_state(ClientState::ConnectIP)),
                    ),
                    (subscribe_on_connect, on_user_insert),
                ),
            )
            .add_plugins(
                StdbPlugin::<DbConnection, RemoteModule>::default()
                    .with_module_name("sticks-and-stones")
                    .with_uri("127.0.0.1:3000")
                    .add_table::<User>(|reg, db| reg.bind(db.user()))
                    .with_subscriptions::<SNSSubKey>()
                    .with_reconnect(StdbReconnectOptions::default())
                    .with_delayed_connection()
                    .with_background_driver(DbConnection::run_threaded),
            );
    }
}

fn start_local_server(mut commands: Commands) {
    let server = LocalServer::start(true);
    commands.insert_resource(server);
}

fn wait_for_server_ready(
    opt_server: Option<Res<LocalServer>>,

    mut db: ResMut<DatabaseProcessor>,
    mut next: ResMut<NextState<ClientState>>,
    mut ticks: Local<i32>,
) {
    // For a local server we need the process to exist before polling.
    if opt_server.is_none() {
        return;
    }

    if is_port_open("127.0.0.1:3000") {
        info!("Server is live");

        // Only publish the DB when connecting locally.
        if !db.exists() {
            info!("Database does not exist, publishing local db");
            if !db.publish_db() {
                panic!("Failed to publish local db");
            }
        }
        info!("Server is ready building bevy stdb bridge");
        next.set(ClientState::BuildBridge);
        return;
    }

    // Timeout guard — panic after ~100 frames of waiting.
    if *ticks >= 100 {
        panic!("Timed out waiting for server");
    }
    *ticks += 1;
}

fn start_bevy_stdb(mut request: WriteRequestStdbConnectionMessage) {
    request.write(RequestStdbConnectionMessage {
        uri: Some("127.0.0.1:3000".to_string()),
        module_name: Some("sticks-and-stones".to_string()),
        token: None,
    });
}

fn subscribe_on_connect(mut connected: ReadStdbConnectedMessage, mut subs: ResMut<StdbSubs>) {
    if connected.read().next().is_some() {
        subs.subscribe_query(SNSSubKey::User, |q| q.from.user());
    }
}

fn on_user_insert(mut msgs: ReadInsertMessage<User>, mut next: ResMut<NextState<ClientState>>) {
    for msg in msgs.read() {
        info!("Connected player inserted: {:?}", msg.row);
        next.set(ClientState::Connected);
    }
}
