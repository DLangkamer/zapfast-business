//! ZapFast Server implementation.
//!
//! Provides the headless multi-instance WhatsApp host, Web Admin Dashboard,
//! UDP Local Discovery beacon, and client WebSocket gateway.

pub mod db;
pub mod discovery;
pub mod http;
pub mod manager;
pub mod web_assets;
pub mod ws;

use std::path::PathBuf;
use std::sync::Arc;
use crate::server::db::ServerDb;
use crate::server::discovery::DiscoveryBeacon;
use crate::server::manager::InstanceManager;

#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub data_dir: PathBuf,
    pub http_port: u16,
    pub server_name: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("data"),
            http_port: 8080,
            server_name: "ZapFast Docker Server".to_string(),
        }
    }
}

pub async fn run_server_main(config: ServerConfig) -> Result<(), Box<dyn std::error::Error>> {
    let _ = std::fs::create_dir_all(&config.data_dir);
    let db_path = config.data_dir.join("server.db");
    let db = ServerDb::open(&db_path)?;
    log::info!("ZapFast Server database opened at {:?}", db_path);

    let manager = InstanceManager::new(&config.data_dir, db.clone());
    manager.load_all();

    // Start UDP beacon for local network auto-discovery
    let beacon = Arc::new(DiscoveryBeacon::new(&config.server_name, config.http_port));
    beacon.start();

    // Start HTTP & WebSocket server
    http::run_server(config.http_port, db, manager).await
}
