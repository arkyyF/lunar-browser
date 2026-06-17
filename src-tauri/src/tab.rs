//! Tab model + lifecycle.
//!
//! Memory strategy (aggressive):
//! - Only the currently active tab holds a live WebView. All other tabs
//!   hold a `TabSnapshot` (URL + title + scroll position + favicon).
//! - On tab switch, we tear down the previous WebView and re-create one
//!   for the newly active tab, restoring scroll + history state.
//! - Inactive tabs are auto-discarded after 60s (configurable).
//! - Pinned tabs are exempt from auto-discard but still release their
//!   WebView when not active.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::storage::Database;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tab {
    pub id: String,
    pub url: String,
    pub title: String,
    pub favicon: Option<String>,
    pub pinned: bool,
    pub muted: bool,
    pub loading: bool,
    pub can_go_back: bool,
    pub can_go_forward: bool,
    pub incognito: bool,
    pub workspace_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
    pub discarded: bool,
    pub history_stack: HistoryStack,
    pub split_with: Option<String>,
    pub estimated_bytes: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HistoryStack {
    pub back: Vec<String>,
    pub forward: Vec<String>,
    pub current: Option<String>,
    pub scroll_y: f64,
}

impl Tab {
    pub fn new(url: String, workspace_id: Option<String>, incognito: bool) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            url,
            title: "New Tab".into(),
            favicon: None,
            pinned: false,
            muted: false,
            loading: false,
            can_go_back: false,
            can_go_forward: false,
            incognito,
            workspace_id,
            created_at: Utc::now(),
            last_active: Utc::now(),
            discarded: false,
            history_stack: HistoryStack::default(),
            split_with: None,
            estimated_bytes: 0,
        }
    }
}

pub struct TabManager {
    pub tabs: Arc<Mutex<HashMap<String, Tab>>>,
    pub active_tab_id: Arc<Mutex<Option<String>>>,
    pub db: Arc<Database>,
    pub memory_config: Arc<parking_lot::RwLock<crate::memory::MemoryConfig>>,
}

impl TabManager {
    pub fn new(db: Arc<Database>, memory_config: Arc<parking_lot::RwLock<crate::memory::MemoryConfig>>) -> Self {
        Self {
            tabs: Arc::new(Mutex::new(HashMap::new())),
            active_tab_id: Arc::new(Mutex::new(None)),
            db,
            memory_config,
        }
    }

    pub async fn create_tab(&self, url: String, workspace_id: Option<String>, incognito: bool) -> Tab {
        let mut tab = Tab::new(url.clone(), workspace_id, incognito);
        // If we're at the soft tab cap, evict the oldest non-pinned, non-active tab.
        let cap = self.memory_config.read().max_active_tabs;
        {
            let mut tabs = self.tabs.lock().await;
            if tabs.len() >= cap {
                let victim = tabs
                    .values()
                    .filter(|t| !t.pinned)
                    .min_by_key(|t| t.last_active)
                    .map(|t| t.id.clone());
                if let Some(id) = victim {
                    if let Some(mut t) = tabs.remove(&id) {
                        t.discarded = true;
                        tabs.insert(id.clone(), t);
                    }
                }
            }
            tab.last_active = Utc::now();
            tabs.insert(tab.id.clone(), tab.clone());
        }
        self.set_active(&tab.id).await;
        tab
    }

    pub async fn close_tab(&self, id: &str) -> Option<Tab> {
        let mut tabs = self.tabs.lock().await;
        let removed = tabs.remove(id)?;
        // If we just closed the active tab, pick a neighbor.
        let mut active = self.active_tab_id.lock().await;
        if active.as_deref() == Some(id) {
            *active = tabs
                .values()
                .max_by_key(|t| t.last_active)
                .map(|t| t.id.clone());
        }
        drop(tabs);
        drop(active);
        // Persist session.
        let _ = self.persist_session().await;
        Some(removed)
    }

    pub async fn set_active(&self, id: &str) {
        // Mark old active as discarded (suspends its WebView).
        {
            let mut tabs = self.tabs.lock().await;
            if let Some(old) = self.active_tab_id.lock().await.as_ref() {
                if old != id {
                    if let Some(t) = tabs.get_mut(old) {
                        if !t.pinned {
                            t.discarded = true;
                        }
                    }
                }
            }
            if let Some(t) = tabs.get_mut(id) {
                t.last_active = Utc::now();
                t.discarded = false;
            }
        }
        *self.active_tab_id.lock().await = Some(id.to_string());
    }

    pub async fn list(&self) -> Vec<Tab> {
        self.tabs.lock().await.values().cloned().collect()
    }

    pub async fn get(&self, id: &str) -> Option<Tab> {
        self.tabs.lock().await.get(id).cloned()
    }

    pub async fn update<F>(&self, id: &str, f: F) -> Option<Tab>
    where
        F: FnOnce(&mut Tab),
    {
        let mut tabs = self.tabs.lock().await;
        let tab = tabs.get_mut(id)?;
        f(tab);
        Some(tab.clone())
    }

    pub async fn discard(&self, id: &str) {
        let mut tabs = self.tabs.lock().await;
        if let Some(t) = tabs.get_mut(id) {
            if !t.pinned {
                t.discarded = true;
            }
        }
    }

    pub async fn restore(&self, id: &str) -> Option<Tab> {
        let mut tabs = self.tabs.lock().await;
        let t = tabs.get_mut(id)?;
        t.discarded = false;
        t.last_active = Utc::now();
        Some(t.clone())
    }

    pub async fn persist_session(&self) -> Result<()> {
        let tabs = self.tabs.lock().await;
        let snapshot: Vec<Tab> = tabs.values().cloned().collect();
        let tabs_json = serde_json::to_string(&snapshot)?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let conn = self.db.conn().await;
        conn.execute(
            "INSERT INTO sessions (id, created_at, tabs_json) VALUES (?1, ?2, ?3)",
            rusqlite::params![id, now, tabs_json],
        )?;
        // Keep only the latest 20 sessions.
        conn.execute(
            "DELETE FROM sessions WHERE id NOT IN (
                SELECT id FROM sessions ORDER BY created_at DESC LIMIT 20
            )",
            [],
        )?;
        Ok(())
    }

    pub async fn restore_last_session(&self) -> Result<()> {
        let conn = self.db.conn().await;
        let row: Option<(String, String)> = conn
            .query_row(
                "SELECT id, tabs_json FROM sessions ORDER BY created_at DESC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .ok();
        if let Some((_, tabs_json)) = row {
            let tabs: Vec<Tab> = serde_json::from_str(&tabs_json)?;
            let mut tab_lock = self.tabs.lock().await;
            for t in tabs {
                let mut restored = t.clone();
                restored.discarded = true; // start cold, load on activation
                restored.loading = false;
                tab_lock.insert(restored.id.clone(), restored);
            }
        }
        Ok(())
    }

    pub async fn split(&self, id: &str, with: &str) -> Option<(Tab, Tab)> {
        let mut tabs = self.tabs.lock().await;
        {
            let a = tabs.get_mut(id)?;
            a.split_with = Some(with.to_string());
        }
        {
            let b = tabs.get_mut(with)?;
            b.split_with = Some(id.to_string());
        }
        let a = tabs.get(id)?.clone();
        let b = tabs.get(with)?.clone();
        Some((a, b))
    }
}
