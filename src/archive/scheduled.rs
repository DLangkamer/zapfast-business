//! Durable scheduled text messages stored in the encrypted archive.

use super::{Archive, Result, params};

pub const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS scheduled_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    chat TEXT NOT NULL,
    text TEXT NOT NULL,
    mentions TEXT NOT NULL DEFAULT '[]',
    quoting TEXT,
    send_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS scheduled_messages_due ON scheduled_messages(send_at, id);
";

#[derive(Clone, Debug, PartialEq)]
pub struct ScheduledMessage {
    pub id: i64,
    pub chat: String,
    pub text: String,
    pub mentions: Vec<String>,
    pub quoting: Option<String>,
    pub send_at: i64,
}

impl Archive {
    pub fn schedule_text(
        &self,
        chat: &str,
        text: &str,
        mentions: &[String],
        quoting: Option<&str>,
        send_at: i64,
    ) -> Result<i64> {
        self.connection.execute(
            "INSERT INTO scheduled_messages (chat, text, mentions, quoting, send_at, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                chat,
                text,
                serde_json::to_string(mentions).unwrap_or_else(|_| "[]".into()),
                quoting,
                send_at,
                crate::util::now(),
            ],
        )?;
        Ok(self.connection.last_insert_rowid())
    }

    pub fn scheduled_due(&self, now: i64) -> Result<Vec<ScheduledMessage>> {
        let mut statement = self.connection.prepare(
            "SELECT id, chat, text, mentions, quoting, send_at
             FROM scheduled_messages WHERE send_at <= ?1 ORDER BY send_at, id LIMIT 20",
        )?;
        let rows = statement.query_map([now], |row| {
            let mentions: String = row.get(3)?;
            Ok(ScheduledMessage {
                id: row.get(0)?,
                chat: row.get(1)?,
                text: row.get(2)?,
                mentions: serde_json::from_str(&mentions).unwrap_or_default(),
                quoting: row.get(4)?,
                send_at: row.get(5)?,
            })
        })?;
        rows.collect()
    }

    pub fn delete_scheduled(&self, id: i64) -> Result<()> {
        self.connection
            .execute("DELETE FROM scheduled_messages WHERE id = ?1", [id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduled_text_survives_restart_and_only_becomes_due_at_its_time() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("fixture.db");
        let key = [31; 32];
        let id;
        {
            let archive = Archive::open_with_key(&path, &key).unwrap();
            id = archive
                .schedule_text(
                    "group@g.us",
                    "planned message",
                    &["one@s.whatsapp.net".into()],
                    Some("quoted"),
                    200,
                )
                .unwrap();
            assert!(archive.scheduled_due(199).unwrap().is_empty());
        }
        let archive = Archive::open_with_key(&path, &key).unwrap();
        let due = archive.scheduled_due(200).unwrap();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].text, "planned message");
        assert_eq!(due[0].mentions, vec!["one@s.whatsapp.net"]);
        archive.delete_scheduled(id).unwrap();
        assert!(archive.scheduled_due(i64::MAX).unwrap().is_empty());
    }
}
