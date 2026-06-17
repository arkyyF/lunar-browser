//! Tauri commands — the bridge between Rust backend and JS frontend.
//!
//! All commands take `state: tauri::State<'_, Arc<AppState>>` to access
//! shared services. Heavy work is async.

use crate::state::AppState;
use crate::tab::Tab;
use crate::memory::{MemoryConfig, MemoryStats, MemoryStrategy};
use crate::bookmarks::{Bookmark, BookmarkFolder};
use crate::history::HistoryEntry;
use crate::downloads::Download;
use crate::workspace::Workspace;
use crate::privacy::PrivacyLevel;
use crate::extensions::Extension;
use crate::settings::Settings;
use std::sync::Arc;
use tauri::State;

// ───────────────── Tabs ─────────────────

#[tauri::command]
pub async fn tab_new(
    state: State<'_, Arc<AppState>>,
    url: Option<String>,
    workspace_id: Option<String>,
    incognito: Option<bool>,
) -> Result<Tab, String> {
    let url = url.unwrap_or_else(|| "lunar://newtab".into());
    let incognito = incognito.unwrap_or(false);
    Ok(state.tabs.create_tab(url, workspace_id, incognito).await)
}

#[tauri::command]
pub async fn tab_close(state: State<'_, Arc<AppState>>, id: String) -> Result<Option<Tab>, String> {
    Ok(state.tabs.close_tab(&id).await)
}

#[tauri::command]
pub async fn tab_activate(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
    state.tabs.set_active(&id).await;
    Ok(())
}

#[tauri::command]
pub async fn tab_reload(state: State<'_, Arc<AppState>>, id: String, bypass_cache: Option<bool>) -> Result<(), String> {
    state.tabs.update(&id, |t| { t.loading = true; }).await;
    let bypass = bypass_cache.unwrap_or(false);
    if let Some(handle) = state.app_handle.lock().as_ref() {
        let _ = handle.emit("lunar://tab/reload", serde_json::json!({"id": id, "bypass_cache": bypass}));
    }
    Ok(())
}

#[tauri::command]
pub async fn tab_navigate(
    state: State<'_, Arc<AppState>>,
    id: String,
    url: String,
) -> Result<Tab, String> {
    let tab = state.tabs.update(&id, |t| {
        t.history_stack.back.push(t.url.clone());
        t.history_stack.forward.clear();
        t.url = url.clone();
        t.loading = true;
        t.discarded = false;
        t.last_active = chrono::Utc::now();
    }).await.ok_or("tab not found")?;
    let _ = state.history.add(tab.url.clone(), Some(tab.title.clone()), tab.favicon.clone(), tab.incognito).await;
    Ok(tab)
}

#[tauri::command]
pub async fn tab_go_back(state: State<'_, Arc<AppState>>, id: String) -> Result<Option<Tab>, String> {
    Ok(state.tabs.update(&id, |t| {
        if let Some(prev) = t.history_stack.back.pop() {
            t.history_stack.forward.push(t.url.clone());
            t.url = prev;
            t.loading = true;
            t.last_active = chrono::Utc::now();
        }
    }).await)
}

#[tauri::command]
pub async fn tab_go_forward(state: State<'_, Arc<AppState>>, id: String) -> Result<Option<Tab>, String> {
    Ok(state.tabs.update(&id, |t| {
        if let Some(next) = t.history_stack.forward.pop() {
            t.history_stack.back.push(t.url.clone());
            t.url = next;
            t.loading = true;
            t.last_active = chrono::Utc::now();
        }
    }).await)
}

#[tauri::command]
pub async fn tab_pin(state: State<'_, Arc<AppState>>, id: String, pinned: bool) -> Result<Option<Tab>, String> {
    Ok(state.tabs.update(&id, |t| { t.pinned = pinned; }).await)
}

#[tauri::command]
pub async fn tab_mute(state: State<'_, Arc<AppState>>, id: String, muted: bool) -> Result<Option<Tab>, String> {
    Ok(state.tabs.update(&id, |t| { t.muted = muted; }).await)
}

#[tauri::command]
pub async fn tab_discard(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
    state.tabs.discard(&id).await;
    Ok(())
}

#[tauri::command]
pub async fn tab_restore(state: State<'_, Arc<AppState>>, id: String) -> Result<Option<Tab>, String> {
    Ok(state.tabs.restore(&id).await)
}

#[tauri::command]
pub async fn tab_list(state: State<'_, Arc<AppState>>) -> Result<Vec<Tab>, String> {
    Ok(state.tabs.list().await)
}

#[tauri::command]
pub async fn tab_get_state(state: State<'_, Arc<AppState>>, id: String) -> Result<Option<Tab>, String> {
    Ok(state.tabs.get(&id).await)
}

#[tauri::command]
pub async fn tab_split(state: State<'_, Arc<AppState>>, id: String, with: String) -> Result<Option<(Tab, Tab)>, String> {
    Ok(state.tabs.split(&id, &with).await)
}

// ───────────────── Workspaces ─────────────────

#[tauri::command]
pub async fn workspace_create(
    state: State<'_, Arc<AppState>>,
    name: String,
    icon: Option<String>,
    color: Option<String>,
) -> Result<Workspace, String> {
    Ok(state.workspaces.create(name, icon.unwrap_or_else(|| "moon".into()), color.unwrap_or_else(|| "#C0C8D0".into())).await
        .map_err(|e| e.to_string())?)
}

#[tauri::command]
pub async fn workspace_switch(state: State<'_, Arc<AppState>>, _id: String) -> Result<Vec<Tab>, String> {
    // In v1 we just emit an event — the frontend filters tabs by workspace_id.
    let tabs: Vec<Tab> = state.tabs.list().await
        .into_iter()
        .filter(|t| t.workspace_id.as_deref() == Some(&_id))
        .collect();
    Ok(tabs)
}

#[tauri::command]
pub async fn workspace_list(state: State<'_, Arc<AppState>>) -> Result<Vec<Workspace>, String> {
    Ok(state.workspaces.list().await.map_err(|e| e.to_string())?)
}

#[tauri::command]
pub async fn workspace_delete(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
    state.workspaces.delete(&id).await.map_err(|e| e.to_string())?;
    Ok(())
}

// ───────────────── Bookmarks ─────────────────

#[tauri::command]
pub async fn bookmark_add(
    state: State<'_, Arc<AppState>>,
    url: String,
    title: String,
    favicon: Option<String>,
    folder_id: Option<String>,
) -> Result<Bookmark, String> {
    let store = crate::bookmarks::BookmarkStore::new(state.db.clone());
    Ok(store.add(url, title, favicon, folder_id).await.map_err(|e| e.to_string())?)
}

#[tauri::command]
pub async fn bookmark_remove(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
    let store = crate::bookmarks::BookmarkStore::new(state.db.clone());
    store.remove(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn bookmark_list(state: State<'_, Arc<AppState>>, folder_id: Option<String>) -> Result<Vec<Bookmark>, String> {
    let store = crate::bookmarks::BookmarkStore::new(state.db.clone());
    Ok(store.list(folder_id).await.map_err(|e| e.to_string())?)
}

#[tauri::command]
pub async fn bookmark_search(state: State<'_, Arc<AppState>>, query: String) -> Result<Vec<Bookmark>, String> {
    let store = crate::bookmarks::BookmarkStore::new(state.db.clone());
    Ok(store.search(&query).await.map_err(|e| e.to_string())?)
}

#[tauri::command]
pub async fn bookmark_folder_create(
    state: State<'_, Arc<AppState>>,
    name: String,
    parent_id: Option<String>,
) -> Result<BookmarkFolder, String> {
    let store = crate::bookmarks::BookmarkStore::new(state.db.clone());
    Ok(store.create_folder(name, parent_id).await.map_err(|e| e.to_string())?)
}

// ───────────────── History ─────────────────

#[tauri::command]
pub async fn history_add(
    state: State<'_, Arc<AppState>>,
    url: String,
    title: Option<String>,
    favicon: Option<String>,
    incognito: Option<bool>,
) -> Result<(), String> {
    let store = crate::history::HistoryStore::new(state.db.clone());
    store.add(url, title, favicon, incognito.unwrap_or(false)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn history_list(
    state: State<'_, Arc<AppState>>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<HistoryEntry>, String> {
    let store = crate::history::HistoryStore::new(state.db.clone());
    Ok(store.list(limit.unwrap_or(100), offset.unwrap_or(0)).await.map_err(|e| e.to_string())?)
}

#[tauri::command]
pub async fn history_search(state: State<'_, Arc<AppState>>, query: String, limit: Option<i64>) -> Result<Vec<HistoryEntry>, String> {
    let store = crate::history::HistoryStore::new(state.db.clone());
    Ok(store.search(&query, limit.unwrap_or(50)).await.map_err(|e| e.to_string())?)
}

#[tauri::command]
pub async fn history_clear(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let store = crate::history::HistoryStore::new(state.db.clone());
    store.clear().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn history_remove(state: State<'_, Arc<AppState>>, id: i64) -> Result<(), String> {
    let store = crate::history::HistoryStore::new(state.db.clone());
    store.remove(id).await.map_err(|e| e.to_string())
}

// ───────────────── Downloads ─────────────────

#[tauri::command]
pub async fn downloads_start(
    state: State<'_, Arc<AppState>>,
    url: String,
    save_dir: Option<String>,
    tab_id: Option<String>,
    app: tauri::AppHandle,
) -> Result<Download, String> {
    let mgr = crate::downloads::DownloadManager::new(state.db.clone());
    let save_dir = save_dir
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(&state.settings.read().download_dir));
    Ok(mgr.start(url, save_dir, tab_id, app).await.map_err(|e| e.to_string())?)
}

#[tauri::command]
pub async fn downloads_pause(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
    let mgr = crate::downloads::DownloadManager::new(state.db.clone());
    mgr.pause(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn downloads_resume(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
    let mgr = crate::downloads::DownloadManager::new(state.db.clone());
    mgr.resume(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn downloads_cancel(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
    let mgr = crate::downloads::DownloadManager::new(state.db.clone());
    mgr.cancel(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn downloads_list(state: State<'_, Arc<AppState>>) -> Result<Vec<Download>, String> {
    let mgr = crate::downloads::DownloadManager::new(state.db.clone());
    Ok(mgr.list().await.map_err(|e| e.to_string())?)
}

// ───────────────── Ad-block / Privacy ─────────────────

#[tauri::command]
pub async fn adblock_status(state: State<'_, Arc<AppState>>) -> Result<serde_json::Value, String> {
    let g = state.adblock.read();
    Ok(serde_json::json!({
        "enabled": g.enabled,
        "lists": g.lists,
        "last_update": g.last_update.map(|t| t.to_rfc3339()),
    }))
}

#[tauri::command]
pub async fn adblock_reload_lists(state: State<'_, Arc<AppState>>, app: tauri::AppHandle) -> Result<(), String> {
    crate::adblock::AdBlockEngine::warm_up(state.adblock.clone(), app).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn adblock_toggle(state: State<'_, Arc<AppState>>, enabled: bool) -> Result<(), String> {
    state.adblock.write().set_enabled(enabled);
    Ok(())
}

#[tauri::command]
pub async fn privacy_block_request(
    state: State<'_, Arc<AppState>>,
    request_url: String,
    source_url: String,
    resource_type: String,
) -> Result<bool, String> {
    let g = state.privacy.read();
    Ok(g.should_block(&request_url, &source_url, &resource_type))
}

#[tauri::command]
pub async fn privacy_set_strict(state: State<'_, Arc<AppState>>, strict: bool) -> Result<(), String> {
    let level = if strict { PrivacyLevel::Strict } else { PrivacyLevel::Standard };
    state.privacy.write().set_level(level);
    Ok(())
}

// ───────────────── Incognito ─────────────────

#[tauri::command]
pub async fn incognito_open_window(
    state: State<'_, Arc<AppState>>,
    url: Option<String>,
) -> Result<crate::incognito::IncognitoWindow, String> {
    Ok(crate::incognito::open_incognito_window(state.inner().clone(), url).map_err(|e| e.to_string())?)
}

#[tauri::command]
pub async fn incognito_close_all(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    crate::incognito::close_all_incognito(state.inner()).map_err(|e| e.to_string())
}

// ───────────────── Extensions ─────────────────

#[tauri::command]
pub async fn extensions_install(state: State<'_, Arc<AppState>>, manifest_json: String) -> Result<Extension, String> {
    let host = crate::extensions::ExtensionHost::new(state.db.clone());
    Ok(host.install(manifest_json).await.map_err(|e| e.to_string())?)
}

#[tauri::command]
pub async fn extensions_list(state: State<'_, Arc<AppState>>) -> Result<Vec<Extension>, String> {
    let host = crate::extensions::ExtensionHost::new(state.db.clone());
    Ok(host.list().await.map_err(|e| e.to_string())?)
}

#[tauri::command]
pub async fn extensions_toggle(state: State<'_, Arc<AppState>>, id: String, enabled: bool) -> Result<(), String> {
    let host = crate::extensions::ExtensionHost::new(state.db.clone());
    host.set_enabled(&id, enabled).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn extensions_remove(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
    let host = crate::extensions::ExtensionHost::new(state.db.clone());
    host.remove(&id).await.map_err(|e| e.to_string())
}

// ───────────────── Memory ─────────────────

#[tauri::command]
pub async fn memory_stats(state: State<'_, Arc<AppState>>) -> Result<MemoryStats, String> {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
    let mut sys = System::new();
    let pid = sysinfo::get_current_pid().unwrap_or(sysinfo::Pid::from(0));
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All, true,
        ProcessRefreshKind::nothing().with_memory(),
    );
    let rss = sys.process(pid).map(|p| p.memory() / 1024).unwrap_or(0);
    let vss = sys.process(pid).map(|p| p.virtual_memory() / 1024).unwrap_or(0);
    let budget = state.memory_config.read().hard_budget_mb;
    let (total, discarded, pinned) = {
        let tabs = state.tabs.tabs.lock().await;
        (tabs.len(), tabs.values().filter(|t| t.discarded).count(), tabs.values().filter(|t| t.pinned).count())
    };
    Ok(MemoryStats {
        rss_mb: rss, virtual_mb: vss, budget_mb: budget,
        tabs_total: total, tabs_discarded: discarded, tabs_pinned: pinned,
        over_budget: rss > budget,
        strategy: state.memory_config.read().strategy,
    })
}

#[tauri::command]
pub async fn memory_force_gc(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    if let Some(handle) = state.app_handle.lock().as_ref() {
        let _ = handle.emit("lunar://memory/gc-request", ());
    }
    Ok(())
}

#[tauri::command]
pub async fn memory_set_budget_mb(state: State<'_, Arc<AppState>>, mb: u64, strategy: Option<String>) -> Result<(), String> {
    let mut cfg = state.memory_config.write();
    cfg.hard_budget_mb = mb;
    if let Some(s) = strategy {
        cfg.strategy = match s.as_str() {
            "balanced" => MemoryStrategy::Balanced,
            "light" => MemoryStrategy::Light,
            _ => MemoryStrategy::Aggressive,
        };
        match cfg.strategy {
            MemoryStrategy::Aggressive => *cfg = MemoryConfig::aggressive(),
            MemoryStrategy::Balanced => *cfg = MemoryConfig::balanced(),
            MemoryStrategy::Light => *cfg = MemoryConfig::light(),
        }
        cfg.hard_budget_mb = mb;
    }
    Ok(())
}

// ───────────────── Settings ─────────────────

#[tauri::command]
pub async fn settings_get(state: State<'_, Arc<AppState>>) -> Result<Settings, String> {
    Ok(state.settings.read().clone())
}

#[tauri::command]
pub async fn settings_set(
    state: State<'_, Arc<AppState>>,
    key: String,
    value: serde_json::Value,
) -> Result<(), String> {
    {
        let mut s = state.settings.write();
        match key.as_str() {
            "default_search_engine" => s.default_search_engine = value.as_str().unwrap_or("google").into(),
            "homepage" => s.homepage = value.as_str().unwrap_or("lunar://newtab").into(),
            "restore_last_session" => s.restore_last_session = value.as_bool().unwrap_or(true),
            "block_ads" => s.block_ads = value.as_bool().unwrap_or(true),
            "block_trackers" => s.block_trackers = value.as_bool().unwrap_or(true),
            "strict_privacy" => s.strict_privacy = value.as_bool().unwrap_or(true),
            "memory_strategy" => s.memory_strategy = value.as_str().unwrap_or("aggressive").into(),
            "memory_budget_mb" => s.memory_budget_mb = value.as_u64().unwrap_or(1024),
            "download_dir" => s.download_dir = value.as_str().unwrap_or("~/Downloads").into(),
            "theme" => s.theme = value.as_str().unwrap_or("moonlit-dark").into(),
            "accent_color" => s.accent_color = value.as_str().unwrap_or("#C0C8D0").into(),
            "enable_extensions" => s.enable_extensions = value.as_bool().unwrap_or(true),
            "enable_split_view" => s.enable_split_view = value.as_bool().unwrap_or(true),
            "vertical_tabs" => s.vertical_tabs = value.as_bool().unwrap_or(true),
            "command_palette_enabled" => s.command_palette_enabled = value.as_bool().unwrap_or(true),
            "do_not_track" => s.do_not_track = value.as_bool().unwrap_or(true),
            "https_only" => s.https_only = value.as_bool().unwrap_or(true),
            _ => { s.extras.insert(key, value); }
        }
    }
    state.settings.read().save(&state.db).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn settings_reset(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    {
        let mut s = state.settings.write();
        *s = Settings::default();
    }
    state.settings.read().save(&state.db).await.map_err(|e| e.to_string())
}

// ───────────────── Search suggestions ─────────────────

#[tauri::command]
pub async fn search_suggest(
    state: State<'_, Arc<AppState>>,
    query: String,
) -> Result<Vec<SearchSuggestion>, String> {
    let engine = state.settings.read().default_search_engine.clone();
    // Use Google's OpenSearch suggest endpoint (no API key required).
    if engine == "google" && !query.trim().is_empty() {
        let url = format!(
            "https://suggestqueries.google.com/complete/search?client=firefox&q={}",
            urlencoding::encode(&query)
        );
        let client = reqwest::Client::builder()
            .user_agent("LunarBrowser/1.0")
            .timeout(std::time::Duration::from_secs(2))
            .build()
            .map_err(|e| e.to_string())?;
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                if let Ok(json) = resp.json::<Vec<serde_json::Value>>().await {
                    if let Some(suggestions) = json.get(1).and_then(|v| v.as_array()) {
                        return Ok(suggestions.iter()
                            .filter_map(|s| s.as_str().map(String::from))
                            .map(|s| SearchSuggestion { text: s, kind: "suggestion".into() })
                            .collect());
                    }
                }
            }
        }
    }
    // Fallback: history search.
    let store = crate::history::HistoryStore::new(state.db.clone());
    let h = store.search(&query, 8).await.map_err(|e| e.to_string())?;
    Ok(h.into_iter().map(|e| SearchSuggestion { text: e.url, kind: "history".into() }).collect())
}

#[derive(serde::Serialize)]
pub struct SearchSuggestion {
    pub text: String,
    pub kind: String,
}

// Tiny URL-encoding shim to avoid an extra dep.
mod urlencoding {
    pub fn encode(s: &str) -> String {
        s.chars().map(|c| match c {
            ' ' => "+".into(),
            c if c.is_alphanumeric() || "-_.~".contains(c) => c.to_string(),
            c => format!("%{:02X}", c as u8),
        }).collect()
    }
}

// Internal helper: emit events on the app handle.
trait EmitHelper {
    fn emit<E: tauri::Event>(&self, event: E, payload: impl serde::Serialize + Clone) -> tauri::Result<()>;
}
impl EmitHelper for tauri::AppHandle {
    fn emit<E: tauri::Event>(&self, event: E, payload: impl serde::Serialize + Clone) -> tauri::Result<()> {
        tauri::Emitter::emit(self, event, payload)
    }
}
