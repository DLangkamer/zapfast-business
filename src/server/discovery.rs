//! Local Network Discovery (UDP Beacon) for ZapFast Server.
//!
//! Listens on UDP port 47120 for desktop client probes and responds with server metadata.

use std::sync::Arc;
use tokio::net::UdpSocket;

pub const DISCOVERY_PORT: u16 = 47120;
pub const PROBE_MESSAGE: &[u8] = b"ZAPFAST_DISCOVER";

pub struct DiscoveryBeacon {
    server_name: String,
    http_port: u16,
}

impl DiscoveryBeacon {
    pub fn new(server_name: impl Into<String>, http_port: u16) -> Self {
        Self {
            server_name: server_name.into(),
            http_port,
        }
    }

    pub fn start(self: Arc<Self>) {
        tokio::spawn(async move {
            let addr = format!("0.0.0.0:{DISCOVERY_PORT}");
            let socket = match UdpSocket::bind(&addr).await {
                Ok(s) => {
                    log::info!("Local discovery beacon listening on UDP {addr}");
                    let _ = s.set_broadcast(true);
                    s
                }
                Err(e) => {
                    log::warn!("Could not bind UDP discovery socket on {addr}: {e}. (Port may be in use)");
                    return;
                }
            };

            let mut buf = [0u8; 512];
            loop {
                match socket.recv_from(&mut buf).await {
                    Ok((len, peer)) => {
                        let msg = &buf[..len];
                        if msg.starts_with(PROBE_MESSAGE) {
                            let response = serde_json::json!({
                                "service": "zapfast-server",
                                "name": self.server_name,
                                "http_port": self.http_port,
                                "version": "0.19.5"
                            });
                            let resp_bytes = response.to_string();
                            let _ = socket.send_to(resp_bytes.as_bytes(), peer).await;
                        }
                    }
                    Err(e) => {
                        log::debug!("Discovery UDP recv error: {e}");
                    }
                }
            }
        });
    }
}
