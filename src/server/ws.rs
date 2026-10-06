//! WebSocket gateway for ZapFast Server.
//!
//! Handles real-time client communication with Desktop apps and the Web UI.

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio_websockets::{Message, WebSocketStream};

use crate::backend::Command;
use crate::server::db::{ServerDb, User};
use crate::server::manager::InstanceManager;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ClientWsMessage {
    #[serde(rename = "auth")]
    Auth {
        token: Option<String>,
        username: Option<String>,
        password: Option<String>,
    },
    #[serde(rename = "subscribe")]
    Subscribe {
        instance_id: String,
    },
    #[serde(rename = "send_text")]
    SendText {
        instance_id: String,
        chat: String,
        text: String,
        quoting: Option<String>,
    },
    #[serde(rename = "mark_read")]
    MarkRead {
        instance_id: String,
        chat: String,
    },
    #[serde(rename = "delete_for_everyone")]
    DeleteForEveryone {
        instance_id: String,
        chat: String,
        id: String,
    },
    #[serde(rename = "ping")]
    Ping,
}

pub async fn handle_ws_stream(
    mut ws: WebSocketStream<TcpStream>,
    db: ServerDb,
    manager: InstanceManager,
) {
    let mut authenticated_user: Option<User> = None;
    let mut allowed_instances: Vec<String> = Vec::new();
    let mut subscribed_instances: Vec<String> = Vec::new();
    let mut bcast_rx = manager.subscribe();

    loop {
        tokio::select! {
            // Incoming broadcast event from WhatsApp instances
            Ok(bcast_msg) = bcast_rx.recv() => {
                if let Some(user) = &authenticated_user {
                    let has_access = user.role == "admin" || allowed_instances.contains(&bcast_msg.instance_id);
                    if has_access && (subscribed_instances.is_empty() || subscribed_instances.contains(&bcast_msg.instance_id)) {
                        let json = serde_json::to_string(&serde_json::json!({
                            "type": "event",
                            "instance_id": bcast_msg.instance_id,
                            "event_type": bcast_msg.event_type,
                            "payload": bcast_msg.payload,
                        })).unwrap_or_default();
                        let _ = ws.send(Message::text(json)).await;
                    }
                }
            }
            // Incoming message from client
            msg_opt = ws.next() => {
                let Some(msg_res) = msg_opt else { break; };
                let Ok(msg) = msg_res else { break; };

                if msg.is_close() {
                    break;
                }

                if let Some(text) = msg.as_text() {
                    let parsed: Result<ClientWsMessage, _> = serde_json::from_str(text);
                    match parsed {
                        Ok(ClientWsMessage::Auth { token, username, password }) => {
                            let user = if let Some(tok) = token {
                                db.validate_session(&tok).unwrap_or(None)
                            } else if let (Some(u), Some(p)) = (username, password) {
                                db.authenticate(&u, &p).unwrap_or(None)
                            } else {
                                None
                            };

                            if let Some(u) = user {
                                let perms = db.get_user_permissions(&u.id).unwrap_or_default();
                                allowed_instances = perms.clone();
                                let resp = serde_json::json!({
                                    "type": "auth_ok",
                                    "user": {
                                        "id": u.id,
                                        "username": u.username,
                                        "role": u.role,
                                    },
                                    "allowed_instances": perms,
                                    "all_instances": manager.list_instances(),
                                });
                                authenticated_user = Some(u);
                                let _ = ws.send(Message::text(resp.to_string())).await;
                            } else {
                                let resp = serde_json::json!({
                                    "type": "auth_error",
                                    "error": "Credenciais inválidas"
                                });
                                let _ = ws.send(Message::text(resp.to_string())).await;
                            }
                        }
                        Ok(ClientWsMessage::Subscribe { instance_id }) => {
                            if let Some(user) = &authenticated_user {
                                if user.role == "admin" || allowed_instances.contains(&instance_id) {
                                    if !subscribed_instances.contains(&instance_id) {
                                        subscribed_instances.push(instance_id.clone());
                                    }
                                    let _ = ws.send(Message::text(serde_json::json!({
                                        "type": "subscribed",
                                        "instance_id": instance_id
                                    }).to_string())).await;
                                } else {
                                    let _ = ws.send(Message::text(serde_json::json!({
                                        "type": "error",
                                        "error": "Permissão negada para esta conta"
                                    }).to_string())).await;
                                }
                            }
                        }
                        Ok(ClientWsMessage::SendText { instance_id, chat, text, quoting }) => {
                            if let Some(user) = &authenticated_user {
                                if user.role == "admin" || allowed_instances.contains(&instance_id) {
                                    manager.send_command(&instance_id, Command::SendText {
                                        chat,
                                        text,
                                        quoting,
                                        mentions: Vec::new(),
                                    });
                                }
                            }
                        }
                        Ok(ClientWsMessage::MarkRead { instance_id, chat }) => {
                            if let Some(user) = &authenticated_user {
                                if user.role == "admin" || allowed_instances.contains(&instance_id) {
                                    manager.send_command(&instance_id, Command::MarkRead { chat, receipts: true });
                                }
                            }
                        }
                        Ok(ClientWsMessage::DeleteForEveryone { instance_id, chat, id }) => {
                            if let Some(user) = &authenticated_user {
                                if user.role == "admin" || allowed_instances.contains(&instance_id) {
                                    manager.send_command(&instance_id, Command::Revoke { chat, id });
                                }
                            }
                        }
                        Ok(ClientWsMessage::Ping) => {
                            let _ = ws.send(Message::text(r#"{"type":"pong"}"#)).await;
                        }
                        Err(e) => {
                            log::debug!("Unrecognized WS message: {e}");
                        }
                    }
                }
            }
        }
    }
}
