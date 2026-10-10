//! Customer Relationship Management (CRM) store: Kanban pipelines, deals,
//! internal contact notes, tags, and follow-up reminders.
//! Stored in the account's encrypted SQLCipher archive.db for 100% offline isolation.

use rusqlite::params;

use super::{Archive, Result};
use crate::model::{CrmBackup, CrmColumn, CrmDeal, CrmFollowup, CrmProject, CrmTask};

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

CREATE TABLE IF NOT EXISTS crm_tasks (
    id TEXT PRIMARY KEY,
    chat_id TEXT,
    project_id TEXT,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'todo',
    priority TEXT NOT NULL DEFAULT 'normal',
    kind TEXT NOT NULL DEFAULT 'task',
    duration_minutes INTEGER,
    due_at INTEGER,
    completed_at INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_crm_tasks_status_due ON crm_tasks (status, due_at);
CREATE INDEX IF NOT EXISTS idx_crm_tasks_chat ON crm_tasks (chat_id);

CREATE TABLE IF NOT EXISTS crm_projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    kind TEXT NOT NULL DEFAULT 'client',
    status TEXT NOT NULL DEFAULT 'active',
    color TEXT NOT NULL DEFAULT '#00a884',
    icon TEXT NOT NULL DEFAULT 'briefcase',
    start_at INTEGER,
    due_at INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS crm_project_chats (
    project_id TEXT NOT NULL,
    chat_id TEXT NOT NULL,
    PRIMARY KEY (project_id, chat_id)
);

CREATE INDEX IF NOT EXISTS idx_crm_project_chats_chat ON crm_project_chats (chat_id);
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
        let mut columns: Vec<CrmColumn> = rows.collect::<std::result::Result<Vec<_>, _>>()?;

        if columns.is_empty() {
            let defaults = vec![
                CrmColumn {
                    id: "lead".to_owned(),
                    title: "Lead".to_owned(),
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

        // Ensure legacy "Novos Contatos" stage title is normalized to "Lead"
        for col in &mut columns {
            if col.id == "lead" && col.title == "Novos Contatos" {
                col.title = "Lead".to_owned();
                let _ = self.upsert_crm_column(col);
            }
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
        let columns = self.crm_columns()?;
        let fallback_id = columns.iter().find(|c| c.id != id).map(|c| c.id.clone());

        self.connection.execute("BEGIN IMMEDIATE", [])?;
        let result = (|| -> Result<bool> {
            if let Some(target) = fallback_id {
                self.connection.execute(
                    "UPDATE crm_deals SET column_id = ?1 WHERE column_id = ?2",
                    params![target, id],
                )?;
            }
            let rows = self
                .connection
                .execute("DELETE FROM crm_columns WHERE id = ?1", params![id])?;
            Ok(rows > 0)
        })();

        match result {
            Ok(deleted) => {
                self.connection.execute("COMMIT", [])?;
                Ok(deleted)
            }
            Err(err) => {
                let _ = self.connection.execute("ROLLBACK", []);
                Err(err)
            }
        }
    }

    pub fn crm_deals(&self) -> Result<Vec<CrmDeal>> {
        let columns = self.crm_columns().unwrap_or_default();
        let first_col_id = columns.first().map(|c| c.id.as_str()).unwrap_or("lead");

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
        let mut deals: Vec<CrmDeal> = rows.collect::<std::result::Result<Vec<_>, _>>()?;

        // Reconcile and heal any deals with invalid or orphaned column_ids
        for deal in &mut deals {
            if !columns.is_empty() && !columns.iter().any(|c| c.id == deal.column_id) {
                deal.column_id = first_col_id.to_owned();
                let _ = self.upsert_crm_deal(deal);
            }
        }
        Ok(deals)
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

    pub fn delete_crm_deal(&self, chat_id: &str) -> Result<bool> {
        let rows = self
            .connection
            .execute("DELETE FROM crm_deals WHERE chat_id = ?1", params![chat_id])?;
        Ok(rows > 0)
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
        Ok(self
            .connection
            .execute("DELETE FROM crm_followups WHERE id = ?1", params![id])?
            > 0)
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

    pub fn crm_tasks(&self) -> Result<Vec<CrmTask>> {
        let mut statement = self.connection.prepare(
            "SELECT id, chat_id, project_id, title, description, status, priority,
                    kind, duration_minutes, due_at, completed_at, created_at, updated_at
             FROM crm_tasks
             ORDER BY CASE WHEN status = 'done' THEN 1 ELSE 0 END, due_at IS NULL, due_at, created_at",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(CrmTask {
                id: row.get(0)?,
                chat_id: row.get(1)?,
                project_id: row.get(2)?,
                title: row.get(3)?,
                description: row.get(4)?,
                status: row.get(5)?,
                priority: row.get(6)?,
                kind: row.get(7)?,
                duration_minutes: row.get(8)?,
                due_at: row.get(9)?,
                completed_at: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            })
        })?;
        rows.collect()
    }

    pub fn upsert_crm_task(&self, task: &CrmTask) -> Result<()> {
        self.connection.execute(
            "INSERT INTO crm_tasks
             (id, chat_id, project_id, title, description, status, priority, kind, duration_minutes, due_at, completed_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(id) DO UPDATE SET chat_id=excluded.chat_id,
                project_id=excluded.project_id, title=excluded.title,
                description=excluded.description, status=excluded.status,
                priority=excluded.priority, kind=excluded.kind,
                duration_minutes=excluded.duration_minutes, due_at=excluded.due_at,
                completed_at=excluded.completed_at, updated_at=excluded.updated_at",
            params![task.id, task.chat_id, task.project_id, task.title, task.description,
                task.status, task.priority, task.kind, task.duration_minutes, task.due_at,
                task.completed_at, task.created_at, task.updated_at],
        )?;
        Ok(())
    }

    pub fn delete_crm_task(&self, id: &str) -> Result<bool> {
        Ok(self
            .connection
            .execute("DELETE FROM crm_tasks WHERE id = ?1", params![id])?
            > 0)
    }

    pub fn crm_projects(&self) -> Result<Vec<CrmProject>> {
        let mut statement = self.connection.prepare(
            "SELECT id, name, description, kind, status, color, icon, start_at,
                    due_at, created_at, updated_at
             FROM crm_projects ORDER BY CASE status WHEN 'active' THEN 0 ELSE 1 END,
                    updated_at DESC, name ASC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(CrmProject {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                kind: row.get(3)?,
                status: row.get(4)?,
                color: row.get(5)?,
                icon: row.get(6)?,
                start_at: row.get(7)?,
                due_at: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
                chat_ids: Vec::new(),
            })
        })?;
        let mut projects = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        let mut chats = self.connection.prepare(
            "SELECT chat_id FROM crm_project_chats WHERE project_id = ?1 ORDER BY rowid",
        )?;
        for project in &mut projects {
            project.chat_ids = chats
                .query_map(params![project.id], |row| row.get(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
        }
        Ok(projects)
    }

    pub fn upsert_crm_project(&self, project: &CrmProject) -> Result<()> {
        self.connection.execute(
            "INSERT INTO crm_projects
             (id, name, description, kind, status, color, icon, start_at, due_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(id) DO UPDATE SET name=excluded.name,
                description=excluded.description, kind=excluded.kind,
                status=excluded.status, color=excluded.color, icon=excluded.icon,
                start_at=excluded.start_at, due_at=excluded.due_at,
                updated_at=excluded.updated_at",
            params![project.id, project.name, project.description, project.kind,
                project.status, project.color, project.icon, project.start_at,
                project.due_at, project.created_at, project.updated_at],
        )?;
        self.connection.execute(
            "DELETE FROM crm_project_chats WHERE project_id = ?1",
            params![project.id],
        )?;
        for chat_id in &project.chat_ids {
            self.connection.execute(
                "INSERT OR IGNORE INTO crm_project_chats (project_id, chat_id) VALUES (?1, ?2)",
                params![project.id, chat_id],
            )?;
        }
        Ok(())
    }

    pub fn delete_crm_project(&self, id: &str) -> Result<bool> {
        self.connection.execute(
            "UPDATE crm_tasks SET project_id = NULL, updated_at = strftime('%s','now') WHERE project_id = ?1",
            params![id],
        )?;
        self.connection.execute(
            "DELETE FROM crm_project_chats WHERE project_id = ?1",
            params![id],
        )?;
        Ok(self
            .connection
            .execute("DELETE FROM crm_projects WHERE id = ?1", params![id])?
            > 0)
    }

    /// Exports all CRM pipeline columns, contact deals, notes, tags, and follow-ups to a portable backup struct.
    pub fn export_crm_backup(&self) -> Result<CrmBackup> {
        let columns = self.crm_columns()?;
        let deals = self.crm_deals()?;
        let followups = self.crm_followups()?;
        let tasks = self.crm_tasks()?;
        let projects = self.crm_projects()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        Ok(CrmBackup {
            version: 4,
            exported_at: now,
            account_id: None,
            columns,
            deals,
            followups,
            tasks,
            projects,
        })
    }

    /// Imports and restores CRM backup data into this account's database.
    pub fn import_crm_backup(&self, backup: &CrmBackup) -> Result<()> {
        self.connection.execute("BEGIN IMMEDIATE", [])?;
        let result = (|| -> Result<()> {
            for col in &backup.columns {
                self.upsert_crm_column(col)?;
            }
            for deal in &backup.deals {
                self.upsert_crm_deal(deal)?;
            }
            for followup in &backup.followups {
                self.upsert_crm_followup(followup)?;
            }
            for task in &backup.tasks {
                self.upsert_crm_task(task)?;
            }
            for project in &backup.projects {
                self.upsert_crm_project(project)?;
            }
            Ok(())
        })();

        match result {
            Ok(()) => {
                self.connection.execute("COMMIT", [])?;
                Ok(())
            }
            Err(err) => {
                let _ = self.connection.execute("ROLLBACK", []);
                Err(err)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crm_default_columns_use_lead_as_first_stage() {
        let archive = Archive::in_memory().unwrap();
        let cols = archive.crm_columns().unwrap();
        assert!(!cols.is_empty());
        assert_eq!(cols[0].id, "lead");
        assert_eq!(cols[0].title, "Lead");
    }

    #[test]
    fn crm_legacy_column_novos_contatos_is_normalized_to_lead() {
        let archive = Archive::in_memory().unwrap();
        // Insert legacy column with "Novos Contatos"
        archive
            .connection
            .execute(
                "UPDATE crm_columns SET title = 'Novos Contatos' WHERE id = 'lead'",
                [],
            )
            .unwrap();
        // crm_columns() auto-normalizes it
        let cols = archive.crm_columns().unwrap();
        assert_eq!(cols[0].title, "Lead");
    }

    #[test]
    fn crm_deal_lifecycle_and_deletion() {
        let archive = Archive::in_memory().unwrap();
        let deal = CrmDeal {
            chat_id: "551199999999@s.whatsapp.net".to_owned(),
            column_id: "lead".to_owned(),
            value_cents: 150000,
            notes: "Cliente interessado em plano empresarial".to_owned(),
            tags: vec!["VIP".to_owned(), "Empresarial".to_owned()],
            updated_at: 1700000000,
        };

        archive.upsert_crm_deal(&deal).unwrap();
        let deals = archive.crm_deals().unwrap();
        assert_eq!(deals.len(), 1);
        assert_eq!(deals[0].chat_id, "551199999999@s.whatsapp.net");
        assert_eq!(deals[0].value_cents, 150000);
        assert_eq!(deals[0].tags, vec!["VIP", "Empresarial"]);

        // Delete deal
        let deleted = archive
            .delete_crm_deal("551199999999@s.whatsapp.net")
            .unwrap();
        assert!(deleted);
        let deals_after = archive.crm_deals().unwrap();
        assert!(deals_after.is_empty());
    }

    #[test]
    fn crm_followups_are_independent_of_deals() {
        let archive = Archive::in_memory().unwrap();
        let followup = CrmFollowup {
            id: "fu_123".to_owned(),
            chat_id: "contact_without_deal@s.whatsapp.net".to_owned(),
            title: "Retornar ligação".to_owned(),
            remind_at: 1700003600,
            done: false,
            created_at: 1700000000,
        };

        archive.upsert_crm_followup(&followup).unwrap();
        // Deals remain empty!
        assert!(archive.crm_deals().unwrap().is_empty());

        let followups = archive.crm_followups().unwrap();
        assert_eq!(followups.len(), 1);
        assert_eq!(followups[0].title, "Retornar ligação");

        archive.complete_crm_followup("fu_123").unwrap();
        let updated = archive.crm_followups().unwrap();
        assert!(updated[0].done);

        archive.delete_crm_followup("fu_123").unwrap();
        assert!(archive.crm_followups().unwrap().is_empty());
    }

    #[test]
    fn crm_task_lifecycle_is_persistent_and_included_in_backup() {
        let archive = Archive::in_memory().unwrap();
        let mut task = CrmTask {
            id: "task_123".to_owned(),
            chat_id: Some("551199999999@s.whatsapp.net".to_owned()),
            project_id: None,
            title: "Preparar proposta".to_owned(),
            description: "Revisar valores antes da reunião".to_owned(),
            status: "todo".to_owned(),
            priority: "high".to_owned(),
            kind: "task".to_owned(),
            duration_minutes: None,
            due_at: Some(1700003600),
            completed_at: None,
            created_at: 1700000000,
            updated_at: 1700000000,
        };

        archive.upsert_crm_task(&task).unwrap();
        let stored = archive.crm_tasks().unwrap();
        assert_eq!(stored, vec![task.clone()]);

        task.status = "done".to_owned();
        task.completed_at = Some(1700001800);
        task.updated_at = 1700001800;
        archive.upsert_crm_task(&task).unwrap();
        assert_eq!(archive.crm_tasks().unwrap()[0].status, "done");

        let backup = archive.export_crm_backup().unwrap();
        assert_eq!(backup.version, 4);
        assert_eq!(backup.tasks, vec![task]);

        assert!(archive.delete_crm_task("task_123").unwrap());
        assert!(archive.crm_tasks().unwrap().is_empty());
    }

    #[test]
    fn crm_project_keeps_chat_links_and_detaches_tasks_when_deleted() {
        let archive = Archive::in_memory().unwrap();
        let project = CrmProject {
            id: "project_123".to_owned(),
            name: "Implantação".to_owned(),
            description: "Projeto do grupo do cliente".to_owned(),
            kind: "client".to_owned(),
            status: "active".to_owned(),
            color: "#00a884".to_owned(),
            icon: "briefcase".to_owned(),
            start_at: Some(1700000000),
            due_at: None,
            created_at: 1700000000,
            updated_at: 1700000000,
            chat_ids: vec!["120363000000000000@g.us".to_owned()],
        };
        archive.upsert_crm_project(&project).unwrap();
        assert_eq!(archive.crm_projects().unwrap(), vec![project.clone()]);

        let task = CrmTask {
            id: "task_project".to_owned(),
            chat_id: None,
            project_id: Some(project.id.clone()),
            title: "Preparar briefing".to_owned(),
            description: String::new(),
            status: "todo".to_owned(),
            priority: "normal".to_owned(),
            kind: "task".to_owned(),
            duration_minutes: None,
            due_at: None,
            completed_at: None,
            created_at: 1700000000,
            updated_at: 1700000000,
        };
        archive.upsert_crm_task(&task).unwrap();
        assert!(archive.delete_crm_project(&project.id).unwrap());
        assert!(archive.crm_projects().unwrap().is_empty());
        assert_eq!(archive.crm_tasks().unwrap()[0].project_id, None);
    }
}
