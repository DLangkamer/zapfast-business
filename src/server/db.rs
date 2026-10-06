//! Database management for ZapFast Server.
//!
//! Stores users, password hashes, sessions, WhatsApp instances, and user permissions.

use std::path::Path;
use std::sync::{Arc, Mutex};
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub role: String, // "admin" or "operator"
    pub created_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WhatsAppInstanceRow {
    pub id: String,
    pub name: String,
    pub phone: Option<String>,
    pub status: String,
    pub created_at: i64,
}

#[derive(Clone)]
pub struct ServerDb {
    conn: Arc<Mutex<Connection>>,
}

impl ServerDb {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;

             CREATE TABLE IF NOT EXISTS users (
                 id TEXT PRIMARY KEY,
                 username TEXT UNIQUE NOT NULL,
                 password_hash TEXT NOT NULL,
                 salt TEXT NOT NULL,
                 role TEXT NOT NULL DEFAULT 'operator',
                 created_at INTEGER NOT NULL
             );

             CREATE TABLE IF NOT EXISTS whatsapp_instances (
                 id TEXT PRIMARY KEY,
                 name TEXT NOT NULL,
                 phone TEXT,
                 status TEXT NOT NULL DEFAULT 'unlinked',
                 created_at INTEGER NOT NULL
             );

             CREATE TABLE IF NOT EXISTS user_permissions (
                 user_id TEXT NOT NULL,
                 account_id TEXT NOT NULL,
                 PRIMARY KEY (user_id, account_id),
                 FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE,
                 FOREIGN KEY(account_id) REFERENCES whatsapp_instances(id) ON DELETE CASCADE
             );

             CREATE TABLE IF NOT EXISTS sessions (
                 token TEXT PRIMARY KEY,
                 user_id TEXT NOT NULL,
                 expires_at INTEGER NOT NULL,
                 FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
             );"
        )?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        db.ensure_default_admin()?;
        Ok(db)
    }

    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS users (
                 id TEXT PRIMARY KEY,
                 username TEXT UNIQUE NOT NULL,
                 password_hash TEXT NOT NULL,
                 salt TEXT NOT NULL,
                 role TEXT NOT NULL DEFAULT 'operator',
                 created_at INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS whatsapp_instances (
                 id TEXT PRIMARY KEY,
                 name TEXT NOT NULL,
                 phone TEXT,
                 status TEXT NOT NULL DEFAULT 'unlinked',
                 created_at INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS user_permissions (
                 user_id TEXT NOT NULL,
                 account_id TEXT NOT NULL,
                 PRIMARY KEY (user_id, account_id)
             );
             CREATE TABLE IF NOT EXISTS sessions (
                 token TEXT PRIMARY KEY,
                 user_id TEXT NOT NULL,
                 expires_at INTEGER NOT NULL
             );"
        )?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.ensure_default_admin()?;
        Ok(db)
    }

    fn ensure_default_admin(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))?;
        if count == 0 {
            let salt = generate_random_token();
            let hash = hash_password("admin123", &salt);
            let id = format!("u_{}", generate_random_token()[..8].to_string());
            let now = crate::util::now();
            conn.execute(
                "INSERT INTO users (id, username, password_hash, salt, role, created_at)
                 VALUES (?1, ?2, ?3, ?4, 'admin', ?5)",
                params![id, "admin", hash, salt, now],
            )?;
            log::info!("Created default administrator account: username='admin', password='admin123'");
        }
        Ok(())
    }

    pub fn authenticate(&self, username: &str, password: &str) -> Result<Option<User>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, username, password_hash, salt, role, created_at FROM users WHERE username = ?1"
        )?;
        let user_opt = stmt.query_row(params![username], |row| {
            let id: String = row.get(0)?;
            let uname: String = row.get(1)?;
            let stored_hash: String = row.get(2)?;
            let salt: String = row.get(3)?;
            let role: String = row.get(4)?;
            let created_at: i64 = row.get(5)?;
            Ok((id, uname, stored_hash, salt, role, created_at))
        });

        match user_opt {
            Ok((id, uname, stored_hash, salt, role, created_at)) => {
                let computed_hash = hash_password(password, &salt);
                if computed_hash == stored_hash {
                    Ok(Some(User {
                        id,
                        username: uname,
                        role,
                        created_at,
                    }))
                } else {
                    Ok(None)
                }
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn create_user(&self, username: &str, password: &str, role: &str) -> Result<User> {
        let conn = self.conn.lock().unwrap();
        let salt = generate_random_token();
        let hash = hash_password(password, &salt);
        let id = format!("u_{}", generate_random_token()[..8].to_string());
        let now = crate::util::now();
        conn.execute(
            "INSERT INTO users (id, username, password_hash, salt, role, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, username, hash, salt, role, now],
        )?;
        Ok(User {
            id,
            username: username.to_string(),
            role: role.to_string(),
            created_at: now,
        })
    }

    pub fn list_users(&self) -> Result<Vec<User>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, username, role, created_at FROM users ORDER BY username ASC")?;
        let rows = stmt.query_map([], |row| {
            Ok(User {
                id: row.get(0)?,
                username: row.get(1)?,
                role: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        let mut users = Vec::new();
        for r in rows {
            users.push(r?);
        }
        Ok(users)
    }

    pub fn delete_user(&self, user_id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute("DELETE FROM users WHERE id = ?1", params![user_id])?;
        Ok(rows > 0)
    }

    pub fn list_whatsapp_instances(&self) -> Result<Vec<WhatsAppInstanceRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, phone, status, created_at FROM whatsapp_instances ORDER BY created_at ASC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(WhatsAppInstanceRow {
                id: row.get(0)?,
                name: row.get(1)?,
                phone: row.get(2)?,
                status: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn upsert_whatsapp_instance(&self, id: &str, name: &str, phone: Option<&str>, status: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = crate::util::now();
        conn.execute(
            "INSERT INTO whatsapp_instances (id, name, phone, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET name = excluded.name, phone = coalesce(excluded.phone, whatsapp_instances.phone), status = excluded.status",
            params![id, name, phone, status, now],
        )?;
        Ok(())
    }

    pub fn delete_whatsapp_instance(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute("DELETE FROM whatsapp_instances WHERE id = ?1", params![id])?;
        Ok(rows > 0)
    }

    pub fn get_user_permissions(&self, user_id: &str) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        // Check role first: admin has access to all instances
        let role: Result<String> = conn.query_row("SELECT role FROM users WHERE id = ?1", params![user_id], |r| r.get(0));
        if let Ok(role) = role && role == "admin" {
            let mut stmt = conn.prepare("SELECT id FROM whatsapp_instances")?;
            let rows = stmt.query_map([], |r| r.get(0))?;
            let mut all = Vec::new();
            for r in rows {
                all.push(r?);
            }
            return Ok(all);
        }

        let mut stmt = conn.prepare("SELECT account_id FROM user_permissions WHERE user_id = ?1")?;
        let rows = stmt.query_map(params![user_id], |r| r.get(0))?;
        let mut allowed = Vec::new();
        for r in rows {
            allowed.push(r?);
        }
        Ok(allowed)
    }

    pub fn set_user_permissions(&self, user_id: &str, account_ids: &[String]) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM user_permissions WHERE user_id = ?1", params![user_id])?;
        for acc in account_ids {
            tx.execute(
                "INSERT OR IGNORE INTO user_permissions (user_id, account_id) VALUES (?1, ?2)",
                params![user_id, acc],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn create_session(&self, user_id: &str) -> Result<String> {
        let conn = self.conn.lock().unwrap();
        let token = generate_random_token();
        // 7 days expiration
        let expires_at = crate::util::now() + (7 * 24 * 3600);
        conn.execute(
            "INSERT INTO sessions (token, user_id, expires_at) VALUES (?1, ?2, ?3)",
            params![token, user_id, expires_at],
        )?;
        Ok(token)
    }

    pub fn validate_session(&self, token: &str) -> Result<Option<User>> {
        let conn = self.conn.lock().unwrap();
        let now = crate::util::now();
        let mut stmt = conn.prepare(
            "SELECT u.id, u.username, u.role, u.created_at
             FROM sessions s
             JOIN users u ON u.id = s.user_id
             WHERE s.token = ?1 AND s.expires_at > ?2"
        )?;
        let user_opt = stmt.query_row(params![token, now], |row| {
            Ok(User {
                id: row.get(0)?,
                username: row.get(1)?,
                role: row.get(2)?,
                created_at: row.get(3)?,
            })
        });

        match user_opt {
            Ok(user) => Ok(Some(user)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

pub fn hash_password(password: &str, salt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(b":");
    hasher.update(password.as_bytes());
    let result = hasher.finalize();
    result.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn generate_random_token() -> String {
    let mut bytes = [0u8; 16];
    let _ = getrandom::fill(&mut bytes);
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
