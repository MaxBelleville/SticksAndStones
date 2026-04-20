mod server;
mod stdb_bindings;
use bevy::prelude::*;

use bevy_stdb::prelude::*;

use stdb_bindings::{DbConnection, RemoteModule};

use server::{DatabaseBuilt, LocalServer};

use crate::net::stdb_bindings::User;

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
    ConnectLocal,
    ConnectRemote,
    BuildingLocalDB,
    BuildingRemoteDB,
    Connecting,
    Connected,
}

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        //add spacetime db client
        app.init_state::<ClientState>()
            .init_resource::<DatabaseBuilt>()
            .add_systems(OnEnter(ClientState::StartLocal), start_local_server)
            .add_systems(OnEnter(ClientState::BuildingLocalDB), build_local_db)
            .add_systems(OnEnter(ClientState::Connecting), start_bevy_stdb)
            .add_systems(
                Update,
                (
                    wait_for_local_ready.run_if(
                        in_state(ClientState::StartLocal).or(in_state(ClientState::ConnectLocal)),
                    ),
                    (subscribe_on_connect, on_user_insert),
                ),
            )
            .add_plugins(
                StdbPlugin::<DbConnection, RemoteModule>::default()
                    .with_module_name("sticks-and-stones")
                    .with_uri("http://localhost:3000")
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

fn wait_for_local_ready(
    opt_server: Option<Res<LocalServer>>,
    mut next: ResMut<NextState<ClientState>>,
    mut val: Local<i32>,
) {
    if let Some(server) = opt_server {
        if server.is_ready() {
            info!("Server is live!");
            next.set(ClientState::BuildingLocalDB);
            return;
        }
    }

    //timeout
    if *val >= 100 {
        panic!("Took too long to boot server");
    }
    *val += 1;
}

fn monitor_local_server_health(
    mut server: ResMut<LocalServer>,
    mut next: ResMut<NextState<ClientState>>,
) {
    if !server.is_alive() {
        error!("Server process died unexpectedly!");
        next.set(ClientState::NotConnected);
    }
}

fn build_local_db(
    server: Res<LocalServer>,
    mut built: ResMut<DatabaseBuilt>,
    mut next: ResMut<NextState<ClientState>>,
) {
    if built.get() {
        return;
    }
    if !server.local_db_exists() {
        info!("Database does not exist, building local db");
        if !server.build_local_db() {
            panic!("Failed to build local db");
        }
        info!("Local database built!");
    } else {
        info!("Local database already exists, skipping build")
    }
    built.set(true);
    next.set(ClientState::Connecting);
}

fn start_bevy_stdb(mut request: WriteRequestStdbConnectionMessage) {
    request.write_default();
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
