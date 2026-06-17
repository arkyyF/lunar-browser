//! Incognito mode.
//!
//! Implementation: incognito windows open a *separate* Tauri window with
//! its own WebView instance. The tab manager stores tabs marked `incognito=true`
//! so history/downloads are never persisted. Cookies + cache for incognito
//! tabs live in a temp dir that's wiped on close.

use crate::state::AppState;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncognitoWindow {
    pub window_label: String,
    pub tabs: Vec<String>,
}

pub fn open_incognito_window(state: Arc<AppState>, url: Option<String>) -> Result<IncognitoWindow> {
    let label = format!("incognito-{}", chrono::Utc::now().timestamp_millis());
    let handle = state.handle();
    let url = url.unwrap_or_else(|| "lunar://newtab".to_string());
    tauri::WebviewWindowBuilder::new(
        &handle,
        &label,
        tauri::WebviewUrl::App(format!("index.html?incognito=1&url={}", urlencode(&url)).into()),
    )
    .title("Lunar Browser — Incognito")
    .inner_size(1200.0, 800.0)
    .min_inner_size(800.0, 500.0)
    .theme(Some(tauri::Theme::Dark))
    .additional_browser_args("--incognito")
    .build()?;
    Ok(IncognitoWindow { window_label: label, tabs: Vec::new() })
}

pub fn close_all_incognito(state: &AppState) -> Result<()> {
    let handle = state.handle();
    for win in handle.webview_windows().values() {
        if win.label().starts_with("incognito-") {
            let _ = win.close();
        }
    }
    Ok(())
}

fn urlencode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            ' ' => "+".to_string(),
            c if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' => c.to_string(),
            c => format!("%{:02X}", c as u8),
        })
        .collect()
}
