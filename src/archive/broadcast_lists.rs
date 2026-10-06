//! Broadcast lists for mass dispatch and segmented messaging.

use rusqlite::params;

use super::{Archive, Result};
use crate::model::BroadcastList;

pub const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS business_broadcast_lists (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    chats TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
";

impl Archive {
    pub fn broadcast_lists(&self) -> Result<Vec<BroadcastList>> {
        let mut statement = self.connection.prepare(
            "SELECT id, name, chats, created_at
             FROM business_broadcast_lists
             ORDER BY lower(name), created_at DESC",
        )?;
        let rows = statement.query_map([], |row| {
            let chats_json: String = row.get(2)?;
            let chats = serde_json::from_str(&chats_json).unwrap_or_default();
            Ok(BroadcastList {
                id: row.get(0)?,
                name: row.get(1)?,
                chats,
                created_at: row.get(3)?,
            })
        })?;
        rows.collect()
    }

    pub fn upsert_broadcast_list(&self, list: &BroadcastList) -> Result<()> {
        let chats_json = serde_json::to_string(&list.chats).unwrap_or_else(|_| "[]".to_owned());
        self.connection.execute(
            "INSERT INTO business_broadcast_lists (id, name, chats, created_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET name=excluded.name, chats=excluded.chats",
            params![list.id, list.name, chats_json, list.created_at,],
        )?;
        Ok(())
    }

    pub fn delete_broadcast_list(&self, id: &str) -> Result<bool> {
        Ok(self.connection.execute(
            "DELETE FROM business_broadcast_lists WHERE id = ?1",
            params![id],
        )? > 0)
    }
}
