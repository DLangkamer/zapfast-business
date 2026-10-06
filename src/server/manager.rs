//! WhatsApp Instance Manager for ZapFast Server.
//!
//! Manages multiple headless WhatsApp connections, routes commands from permitted users,
//! and broadcasts events to Desktop clients and the Web UI.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::backend::{Backend, Command, Event, LinkStatus, Waker};
use crate::paths::AccountDirs;
use crate::server::db::ServerDb;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstanceInfo {
    pub id: String,
    pub name: String,
    pub phone: Option<String>,
    pub status: String,
    pub qr: Option<String>,
    pub connected: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BroadcastMessage {
    pub instance_id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
}

struct ManagedInstance {
    id: String,
    name: String,
    backend: Arc<Mutex<Backend>>,
    phone: Arc<Mutex<Option<String>>>,
    status: Arc<Mutex<String>>,
    qr: Arc<Mutex<Option<String>>>,
    connected: Arc<Mutex<bool>>,
    stop_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

#[derive(Clone)]
pub struct InstanceManager {
    data_dir: PathBuf,
    db: ServerDb,
    instances: Arc<RwLock<HashMap<String, Arc<ManagedInstance>>>>,
    broadcast_tx: broadcast::Sender<BroadcastMessage>,
}

impl InstanceManager {
    pub fn new(data_dir: impl AsRef<Path>, db: ServerDb) -> Self {
        let (broadcast_tx, _) = broadcast::channel(1024);
        Self {
            data_dir: data_dir.as_ref().to_path_buf(),
            db,
            instances: Arc::new(RwLock::new(HashMap::new())),
            broadcast_tx,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<BroadcastMessage> {
        self.broadcast_tx.subscribe()
    }

    /// Load and spawn all instances saved in the database.
    pub fn load_all(&self) {
        let instances = match self.db.list_whatsapp_instances() {
            Ok(list) => list,
            Err(e) => {
                log::error!("Failed to list WhatsApp instances from db: {e}");
                return;
            }
        };

        for row in instances {
            self.start_instance(&row.id, &row.name);
        }
    }

    pub fn start_instance(&self, id: &str, name: &str) -> bool {
        {
            let map = self.instances.read().unwrap();
            if map.contains_key(id) {
                return true;
            }
        }

        let instance_dir = self.data_dir.join("accounts").join(id);
        let _ = std::fs::create_dir_all(&instance_dir);
        let dirs = AccountDirs::under(&instance_dir);

        let mut backend = Backend::spawn(dirs, Waker::default());
        if let Some(startup) = backend.take_startup() {
            let _ = startup.send(());
        }

        let backend = Arc::new(Mutex::new(backend));
        let phone = Arc::new(Mutex::new(None));
        let status = Arc::new(Mutex::new("starting".to_string()));
        let qr = Arc::new(Mutex::new(None));
        let connected = Arc::new(Mutex::new(false));

        let (stop_tx, mut stop_rx) = tokio::sync::oneshot::channel::<()>();

        // Background poller for this instance
        let b_clone = Arc::clone(&backend);
        let p_clone = Arc::clone(&phone);
        let s_clone = Arc::clone(&status);
        let q_clone = Arc::clone(&qr);
        let c_clone = Arc::clone(&connected);
        let inst_id = id.to_string();
        let inst_name = name.to_string();
        let bcast = self.broadcast_tx.clone();
        let db_clone = self.db.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(80));
            loop {
                tokio::select! {
                    _ = &mut stop_rx => {
                        log::info!("Stopping poller for WhatsApp instance {inst_id}");
                        break;
                    }
                    _ = interval.tick() => {
                        let events = {
                            let b = b_clone.lock().unwrap();
                            b.poll()
                        };

                        for event in events {
                            Self::handle_instance_event(
                                &inst_id,
                                &inst_name,
                                &event,
                                &p_clone,
                                &s_clone,
                                &q_clone,
                                &c_clone,
                                &bcast,
                                &db_clone,
                            );
                        }
                    }
                }
            }
        });

        let managed = Arc::new(ManagedInstance {
            id: id.to_string(),
            name: name.to_string(),
            backend,
            phone,
            status,
            qr,
            connected,
            stop_tx: Some(stop_tx),
        });

        let mut map = self.instances.write().unwrap();
        map.insert(id.to_string(), managed);
        let _ = self.db.upsert_whatsapp_instance(id, name, None, "starting");
        true
    }

    fn handle_instance_event(
        inst_id: &str,
        inst_name: &str,
        event: &Event,
        phone: &Arc<Mutex<Option<String>>>,
        status: &Arc<Mutex<String>>,
        qr: &Arc<Mutex<Option<String>>>,
        connected: &Arc<Mutex<bool>>,
        bcast: &broadcast::Sender<BroadcastMessage>,
        db: &ServerDb,
    ) {
        match event {
            Event::Link(link_status) => {
                let mut s = status.lock().unwrap();
                let mut q = qr.lock().unwrap();
                let mut c = connected.lock().unwrap();
                match link_status {
                    LinkStatus::Starting => {
                        *s = "starting".to_string();
                        *c = false;
                    }
                    LinkStatus::Unlinked { qr: new_qr, .. } => {
                        *s = "unlinked".to_string();
                        *q = new_qr.clone();
                        *c = false;
                    }
                    LinkStatus::Connecting => {
                        *s = "connecting".to_string();
                        *c = false;
                    }
                    LinkStatus::Connected => {
                        *s = "connected".to_string();
                        *q = None;
                        *c = true;
                    }
                    LinkStatus::Disconnected { reason } => {
                        *s = format!("disconnected: {reason}");
                        *c = false;
                    }
                    LinkStatus::LoggedOut => {
                        *s = "logged_out".to_string();
                        *q = None;
                        *c = false;
                    }
                    LinkStatus::Failed(err) => {
                        *s = format!("failed: {err}");
                        *c = false;
                    }
                }
                let current_phone = phone.lock().unwrap().clone();
                let _ = db.upsert_whatsapp_instance(inst_id, inst_name, current_phone.as_deref(), &s);
                let _ = bcast.send(BroadcastMessage {
                    instance_id: inst_id.to_string(),
                    event_type: "link_status".to_string(),
                    payload: serde_json::json!({
                        "status": *s,
                        "qr": *q,
                        "connected": *c,
                    }),
                });
            }
            Event::Me { id: my_id, name: my_name, .. } => {
                let mut p = phone.lock().unwrap();
                let phone_num = my_id.split('@').next().unwrap_or(my_id).to_string();
                *p = Some(phone_num.clone());
                let cur_status = status.lock().unwrap().clone();
                let display_name = my_name.clone().unwrap_or_else(|| inst_name.to_string());
                let _ = db.upsert_whatsapp_instance(inst_id, &display_name, Some(&phone_num), &cur_status);
                let _ = bcast.send(BroadcastMessage {
                    instance_id: inst_id.to_string(),
                    event_type: "me".to_string(),
                    payload: serde_json::json!({
                        "phone": phone_num,
                        "name": my_name,
                    }),
                });
            }
            Event::Incoming { chat, message } => {
                let _ = bcast.send(BroadcastMessage {
                    instance_id: inst_id.to_string(),
                    event_type: "incoming_message".to_string(),
                    payload: serde_json::json!({
                        "chat": chat,
                        "message": message,
                    }),
                });
            }
            Event::MessageUpdated(message) => {
                let _ = bcast.send(BroadcastMessage {
                    instance_id: inst_id.to_string(),
                    event_type: "message_updated".to_string(),
                    payload: serde_json::json!({
                        "message": message,
                    }),
                });
            }
            Event::ChatUpdated(chat) => {
                let _ = bcast.send(BroadcastMessage {
                    instance_id: inst_id.to_string(),
                    event_type: "chat_updated".to_string(),
                    payload: serde_json::json!({
                        "chat": chat,
                    }),
                });
            }
            Event::Chats(chats) => {
                let _ = bcast.send(BroadcastMessage {
                    instance_id: inst_id.to_string(),
                    event_type: "chats".to_string(),
                    payload: serde_json::json!({
                        "chats": chats,
                    }),
                });
            }
            _ => {}
        }
    }

    pub fn list_instances(&self) -> Vec<InstanceInfo> {
        let map = self.instances.read().unwrap();
        let mut list = Vec::new();
        for (_, inst) in map.iter() {
            list.push(InstanceInfo {
                id: inst.id.clone(),
                name: inst.name.clone(),
                phone: inst.phone.lock().unwrap().clone(),
                status: inst.status.lock().unwrap().clone(),
                qr: inst.qr.lock().unwrap().clone(),
                connected: *inst.connected.lock().unwrap(),
            });
        }
        list
    }

    pub fn get_instance(&self, id: &str) -> Option<InstanceInfo> {
        let map = self.instances.read().unwrap();
        map.get(id).map(|inst| InstanceInfo {
            id: inst.id.clone(),
            name: inst.name.clone(),
            phone: inst.phone.lock().unwrap().clone(),
            status: inst.status.lock().unwrap().clone(),
            qr: inst.qr.lock().unwrap().clone(),
            connected: *inst.connected.lock().unwrap(),
        })
    }

    pub fn send_command(&self, id: &str, command: Command) -> bool {
        let map = self.instances.read().unwrap();
        if let Some(inst) = map.get(id) {
            let b = inst.backend.lock().unwrap();
            b.send(command);
            true
        } else {
            false
        }
    }

    pub fn remove_instance(&self, id: &str) -> bool {
        let mut map = self.instances.write().unwrap();
        if let Some(mut inst) = map.remove(id) {
            if let Some(mut managed) = Arc::get_mut(&mut inst) {
                if let Some(stop) = managed.stop_tx.take() {
                    let _ = stop.send(());
                }
            }
            {
                let mut b = inst.backend.lock().unwrap();
                b.shutdown();
            }
            let _ = self.db.delete_whatsapp_instance(id);
            true
        } else {
            false
        }
    }
}
