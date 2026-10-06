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
    count INTEGER NOT NULL DEFAULT 0,
    voice BLOB
);
";

impl Archive {
    pub fn quick_replies(&self) -> Result<Vec<QuickReply>> {
        let mut statement = self.connection.prepare(
            "SELECT id, shortcut, message, keywords, count, voice
             FROM business_quick_replies
             ORDER BY count DESC, lower(shortcut), id",
        )?;
        let rows = statement.query_map([], |row| {
            let keywords: String = row.get(3)?;
            let voice_bytes: Option<Vec<u8>> = row.get(5)?;
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
                voice: voice_bytes.map(bytes_to_samples),
            })
        })?;
        rows.collect()
    }

    pub fn upsert_quick_reply(&self, reply: &QuickReply) -> Result<()> {
        let voice_bytes: Option<Vec<u8>> = reply.voice.as_ref().map(|samples| {
            samples
                .iter()
                .flat_map(|sample| sample.to_le_bytes())
                .collect()
        });
        self.connection.execute(
            "INSERT INTO business_quick_replies (id, shortcut, message, keywords, count, voice)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET shortcut=excluded.shortcut,
                 message=excluded.message, keywords=excluded.keywords, count=excluded.count,
                 voice=coalesce(excluded.voice, business_quick_replies.voice)",
            params![
                reply.id,
                reply.shortcut,
                reply.message,
                reply.keywords.join("\u{1f}"),
                reply.count,
                voice_bytes,
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

fn bytes_to_samples(bytes: Vec<u8>) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|part| f32::from_le_bytes(part.try_into().unwrap()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quick_replies_store_text_and_voice_samples() {
        let archive = Archive::in_memory().unwrap();
        let reply = QuickReply {
            id: "1".into(),
            shortcut: "audio1".into(),
            message: "Hello".into(),
            keywords: vec!["intro".into()],
            count: 0,
            voice: Some(vec![0.1, -0.2, 0.5]),
        };
        archive.upsert_quick_reply(&reply).unwrap();
        let loaded = archive.quick_replies().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].shortcut, "audio1");
        assert_eq!(loaded[0].voice, Some(vec![0.1, -0.2, 0.5]));

        let text_update = QuickReply {
            id: "1".into(),
            shortcut: "audio1".into(),
            message: "Updated Hello".into(),
            keywords: vec!["intro".into()],
            count: 1,
            voice: None,
        };
        archive.upsert_quick_reply(&text_update).unwrap();
        let reloaded = archive.quick_replies().unwrap();
        assert_eq!(reloaded[0].message, "Updated Hello");
        assert_eq!(reloaded[0].voice, Some(vec![0.1, -0.2, 0.5]));

        archive.delete_quick_reply("1").unwrap();
        assert!(archive.quick_replies().unwrap().is_empty());
    }
}
