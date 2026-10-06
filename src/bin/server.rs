//! ZapFast Server binary for Docker / server deployment.

use std::path::PathBuf;
use clap::Parser;
use zapfast::server::{run_server_main, ServerConfig};

#[derive(Parser, Debug)]
#[command(name = "zapfast-server", about = "ZapFast Multi-Account Docker/Headless Server")]
struct Cli {
    /// HTTP and WebSocket port to listen on
    #[arg(short, long, default_value_t = 8080)]
    port: u16,

    /// Directory for server data and WhatsApp account sessions
    #[arg(short, long, default_value = "data")]
    data_dir: PathBuf,

    /// Name of the server advertised on local network
    #[arg(long, default_value = "ZapFast Docker Server")]
    name: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = fastframe_log::Logging::new("zapfast-server", env!("CARGO_PKG_VERSION")).init();

    let cli = Cli::parse();
    log::info!("Starting ZapFast Server on port {}", cli.port);
    log::info!("Data directory: {:?}", cli.data_dir);

    let config = ServerConfig {
        data_dir: cli.data_dir,
        http_port: cli.port,
        server_name: cli.name,
    };

    run_server_main(config).await
}
