//! Browsing history storage.

use crate::storage::Database;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: i64,
    pub url: String,
    pub title: Option<String>,
    pub visit_time: String,
    pub visit_count: i32,
    pub favicon: Option<String>,
}

pub struct HistoryStore {
    db: Arc<Database>,
}

impl HistoryStore {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub async fn add(&self, url: String, title: Option<String>, favicon: Option<String>, incognito: bool) -> Result<()> {
        if incognito {
            return Ok(()); // never persisted
        }
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.db.conn().await;
        // Upsert: bump visit_count if URL exists.
        conn.execute(
            "INSERT INTO history (url, title, visit_time, visit_count, favicon, incognito)
             VALUES (?1, ?2, ?3, 1, ?4, 0)
             ON CONFLICT(url) DO UPDATE SET
                visit_count = visit_count + 1,
                visit_time = excluded.visit_time,
                title = COALESCE(excluded.title, title),
                favicon = COALESCE(excluded.favicon, favicon)",
            rusqlite::params![url, title, now, favicon],
        )?;
        // Trim history to last 10k entries.
        conn.execute(
            "DELETE FROM history WHERE id NOT IN (
                SELECT id FROM history ORDER BY visit_time DESC LIMIT 10000
            )",
            [],
        )?;
        Ok(())
    }

    pub async fn list(&self, limit: i64, offset: i64) -> Result<Vec<HistoryEntry>> {
        let conn = self.db.conn().await;
        let mut stmt = conn.prepare(
            "SELECT id, url, title, visit_time, visit_count, favicon
             FROM history ORDER BY visit_time DESC LIMIT ?1 OFFSET ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![limit, offset], |r| {
            Ok(HistoryEntry {
                id: r.get(0)?, url: r.get(1)?, title: r.get(2)?,
                visit_time: r.get(3)?, visit_count: r.get(4)?, favicon: r.get(5)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows { out.push(row?); }
        Ok(out)
    }

    pub async fn search(&self, query: &str, limit: i64) -> Result<Vec<HistoryEntry>> {
        let q = format!("%{}%", query.to_lowercase());
        let conn = self.db.conn().await;
        let mut stmt = conn.prepare(
            "SELECT id, url, title, visit_time, visit_count, favicon
             FROM history
             WHERE LOWER(url) LIKE ?1 OR LOWER(COALESCE(title, '')) LIKE ?1
             ORDER BY visit_count DESC, visit_time DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![q, limit], |r| {
            Ok(HistoryEntry {
                id: r.get(0)?, url: r.get(1)?, title: r.get(2)?,
                visit_time: r.get(3)?, visit_count: r.get(4)?, favicon: r.get(5)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows { out.push(row?); }
        Ok(out)
    }

    pub async fn remove(&self, id: i64) -> Result<()> {
        let conn = self.db.conn().await;
        conn.execute("DELETE FROM history WHERE id = ?1", rusqlite::params![id])?;
        Ok(())
    }

    pub async fn clear(&self) -> Result<()> {
        let conn = self.db.conn().await;
        conn.execute("DELETE FROM history", [])?;
        Ok(())
    }
}
