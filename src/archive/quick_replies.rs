//! Cached WhatsApp Business quick replies.

use rusqlite::params;

use super::{Archive, Result};
use crate::model::QuickReply;

pub const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS business_quick_replies (
    id TEXT PRIMARY KEY,
    shortcut TEXT NOT NULL,
    message TEXT NOT NULL,
    keywords TEXT NOT NULL,
    count INTEGER NOT NULL DEFAULT 0
);
";

impl Archive {
    pub fn quick_replies(&self) -> Result<Vec<QuickReply>> {
        let mut statement = self.connection.prepare(
            "SELECT id, shortcut, message, keywords, count
             FROM business_quick_replies
             ORDER BY count DESC, lower(shortcut), id",
        )?;
        let rows = statement.query_map([], |row| {
            let keywords: String = row.get(3)?;
            Ok(QuickReply {
                id: row.get(0)?,
                shortcut: row.get(1)?,
                message: row.get(2)?,
                keywords: keywords
                    .split('\u{1f}')
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned)
                    .collect(),
                count: row.get(4)?,
            })
        })?;
        rows.collect()
    }

    pub fn upsert_quick_reply(&self, reply: &QuickReply) -> Result<()> {
        self.connection.execute(
            "INSERT INTO business_quick_replies (id, shortcut, message, keywords, count)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET shortcut=excluded.shortcut,
                 message=excluded.message, keywords=excluded.keywords, count=excluded.count",
            params![
                reply.id,
                reply.shortcut,
                reply.message,
                reply.keywords.join("\u{1f}"),
                reply.count
            ],
        )?;
        Ok(())
    }

    pub fn delete_quick_reply(&self, id: &str) -> Result<bool> {
        Ok(self.connection.execute(
            "DELETE FROM business_quick_replies WHERE id = ?1",
            params![id],
        )? > 0)
    }
}
