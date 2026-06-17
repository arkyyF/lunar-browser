//! Bookmark storage.

use crate::storage::Database;
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub id: String,
    pub url: String,
    pub title: String,
    pub favicon: Option<String>,
    pub folder_id: Option<String>,
    pub position: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkFolder {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub position: i32,
    pub created_at: String,
}

pub struct BookmarkStore {
    db: Arc<Database>,
}

impl BookmarkStore {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub async fn add(&self, url: String, title: String, favicon: Option<String>, folder_id: Option<String>) -> Result<Bookmark> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let conn = self.db.conn().await;
        let position: i32 = conn
            .query_row(
                "SELECT COALESCE(MAX(position), -1) + 1 FROM bookmarks WHERE folder_id IS ?1",
                rusqlite::params![folder_id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        conn.execute(
            "INSERT INTO bookmarks (id, url, title, favicon, folder_id, position, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![id, url, title, favicon, folder_id, position, now, now],
        )?;
        Ok(Bookmark {
            id, url, title, favicon, folder_id, position,
            created_at: now.clone(), updated_at: now,
        })
    }

    pub async fn remove(&self, id: &str) -> Result<()> {
        let conn = self.db.conn().await;
        conn.execute("DELETE FROM bookmarks WHERE id = ?1", rusqlite::params![id])?;
        Ok(())
    }

    pub async fn list(&self, folder_id: Option<String>) -> Result<Vec<Bookmark>> {
        let conn = self.db.conn().await;
        let mut stmt = conn.prepare(
            "SELECT id, url, title, favicon, folder_id, position, created_at, updated_at
             FROM bookmarks WHERE folder_id IS ?1 ORDER BY position ASC",
        )?;
        let rows = stmt.query_map(rusqlite::params![folder_id], |r| {
            Ok(Bookmark {
                id: r.get(0)?,
                url: r.get(1)?,
                title: r.get(2)?,
                favicon: r.get(3)?,
                folder_id: r.get(4)?,
                position: r.get(5)?,
                created_at: r.get(6)?,
                updated_at: r.get(7)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows { out.push(row?); }
        Ok(out)
    }

    pub async fn search(&self, query: &str) -> Result<Vec<Bookmark>> {
        let q = format!("%{}%", query.to_lowercase());
        let conn = self.db.conn().await;
        let mut stmt = conn.prepare(
            "SELECT id, url, title, favicon, folder_id, position, created_at, updated_at
             FROM bookmarks
             WHERE LOWER(title) LIKE ?1 OR LOWER(url) LIKE ?1
             ORDER BY position ASC LIMIT 100",
        )?;
        let rows = stmt.query_map(rusqlite::params![q], |r| {
            Ok(Bookmark {
                id: r.get(0)?, url: r.get(1)?, title: r.get(2)?,
                favicon: r.get(3)?, folder_id: r.get(4)?, position: r.get(5)?,
                created_at: r.get(6)?, updated_at: r.get(7)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows { out.push(row?); }
        Ok(out)
    }

    pub async fn create_folder(&self, name: String, parent_id: Option<String>) -> Result<BookmarkFolder> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let conn = self.db.conn().await;
        let position: i32 = conn
            .query_row(
                "SELECT COALESCE(MAX(position), -1) + 1 FROM bookmark_folders WHERE parent_id IS ?1",
                rusqlite::params![parent_id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        conn.execute(
            "INSERT INTO bookmark_folders (id, name, parent_id, position, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, name, parent_id, position, now],
        )?;
        Ok(BookmarkFolder { id, name, parent_id, position, created_at: now })
    }
}
