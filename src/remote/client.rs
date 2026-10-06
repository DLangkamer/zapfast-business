//! Remote Server WebSocket Client for ZapFast Desktop.
//!
//! Connects to a ZapFast Server running in Docker or on the local network,
//! authenticates with user credentials, and forwards events to the desktop UI.

use std::sync::{Arc, Mutex};
use std::time::Duration;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_websockets::{ClientBuilder, Message};

use crate::server::manager::InstanceInfo;
use crate::server::ws::ClientWsMessage;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoteUser {
    pub id: String,
    pub username: String,
    pub role: String,
}

#[derive(Clone, Debug)]
pub enum RemoteState {
    Disconnected,
    Connecting,
    Connected {
        user: RemoteUser,
        allowed_instances: Vec<String>,
        instances: Vec<InstanceInfo>,
    },
    AuthFailed(String),
    Error(String),
}

pub struct RemoteClient {
    state: Arc<Mutex<RemoteState>>,
    command_tx: Option<mpsc::UnboundedSender<ClientWsMessage>>,
    stop_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl RemoteClient {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(RemoteState::Disconnected)),
            command_tx: None,
            stop_tx: None,
        }
    }

    pub fn state(&self) -> RemoteState {
        self.state.lock().unwrap().clone()
    }

    pub fn is_connected(&self) -> bool {
        matches!(*self.state.lock().unwrap(), RemoteState::Connected { .. })
    }

    pub fn connect(
        &mut self,
        server_address: String,
        username: String,
        password: String,
    ) {
        self.disconnect();

        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<ClientWsMessage>();
        let (stop_tx, mut stop_rx) = tokio::sync::oneshot::channel::<()>();
        let state_clone = Arc::clone(&self.state);

        *state_clone.lock().unwrap() = RemoteState::Connecting;
        self.command_tx = Some(cmd_tx);
        self.stop_tx = Some(stop_tx);

        tokio::spawn(async move {
            let ws_url = if server_address.starts_with("ws://") || server_address.starts_with("wss://") {
                format!("{}/api/ws", server_address.trim_end_matches('/'))
            } else {
                format!("ws://{}/api/ws", server_address.trim_end_matches('/'))
            };

            let uri = match ws_url.parse::<http::Uri>() {
                Ok(u) => u,
                Err(e) => {
                    *state_clone.lock().unwrap() = RemoteState::Error(format!("URL inválida: {e}"));
                    return;
                }
            };

            log::info!("Connecting to ZapFast Server at {uri}...");
            let ws_stream = match ClientBuilder::from_uri(uri).connect().await {
                Ok((stream, _resp)) => stream,
                Err(e) => {
                    log::warn!("Failed to connect to ZapFast Server: {e}");
                    *state_clone.lock().unwrap() = RemoteState::Error(format!("Falha na conexão: {e}"));
                    return;
                }
            };

            let (mut ws_sink, mut ws_stream) = ws_stream.split();

            // Send authentication message
            let auth_msg = serde_json::to_string(&ClientWsMessage::Auth {
                token: None,
                username: Some(username),
                password: Some(password),
            }).unwrap();

            if let Err(e) = ws_sink.send(Message::text(auth_msg)).await {
                *state_clone.lock().unwrap() = RemoteState::Error(format!("Erro ao autenticar: {e}"));
                return;
            }

            // Read auth response
            let auth_ok = match ws_stream.next().await {
                Some(Ok(msg)) => {
                    if let Some(text) = msg.as_text() {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(text) {
                            if val.get("type").and_then(|v| v.as_str()) == Some("auth_ok") {
                                let u_val = val.get("user").cloned().unwrap_or_default();
                                let user = RemoteUser {
                                    id: u_val.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                    username: u_val.get("username").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                    role: u_val.get("role").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                };
                                let allowed: Vec<String> = val.get("allowed_instances")
                                    .and_then(|v| v.as_array())
                                    .map(|arr| arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
                                    .unwrap_or_default();
                                let instances: Vec<InstanceInfo> = val.get("all_instances")
                                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                                    .unwrap_or_default();

                                *state_clone.lock().unwrap() = RemoteState::Connected {
                                    user,
                                    allowed_instances: allowed,
                                    instances,
                                };
                                true
                            } else {
                                let err = val.get("error").and_then(|v| v.as_str()).unwrap_or("Erro de autenticação");
                                *state_clone.lock().unwrap() = RemoteState::AuthFailed(err.to_string());
                                false
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
                _ => false,
            };

            if !auth_ok {
                return;
            }

            log::info!("Successfully authenticated with ZapFast Server!");

            loop {
                tokio::select! {
                    _ = &mut stop_rx => {
                        let _ = ws_sink.close().await;
                        break;
                    }
                    Some(cmd) = cmd_rx.recv() => {
                        if let Ok(json) = serde_json::to_string(&cmd) {
                            let _ = ws_sink.send(Message::text(json)).await;
                        }
                    }
                    msg_opt = ws_stream.next() => {
                        let Some(msg_res) = msg_opt else { break; };
                        let Ok(msg) = msg_res else { break; };
                        if msg.is_close() { break; }
                        // Handle server broadcast events
                        if let Some(text) = msg.as_text() {
                            log::debug!("Remote event: {text}");
                        }
                    }
                }
            }

            *state_clone.lock().unwrap() = RemoteState::Disconnected;
        });
    }

    pub fn disconnect(&mut self) {
        if let Some(stop) = self.stop_tx.take() {
            let _ = stop.send(());
        }
        self.command_tx = None;
        *self.state.lock().unwrap() = RemoteState::Disconnected;
    }

    pub fn send_text(&self, instance_id: &str, chat: &str, text: &str) {
        if let Some(tx) = &self.command_tx {
            let _ = tx.send(ClientWsMessage::SendText {
                instance_id: instance_id.to_string(),
                chat: chat.to_string(),
                text: text.to_string(),
                quoting: None,
            });
        }
    }
}
