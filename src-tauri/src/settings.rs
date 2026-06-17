//! User settings (key-value store, persisted to SQLite).

use crate::storage::Database;
use anyhow::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub default_search_engine: String,
    pub homepage: String,
    pub restore_last_session: bool,
    pub block_ads: bool,
    pub block_trackers: bool,
    pub strict_privacy: bool,
    pub memory_strategy: String,
    pub memory_budget_mb: u64,
    pub download_dir: String,
    pub theme: String,
    pub accent_color: String,
    pub enable_extensions: bool,
    pub enable_split_view: bool,
    pub vertical_tabs: bool,
    pub command_palette_enabled: bool,
    pub do_not_track: bool,
    pub https_only: bool,
    pub extras: HashMap<String, serde_json::Value>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            default_search_engine: "google".into(),
            homepage: "lunar://newtab".into(),
            restore_last_session: true,
            block_ads: true,
            block_trackers: true,
            strict_privacy: true,
            memory_strategy: "aggressive".into(),
            memory_budget_mb: 1024,
            download_dir: dirs::download_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "~/Downloads".into()),
            theme: "moonlit-dark".into(),
            accent_color: "#C0C8D0".into(),
            enable_extensions: true,
            enable_split_view: true,
            vertical_tabs: true,
            command_palette_enabled: true,
            do_not_track: true,
            https_only: true,
            extras: HashMap::new(),
        }
    }
}

impl Settings {
    pub fn load(db: &Arc<Database>) -> Result<Self> {
        let conn = futures::executor::block_on(db.conn());
        let mut s = Settings::default();
        let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
        let rows = stmt.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (k, v) = row?;
            match k.as_str() {
                "default_search_engine" => s.default_search_engine = v,
                "homepage" => s.homepage = v,
                "restore_last_session" => s.restore_last_session = v == "1",
                "block_ads" => s.block_ads = v == "1",
                "block_trackers" => s.block_trackers = v == "1",
                "strict_privacy" => s.strict_privacy = v == "1",
                "memory_strategy" => s.memory_strategy = v,
                "memory_budget_mb" => s.memory_budget_mb = v.parse().unwrap_or(1024),
                "download_dir" => s.download_dir = v,
                "theme" => s.theme = v,
                "accent_color" => s.accent_color = v,
                "enable_extensions" => s.enable_extensions = v == "1",
                "enable_split_view" => s.enable_split_view = v == "1",
                "vertical_tabs" => s.vertical_tabs = v == "1",
                "command_palette_enabled" => s.command_palette_enabled = v == "1",
                "do_not_track" => s.do_not_track = v == "1",
                "https_only" => s.https_only = v == "1",
                _ => {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&v) {
                        s.extras.insert(k, val);
                    }
                }
            }
        }
        Ok(s)
    }

    pub async fn save(&self, db: &Arc<Database>) -> Result<()> {
        let conn = db.conn().await;
        let now = chrono::Utc::now().to_rfc3339();
        let mut pairs = vec![
            ("default_search_engine", self.default_search_engine.clone()),
            ("homepage", self.homepage.clone()),
            ("restore_last_session", if self.restore_last_session { "1".into() } else { "0".into() }),
            ("block_ads", if self.block_ads { "1".into() } else { "0".into() }),
            ("block_trackers", if self.block_trackers { "1".into() } else { "0".into() }),
            ("strict_privacy", if self.strict_privacy { "1".into() } else { "0".into() }),
            ("memory_strategy", self.memory_strategy.clone()),
            ("memory_budget_mb", self.memory_budget_mb.to_string()),
            ("download_dir", self.download_dir.clone()),
            ("theme", self.theme.clone()),
            ("accent_color", self.accent_color.clone()),
            ("enable_extensions", if self.enable_extensions { "1".into() } else { "0".into() }),
            ("enable_split_view", if self.enable_split_view { "1".into() } else { "0".into() }),
            ("vertical_tabs", if self.vertical_tabs { "1".into() } else { "0".into() }),
            ("command_palette_enabled", if self.command_palette_enabled { "1".into() } else { "0".into() }),
            ("do_not_track", if self.do_not_track { "1".into() } else { "0".into() }),
            ("https_only", if self.https_only { "1".into() } else { "0".into() }),
        ];
        for (k, v) in &self.extras {
            pairs.push((k.as_str(), v.to_string()));
        }
        for (k, v) in pairs {
            conn.execute(
                "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
                rusqlite::params![k, v, now],
            )?;
        }
        Ok(())
    }
}

pub type SettingsLock = RwLock<Settings>;

// Minimal `dirs` shim — avoids pulling the dirs crate for one function.
mod dirs {
    pub fn download_dir() -> Option<std::path::PathBuf> {
        std::env::var_os("XDG_DOWNLOAD_DIR")
            .or_else(|| std::env::var_os("HOME").map(|h| {
                let mut p = std::path::PathBuf::from(h);
                p.push("Downloads");
                p.into()
            }))
            .map(std::path::PathBuf::from)
    }
}
