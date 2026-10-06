//! Local Network Auto-Discovery client for ZapFast Desktop.
//!
//! Sends UDP broadcast probes to locate ZapFast servers running on the LAN.

use std::net::SocketAddr;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;

use crate::server::discovery::{DISCOVERY_PORT, PROBE_MESSAGE};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiscoveredServer {
    pub name: String,
    pub ip: String,
    pub http_port: u16,
    pub version: String,
}

impl DiscoveredServer {
    pub fn address(&self) -> String {
        format!("{}:{}", self.ip, self.http_port)
    }
}

/// Scans the local network for available ZapFast servers.
pub async fn discover_local_servers(timeout_duration: Duration) -> Vec<DiscoveredServer> {
    let socket = match UdpSocket::bind("0.0.0.0:0").await {
        Ok(s) => s,
        Err(e) => {
            log::warn!("Could not bind UDP discovery socket: {e}");
            return Vec::new();
        }
    };

    let _ = socket.set_broadcast(true);
    let broadcast_addr = format!("255.255.255.255:{DISCOVERY_PORT}");

    if let Err(e) = socket.send_to(PROBE_MESSAGE, &broadcast_addr).await {
        log::warn!("Failed to send discovery probe to {broadcast_addr}: {e}");
        return Vec::new();
    }

    let mut discovered = Vec::new();
    let mut buf = [0u8; 1024];
    let deadline = tokio::time::Instant::now() + timeout_duration;

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }

        tokio::select! {
            _ = tokio::time::sleep(remaining) => {
                break;
            }
            res = socket.recv_from(&mut buf) => {
                if let Ok((len, peer)) = res {
                    if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&buf[..len]) {
                        if val.get("service").and_then(|v| v.as_str()) == Some("zapfast-server") {
                            let name = val.get("name").and_then(|v| v.as_str()).unwrap_or("ZapFast Server").to_string();
                            let port = val.get("http_port").and_then(|v| v.as_u64()).unwrap_or(8080) as u16;
                            let ver = val.get("version").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let ip = peer.ip().to_string();

                            if !discovered.iter().any(|d: &DiscoveredServer| d.ip == ip && d.http_port == port) {
                                discovered.push(DiscoveredServer {
                                    name,
                                    ip,
                                    http_port: port,
                                    version: ver,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    discovered
}
