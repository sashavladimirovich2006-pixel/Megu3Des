// Release builds are GUI apps: no console window on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ipc;

use std::sync::Mutex;

use megu3d_cmd::dispatch::Session;
use tracing_subscriber::EnvFilter;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("MEGU3D_LOG").unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        schema = megu3d_core::SCHEMA_VERSION,
        "starting Megu3D"
    );

    tauri::Builder::default()
        // One session per window process; the mutex makes the core the sole
        // mutator even when several IPC calls arrive at once.
        .manage(Mutex::new(Session::new()))
        .invoke_handler(tauri::generate_handler![
            ipc::megu3d_query_app_info,
            ipc::megu3d_query_scene_stats,
            ipc::megu3d_query_scene,
            ipc::megu3d_cmd_dispatch,
        ])
        .run(tauri::generate_context!())
        .expect("Megu3D failed to start");
}
