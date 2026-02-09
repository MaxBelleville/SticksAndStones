use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use std::vec;

use bevy::log::info;

pub struct LocalServer {
    _process: Child,
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
        while TcpStream::connect("127.0.0.1:3000").is_err() {
            if start.elapsed() > timeout {
                panic!("SpacetimeDB failed to start within 5 seconds.");
            }
            std::thread::sleep(Duration::from_millis(100));
        }

        info!("Server is live!");
        Self { _process: child }
    }
    pub fn stop(&mut self) {
        self._process.kill().unwrap();
    }
    pub fn is_running(&mut self) -> bool {
        self._process.try_wait().unwrap().is_none()
    }
    pub fn restart(&mut self, is_single_player: bool) -> Self {
        self.stop();
        Self::start(is_single_player)
    }
    pub fn build_dbs(&self) {
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
                "Failed to find 'spacetimedb.wasm'. In the same folder as your exe",
            );
        info!("Database built!");
    }
}
