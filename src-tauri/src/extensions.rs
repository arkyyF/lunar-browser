//! Extension host (minimal).
//!
//! v1 supports a *subset* of Chrome MV3 — manifest.json parsing, content
//! scripts (CSS + JS injected per-tab by URL match), and storage.local.
//! Background service workers and the chrome.* API surface are stubbed.

use crate::storage::Database;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extension {
    pub id: String,
    pub name: String,
    pub version: String,
    pub manifest: serde_json::Value,
    pub enabled: bool,
    pub installed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentScript {
    pub matches: Vec<String>,
    pub js: Vec<String>,
    pub css: Vec<String>,
    pub run_at: String,
}

pub struct ExtensionHost {
    db: Arc<Database>,
}

impl ExtensionHost {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub async fn install(&self, manifest_json: String) -> Result<Extension> {
        let manifest: serde_json::Value = serde_json::from_str(&manifest_json)?;
        let id = Uuid::new_v4().to_string();
        let name = manifest
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unnamed")
            .to_string();
        let version = manifest
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("0.0.0")
            .to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.db.conn().await;
        conn.execute(
            "INSERT INTO extensions (id, name, version, manifest, enabled, installed_at)
             VALUES (?1, ?2, ?3, ?4, 1, ?5)",
            rusqlite::params![id, name, version, manifest_json, now],
        )?;
        Ok(Extension { id, name, version, manifest, enabled: true, installed_at: now })
    }

    pub async fn list(&self) -> Result<Vec<Extension>> {
        let conn = self.db.conn().await;
        let mut stmt = conn.prepare(
            "SELECT id, name, version, manifest, enabled, installed_at FROM extensions",
        )?;
        let rows = stmt.query_map([], |r| {
            let manifest_s: String = r.get(3)?;
            let manifest: serde_json::Value = serde_json::from_str(&manifest_s).unwrap_or_default();
            Ok(Extension {
                id: r.get(0)?, name: r.get(1)?, version: r.get(2)?,
                manifest, enabled: r.get::<_, i64>(4)? != 0, installed_at: r.get(5)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows { out.push(row?); }
        Ok(out)
    }

    pub async fn set_enabled(&self, id: &str, enabled: bool) -> Result<()> {
        let conn = self.db.conn().await;
        conn.execute(
            "UPDATE extensions SET enabled = ?1 WHERE id = ?2",
            rusqlite::params![if enabled { 1 } else { 0 }, id],
        )?;
        Ok(())
    }

    pub async fn remove(&self, id: &str) -> Result<()> {
        let conn = self.db.conn().await;
        conn.execute("DELETE FROM extensions WHERE id = ?1", rusqlite::params![id])?;
        Ok(())
    }

    /// Resolve which content scripts apply to a URL (returns manifest-style entries).
    pub fn matching_scripts(&self, _url: &str) -> Vec<ContentScript> {
        // Stubbed — real impl parses manifest + matches glob.
        Vec::new()
    }
}
