use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use std::vec;

use bevy::ecs::resource::Resource;
use bevy::log::info;

#[derive(Resource)]
pub struct LocalServer {
    process: Child,
    singeplayer: bool,
}

#[derive(Resource, Default)]
pub struct DatabaseBuilt(bool);
impl DatabaseBuilt  {
    pub fn get(&self) -> bool {
        self.0
    }
    pub fn set(&mut self, val: bool) {
        self.0 = val;
    }
}

impl LocalServer {
    pub fn start(is_single_player: bool) -> Self {
        let exe_path = std::env::current_exe().unwrap();
        let exe_folder = exe_path.parent().unwrap();
        let exe_folder_str = exe_folder.to_str().unwrap();
        let cli_path = format!("{exe_folder_str}/spacetimedb-cli.exe");
        let data_dir = format!("{exe_folder_str}/data");
        let mut args = vec![
            "--root-dir",
            exe_folder_str,
            "start",
            "--listen-addr",
            "127.0.0.1:3000",
            "--in-memory",
            "--data-dir",
            data_dir.as_str(),
        ];
        if !is_single_player {
            args.remove(5); // delete in memory tag
        }

        // 1. Start the engine
        let child: Child = Command::new(cli_path)
            .args(args)
            .stdout(Stdio::null()) // Keeps your console clean
            .spawn()
            .expect(
                "Failed to find 'spacetimedb-cli.exe'. In the same folder as your exe",
            );

        // 2. Poll the port until it's open (Prevents the crash you saw)
        let start = Instant::now();
        let timeout = Duration::from_secs(5);

        info!("Waiting for local SpacetimeDB to boot...");
        Self { process: child, singeplayer: is_single_player }
    }
    //Check if server is done booting
    pub fn is_ready(&self) -> bool {
         let Ok(_) = TcpStream::connect("127.0.0.1:3000") else {
            return false;
        };
        true
    }

    pub fn stop(&mut self) {
        self.process.kill().unwrap();
    }
    //Check if the proccess is alive(doesn't care if server's done booting).
    pub fn is_alive(&mut self) -> bool {
        self.process.try_wait().ok().flatten().is_none()
    }
    pub fn restart(&mut self, is_single_player: bool) -> Self {
        self.stop();
        Self::start(is_single_player)
    }

    pub fn local_db_exists(&self)->bool {
        let exe_path = std::env::current_exe().unwrap();
        let exe_folder = exe_path.parent().unwrap();
        let exe_folder_str = exe_folder.to_str().unwrap();
        let cli_path = format!("{exe_folder_str}/spacetimedb-cli.exe");
            let args = vec![
            "--root-dir",
            exe_folder_str,
            "sql",
            "--server",
            "local",
            "sticks-and-stones",
            "SELECT * FROM user"
            ];
        let child: Child = Command::new(cli_path)
            .args(args)
            .stdout(Stdio::null()) 
            .stderr(Stdio::null())// Keeps your console clean
            .spawn().expect("Fail");
        //Check stdout 
        child.wait_with_output().unwrap().status.success()

    }

    pub fn build_local_db(&self)->bool {
        let exe_path = std::env::current_exe().unwrap();
        let exe_folder = exe_path.parent().unwrap();
        let exe_folder_str = exe_folder.to_str().unwrap();
        let cli_path = format!("{exe_folder_str}/spacetimedb-cli.exe");
        let wasm = format!("{exe_folder_str}/spacetime.wasm");
        let args = vec![
            "--root-dir",
            exe_folder_str,
            "publish",
            "-s",
            "local",
            "--bin-path",
            wasm.as_str(),
            "sticks-and-stones"
        ];


        let child: Child = Command::new(cli_path)
            .args(args)
            .stdout(Stdio::null()) // Keeps your console clean
            .spawn()
            .expect(
                "Failed to find 'spacetime.wasm'. In the same folder as your exe",
            );

        child.wait_with_output().unwrap().status.success()
    }
}
