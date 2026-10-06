//! Durable scheduled text and voice messages stored in the encrypted archive.

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScheduledKind {
    Text,
    Voice,
    Files,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScheduledMessage {
    pub id: i64,
    pub chat: String,
    pub text: String,
    pub mentions: Vec<String>,
    pub quoting: Option<String>,
    pub send_at: i64,
    pub kind: ScheduledKind,
    pub voice: Option<Vec<f32>>,
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
            "SELECT id, chat, text, mentions, quoting, send_at, kind, voice
             FROM scheduled_messages WHERE send_at <= ?1 ORDER BY send_at, id LIMIT 20",
        )?;
        let rows = statement.query_map([now], |row| {
            let mentions: String = row.get(3)?;
            let kind = match row.get::<_, String>(6)?.as_str() {
                "voice" => ScheduledKind::Voice,
                "files" => ScheduledKind::Files,
                _ => ScheduledKind::Text,
            };
            Ok(ScheduledMessage {
                id: row.get(0)?,
                chat: row.get(1)?,
                text: row.get(2)?,
                mentions: serde_json::from_str(&mentions).unwrap_or_default(),
                quoting: row.get(4)?,
                send_at: row.get(5)?,
                kind,
                voice: row.get::<_, Option<Vec<u8>>>(7)?.map(bytes_to_samples),
            })
        })?;
        rows.collect()
    }

    pub fn scheduled_messages(&self) -> Result<Vec<ScheduledMessage>> {
        let mut statement = self.connection.prepare(
            "SELECT id, chat, text, mentions, quoting, send_at, kind, voice
             FROM scheduled_messages ORDER BY send_at, id",
        )?;
        let rows = statement.query_map([], |row| {
            let mentions: String = row.get(3)?;
            let kind = match row.get::<_, String>(6)?.as_str() {
                "voice" => ScheduledKind::Voice,
                "files" => ScheduledKind::Files,
                _ => ScheduledKind::Text,
            };
            Ok(ScheduledMessage {
                id: row.get(0)?,
                chat: row.get(1)?,
                text: row.get(2)?,
                mentions: serde_json::from_str(&mentions).unwrap_or_default(),
                quoting: row.get(4)?,
                send_at: row.get(5)?,
                kind,
                voice: row.get::<_, Option<Vec<u8>>>(7)?.map(bytes_to_samples),
            })
        })?;
        rows.collect()
    }

    pub fn schedule_voice(
        &self,
        chat: &str,
        samples: &[f32],
        quoting: Option<&str>,
        send_at: i64,
    ) -> Result<i64> {
        let bytes: Vec<u8> = samples
            .iter()
            .flat_map(|sample| sample.to_le_bytes())
            .collect();
        self.connection.execute(
            "INSERT INTO scheduled_messages (chat, text, mentions, quoting, send_at, created_at, kind, voice)
             VALUES (?1, '', '[]', ?2, ?3, ?4, 'voice', ?5)",
            params![chat, quoting, send_at, crate::util::now(), bytes],
        )?;
        Ok(self.connection.last_insert_rowid())
    }

    pub fn schedule_files(
        &self,
        chat: &str,
        paths: &[std::path::PathBuf],
        caption: Option<&str>,
        quoting: Option<&str>,
        send_at: i64,
    ) -> Result<i64> {
        let payload = serde_json::to_string(&(paths, caption)).unwrap_or_else(|_| "[]".into());
        self.connection.execute(
            "INSERT INTO scheduled_messages (chat, text, mentions, quoting, send_at, created_at, kind, voice)
             VALUES (?1, ?2, '[]', ?3, ?4, ?5, 'files', NULL)",
            params![chat, payload, quoting, send_at, crate::util::now()],
        )?;
        Ok(self.connection.last_insert_rowid())
    }

    pub fn update_scheduled(&self, id: i64, text: Option<&str>, send_at: i64) -> Result<()> {
        self.connection.execute(
            "UPDATE scheduled_messages
             SET text = coalesce(?2, text),
                 mentions = CASE WHEN ?2 IS NULL THEN mentions ELSE '[]' END,
                 send_at = ?3
             WHERE id = ?1",
            params![id, text, send_at],
        )?;
        Ok(())
    }

    pub fn delete_scheduled(&self, id: i64) -> Result<()> {
        self.connection
            .execute("DELETE FROM scheduled_messages WHERE id = ?1", [id])?;
        Ok(())
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
        assert_eq!(due[0].kind, ScheduledKind::Text);
        archive.delete_scheduled(id).unwrap();
        assert!(archive.scheduled_due(i64::MAX).unwrap().is_empty());
    }

    #[test]
    fn scheduled_items_can_be_listed_edited_cancelled_and_include_voice() {
        let archive = Archive::in_memory().unwrap();
        let text = archive
            .schedule_text("peer", "before", &[], None, 200)
            .unwrap();
        let voice = archive
            .schedule_voice("group", &[0.25, -0.5, 1.0], Some("quoted"), 300)
            .unwrap();
        archive.update_scheduled(text, Some("after"), 250).unwrap();
        let rows = archive.scheduled_messages().unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!((rows[0].text.as_str(), rows[0].send_at), ("after", 250));
        assert_eq!(rows[1].kind, ScheduledKind::Voice);
        assert_eq!(rows[1].voice.as_deref(), Some(&[0.25, -0.5, 1.0][..]));
        archive.delete_scheduled(voice).unwrap();
        assert_eq!(archive.scheduled_messages().unwrap().len(), 1);
    }
}
