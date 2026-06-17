//! Global app state shared across all Tauri commands.

use crate::storage::Database;
use crate::tab::TabManager;
use crate::memory::MemoryConfig;
use crate::adblock::AdBlockEngine;
use crate::privacy::PrivacyEngine;
use crate::workspace::WorkspaceManager;
use crate::settings::Settings;
use anyhow::Result;
use parking_lot::RwLock;
use std::sync::Arc;
use tauri::{App, Manager};

pub struct AppState {
    pub db: Arc<Database>,
    pub tabs: Arc<TabManager>,
    pub adblock: Arc<RwLock<AdBlockEngine>>,
    pub privacy: Arc<RwLock<PrivacyEngine>>,
    pub workspaces: Arc<WorkspaceManager>,
    pub settings: Arc<RwLock<Settings>>,
    pub memory_config: Arc<RwLock<MemoryConfig>>,
    pub app_handle: parking_lot::Mutex<Option<tauri::AppHandle>>,
}

impl AppState {
    pub fn new() -> Result<Self> {
        // Open the persistent database in the user's data dir.
        // For incognito windows we open a separate in-memory DB.
        let data_dir = tauri::api::path::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("LunarBrowser");
        std::fs::create_dir_all(&data_dir)?;
        let db_path = data_dir.join("lunar.db");
        let db = Arc::new(Database::open(db_path)?);

        let adblock = Arc::new(RwLock::new(AdBlockEngine::new()));
        let privacy = Arc::new(RwLock::new(PrivacyEngine::new()));
        let workspaces = Arc::new(WorkspaceManager::new(db.clone()));
        let settings = Arc::new(RwLock::new(Settings::load(&db)?));
        let memory_config = Arc::new(RwLock::new(MemoryConfig::aggressive()));
        let tabs = Arc::new(TabManager::new(db.clone(), memory_config.clone()));

        Ok(Self {
            db,
            tabs,
            adblock,
            privacy,
            workspaces,
            settings,
            memory_config,
            app_handle: parking_lot::Mutex::new(None),
        })
    }

    pub fn on_app_ready(&self, app: &App) -> Result<()> {
        *self.app_handle.lock() = Some(app.handle());
        // Warm up ad-block engine in background.
        let adblock = self.adblock.clone();
        let handle = app.handle();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = AdBlockEngine::warm_up(adblock.clone(), handle).await {
                log::warn!("adblock warm-up failed: {e}");
            }
        });
        // Restore last session if setting enabled.
        if self.settings.read().restore_last_session {
            let tabs = self.tabs.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = tabs.restore_last_session().await {
                    log::warn!("session restore failed: {e}");
                }
            });
        }
        Ok(())
    }

    pub fn handle(&self) -> tauri::AppHandle {
        self.app_handle.lock().clone().expect("app not ready")
    }
}
