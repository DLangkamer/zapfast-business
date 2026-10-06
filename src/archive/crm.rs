//! Customer Relationship Management (CRM) store: Kanban pipelines, deals,
//! internal contact notes, tags, and follow-up reminders.
//! Stored in the account's encrypted SQLCipher archive.db for 100% offline isolation.

use rusqlite::params;

use super::{Archive, Result};
use crate::model::{CrmBackup, CrmColumn, CrmDeal, CrmFollowup};

pub const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS crm_columns (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    color TEXT NOT NULL,
    sort_order INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS crm_deals (
    chat_id TEXT PRIMARY KEY,
    column_id TEXT NOT NULL,
    value_cents INTEGER NOT NULL DEFAULT 0,
    notes TEXT NOT NULL DEFAULT '',
    tags TEXT NOT NULL DEFAULT '[]',
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS crm_followups (
    id TEXT PRIMARY KEY,
    chat_id TEXT NOT NULL,
    title TEXT NOT NULL,
    remind_at INTEGER NOT NULL,
    done INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_crm_followups_remind ON crm_followups (remind_at, done);
";

impl Archive {
    /// Returns all pipeline columns in display order. Seeds default stages if none exist.
    pub fn crm_columns(&self) -> Result<Vec<CrmColumn>> {
        let mut statement = self.connection.prepare(
            "SELECT id, title, color, sort_order
             FROM crm_columns
             ORDER BY sort_order ASC, title ASC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(CrmColumn {
                id: row.get(0)?,
                title: row.get(1)?,
                color: row.get(2)?,
                order: row.get(3)?,
            })
        })?;
        let columns: Vec<CrmColumn> = rows.collect::<std::result::Result<Vec<_>, _>>()?;

        if columns.is_empty() {
            let defaults = vec![
                CrmColumn {
                    id: "lead".to_owned(),
                    title: "Novos Contatos".to_owned(),
                    color: "#3b82f6".to_owned(),
                    order: 0,
                },
                CrmColumn {
                    id: "qual".to_owned(),
                    title: "Qualificação".to_owned(),
                    color: "#eab308".to_owned(),
                    order: 1,
                },
                CrmColumn {
                    id: "prop".to_owned(),
                    title: "Proposta / Negociação".to_owned(),
                    color: "#8b5cf6".to_owned(),
                    order: 2,
                },
                CrmColumn {
                    id: "close".to_owned(),
                    title: "Fechamento".to_owned(),
                    color: "#10b981".to_owned(),
                    order: 3,
                },
                CrmColumn {
                    id: "post".to_owned(),
                    title: "Pós-Venda".to_owned(),
                    color: "#6b7280".to_owned(),
                    order: 4,
                },
            ];
            for col in &defaults {
                let _ = self.upsert_crm_column(col);
            }
            return Ok(defaults);
        }

        Ok(columns)
    }

    pub fn upsert_crm_column(&self, column: &CrmColumn) -> Result<()> {
        self.connection.execute(
            "INSERT INTO crm_columns (id, title, color, sort_order)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET title=excluded.title, color=excluded.color, sort_order=excluded.sort_order",
            params![column.id, column.title, column.color, column.order],
        )?;
        Ok(())
    }

    pub fn delete_crm_column(&self, id: &str) -> Result<bool> {
        Ok(self.connection.execute(
            "DELETE FROM crm_columns WHERE id = ?1",
            params![id],
        )? > 0)
    }

    pub fn crm_deals(&self) -> Result<Vec<CrmDeal>> {
        let mut statement = self.connection.prepare(
            "SELECT chat_id, column_id, value_cents, notes, tags, updated_at
             FROM crm_deals
             ORDER BY updated_at DESC",
        )?;
        let rows = statement.query_map([], |row| {
            let tags_json: String = row.get(4)?;
            let tags = serde_json::from_str(&tags_json).unwrap_or_default();
            Ok(CrmDeal {
                chat_id: row.get(0)?,
                column_id: row.get(1)?,
                value_cents: row.get(2)?,
                notes: row.get(3)?,
                tags,
                updated_at: row.get(5)?,
            })
        })?;
        rows.collect()
    }

    pub fn crm_deal(&self, chat_id: &str) -> Result<Option<CrmDeal>> {
        let mut statement = self.connection.prepare(
            "SELECT chat_id, column_id, value_cents, notes, tags, updated_at
             FROM crm_deals
             WHERE chat_id = ?1",
        )?;
        let mut rows = statement.query_map(params![chat_id], |row| {
            let tags_json: String = row.get(4)?;
            let tags = serde_json::from_str(&tags_json).unwrap_or_default();
            Ok(CrmDeal {
                chat_id: row.get(0)?,
                column_id: row.get(1)?,
                value_cents: row.get(2)?,
                notes: row.get(3)?,
                tags,
                updated_at: row.get(5)?,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn upsert_crm_deal(&self, deal: &CrmDeal) -> Result<()> {
        let tags_json = serde_json::to_string(&deal.tags).unwrap_or_else(|_| "[]".to_owned());
        self.connection.execute(
            "INSERT INTO crm_deals (chat_id, column_id, value_cents, notes, tags, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(chat_id) DO UPDATE SET
                column_id=excluded.column_id,
                value_cents=excluded.value_cents,
                notes=excluded.notes,
                tags=excluded.tags,
                updated_at=excluded.updated_at",
            params![
                deal.chat_id,
                deal.column_id,
                deal.value_cents,
                deal.notes,
                tags_json,
                deal.updated_at
            ],
        )?;
        Ok(())
    }

    pub fn crm_followups(&self) -> Result<Vec<CrmFollowup>> {
        let mut statement = self.connection.prepare(
            "SELECT id, chat_id, title, remind_at, done, created_at
             FROM crm_followups
             ORDER BY remind_at ASC, created_at ASC",
        )?;
        let rows = statement.query_map([], |row| {
            let done_int: i64 = row.get(4)?;
            Ok(CrmFollowup {
                id: row.get(0)?,
                chat_id: row.get(1)?,
                title: row.get(2)?,
                remind_at: row.get(3)?,
                done: done_int != 0,
                created_at: row.get(5)?,
            })
        })?;
        rows.collect()
    }

    pub fn upsert_crm_followup(&self, followup: &CrmFollowup) -> Result<()> {
        self.connection.execute(
            "INSERT INTO crm_followups (id, chat_id, title, remind_at, done, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
                chat_id=excluded.chat_id,
                title=excluded.title,
                remind_at=excluded.remind_at,
                done=excluded.done",
            params![
                followup.id,
                followup.chat_id,
                followup.title,
                followup.remind_at,
                if followup.done { 1 } else { 0 },
                followup.created_at,
            ],
        )?;
        Ok(())
    }

    pub fn delete_crm_followup(&self, id: &str) -> Result<bool> {
        Ok(self.connection.execute(
            "DELETE FROM crm_followups WHERE id = ?1",
            params![id],
        )? > 0)
    }

    pub fn complete_crm_followup(&self, id: &str) -> Result<bool> {
        Ok(self.connection.execute(
            "UPDATE crm_followups SET done = 1 WHERE id = ?1",
            params![id],
        )? > 0)
    }

    pub fn snooze_crm_followup(&self, id: &str, until: i64) -> Result<bool> {
        Ok(self.connection.execute(
            "UPDATE crm_followups SET remind_at = ?2, done = 0 WHERE id = ?1",
            params![id, until],
        )? > 0)
    }

    /// Exports all CRM pipeline columns, contact deals, notes, tags, and follow-ups to a portable backup struct.
    pub fn export_crm_backup(&self) -> Result<CrmBackup> {
        let columns = self.crm_columns()?;
        let deals = self.crm_deals()?;
        let followups = self.crm_followups()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        Ok(CrmBackup {
            version: 1,
            exported_at: now,
            account_id: None,
            columns,
            deals,
            followups,
        })
    }

    /// Imports and restores CRM backup data into this account's database.
    pub fn import_crm_backup(&self, backup: &CrmBackup) -> Result<()> {
        for col in &backup.columns {
            self.upsert_crm_column(col)?;
        }
        for deal in &backup.deals {
            self.upsert_crm_deal(deal)?;
        }
        for followup in &backup.followups {
            self.upsert_crm_followup(followup)?;
        }
        Ok(())
    }
}
