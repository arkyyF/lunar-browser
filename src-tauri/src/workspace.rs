//! Workspaces — switchable groups of tabs (Edge/Zen style).

use crate::storage::Database;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub color: String,
    pub position: i32,
    pub created_at: String,
}

pub struct WorkspaceManager {
    db: Arc<Database>,
}

impl WorkspaceManager {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub async fn create(&self, name: String, icon: String, color: String) -> Result<Workspace> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.db.conn().await;
        let position: i32 = conn
            .query_row("SELECT COALESCE(MAX(position), -1) + 1 FROM workspaces", [], |r| r.get(0))
            .unwrap_or(0);
        conn.execute(
            "INSERT INTO workspaces (id, name, icon, color, position, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, name, icon, color, position, now],
        )?;
        Ok(Workspace { id, name, icon, color, position, created_at: now })
    }

    pub async fn list(&self) -> Result<Vec<Workspace>> {
        let conn = self.db.conn().await;
        let mut stmt = conn.prepare(
            "SELECT id, name, icon, color, position, created_at FROM workspaces ORDER BY position ASC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(Workspace {
                id: r.get(0)?, name: r.get(1)?, icon: r.get(2)?,
                color: r.get(3)?, position: r.get(4)?, created_at: r.get(5)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows { out.push(row?); }
        // If no workspaces exist, ensure a default exists.
        if out.is_empty() {
            drop(rows); drop(stmt);
            let default = self.create("Default".into(), "moon".into(), "#C0C8D0".into()).await?;
            out.push(default);
        }
        Ok(out)
    }

    pub async fn delete(&self, id: &str) -> Result<()> {
        let conn = self.db.conn().await;
        conn.execute("DELETE FROM workspaces WHERE id = ?1", rusqlite::params![id])?;
        Ok(())
    }
}
