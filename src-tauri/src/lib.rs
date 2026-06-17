//! Lunar Browser — Tauri backend.
//!
//! Module map:
//! - `state`      → global app state (shared across all windows/commands)
//! - `tab`        → tab lifecycle + aggressive memory strategy
//! - `memory`     → RAM budget enforcement, auto-discard scheduler
//! - `bookmarks`  → bookmark storage (SQLite)
//! - `history`    → history storage (SQLite)
//! - `downloads`  → download manager with pause/resume
//! - `adblock`    → built-in ad-block + filter list engine
//! - `privacy`    → tracker blocking, fingerprinting protection
//! - `incognito`  → incognito mode isolation
//! - `extensions` → minimal extension host (content scripts only)
//! - `storage`    → SQLite connection pool, migrations
//! - `commands`   → all `#[tauri::command]` functions exposed to the frontend

pub mod state;
pub mod tab;
pub mod memory;
pub mod bookmarks;
pub mod history;
pub mod downloads;
pub mod adblock;
pub mod privacy;
pub mod incognito;
pub mod extensions;
pub mod storage;
pub mod commands;
pub mod workspace;
pub mod settings;

use state::AppState;
use std::sync::Arc;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .try_init()
        .ok();

    log::info!("Lunar Browser starting up…");

    let state = AppState::new().expect("failed to initialize app state");
    let shared = Arc::new(state);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_notification::init())
        .manage(shared.clone())
        .setup({
            let shared = shared.clone();
            move |app| {
                shared.on_app_ready(app)?;
                // Spawn the memory watchdog that enforces the 1GB / 20-tab budget.
                memory::spawn_watchdog(shared.clone());
                // Spawn the tab auto-discard scheduler (60s aggressive).
                memory::spawn_discard_scheduler(shared.clone());
                log::info!("Lunar Browser ready.");
                Ok(())
            }
        })
        .invoke_handler(tauri::generate_handler![
            // Tab lifecycle
            commands::tab_new,
            commands::tab_close,
            commands::tab_activate,
            commands::tab_reload,
            commands::tab_navigate,
            commands::tab_go_back,
            commands::tab_go_forward,
            commands::tab_pin,
            commands::tab_mute,
            commands::tab_discard,
            commands::tab_restore,
            commands::tab_list,
            commands::tab_get_state,
            commands::tab_split,
            // Workspaces
            commands::workspace_create,
            commands::workspace_switch,
            commands::workspace_list,
            commands::workspace_delete,
            // Bookmarks
            commands::bookmark_add,
            commands::bookmark_remove,
            commands::bookmark_list,
            commands::bookmark_search,
            commands::bookmark_folder_create,
            // History
            commands::history_add,
            commands::history_list,
            commands::history_search,
            commands::history_clear,
            commands::history_remove,
            // Downloads
            commands::downloads_start,
            commands::downloads_pause,
            commands::downloads_resume,
            commands::downloads_cancel,
            commands::downloads_list,
            // Ad-block / privacy
            commands::adblock_status,
            commands::adblock_reload_lists,
            commands::adblock_toggle,
            commands::privacy_block_request,
            commands::privacy_set_strict,
            // Incognito
            commands::incognito_open_window,
            commands::incognito_close_all,
            // Extensions
            commands::extensions_install,
            commands::extensions_list,
            commands::extensions_toggle,
            commands::extensions_remove,
            // Memory
            commands::memory_stats,
            commands::memory_force_gc,
            commands::memory_set_budget_mb,
            // Settings
            commands::settings_get,
            commands::settings_set,
            commands::settings_reset,
            // Search
            commands::search_suggest,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Lunar Browser");
}
