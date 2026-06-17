//! Built-in ad-block engine (wraps the `adblock` crate).
//!
//! Default filter lists (downloaded async on first launch):
//!   - EasyList (https://easylist.to/easylist/easylist.txt)
//!   - EasyPrivacy (https://easylist.to/easylist/easyprivacy.txt)
//!   - uBlock Origin filters (https://filters.adtidso.org/)
//!
//! Lists are cached at $DATA/LunarBrowser/adblock/*.txt and re-downloaded
//! every 7 days.

use anyhow::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::AppHandle;
use tauri::Manager;

pub struct AdBlockEngine {
    pub engine: Option<adblock::Engine>,
    pub enabled: bool,
    pub lists: Vec<FilterList>,
    pub last_update: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterList {
    pub name: String,
    pub url: String,
    pub enabled: bool,
    pub rules: usize,
}

impl AdBlockEngine {
    pub fn new() -> Self {
        Self {
            engine: None,
            enabled: true,
            lists: default_lists(),
            last_update: None,
        }
    }

    pub fn should_block(&self, url: &str, source_url: &str, resource_type: &str) -> bool {
        if !self.enabled {
            return false;
        }
        if let Some(engine) = &self.engine {
            let req = adblock::request::Request::parse(
                url,
                source_url,
                resource_type,
                false,
            ).ok();
            if let Some(req) = req {
                return engine.check_network_request(&req).matched;
            }
        }
        false
    }

    pub async fn warm_up(self: Arc<RwLock<Self>>, app: AppHandle) -> Result<()> {
        let data_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("./data"));
        let adblock_dir = data_dir.join("adblock");
        std::fs::create_dir_all(&adblock_dir)?;

        let mut combined_rules: Vec<String> = Vec::new();
        let mut lists = self.read().lists.clone();
        let client = reqwest::Client::builder()
            .user_agent("LunarBrowser/1.0")
            .timeout(std::time::Duration::from_secs(20))
            .build()?;

        for list in lists.iter_mut() {
            if !list.enabled { continue; }
            let local_path = adblock_dir.join(format!("{}.txt", sanitize(&list.name)));
            let need_download = match std::fs::metadata(&local_path) {
                Ok(meta) => meta.modified().ok()
                    .and_then(|t| t.elapsed().ok())
                    .map(|e| e > std::time::Duration::from_secs(7 * 24 * 3600))
                    .unwrap_or(true),
                Err(_) => true,
            };
            if need_download {
                log::info!("adblock: downloading {} from {}", list.name, list.url);
                match client.get(&list.url).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        let text = resp.text().await?;
                        std::fs::write(&local_path, &text)?;
                    }
                    Ok(resp) => log::warn!("adblock: {} returned {}", list.name, resp.status()),
                    Err(e) => log::warn!("adblock: {} failed: {}", list.name, e),
                }
            }
            if let Ok(text) = std::fs::read_to_string(&local_path) {
                let rule_count = text.lines().filter(|l| !l.is_empty() && !l.starts_with('!')).count();
                list.rules = rule_count;
                for line in text.lines() {
                    if line.is_empty() || line.starts_with('!') { continue; }
                    combined_rules.push(line.to_string());
                }
            }
        }

        let mut engine = adblock::Engine::new();
        let _ = engine.add_filters(&combined_rules);
        engine.optimize();
        log::info!("adblock: loaded {} rules from {} lists", combined_rules.len(), lists.len());

        let mut guard = self.write();
        guard.engine = Some(engine);
        guard.lists = lists;
        guard.last_update = Some(chrono::Utc::now());
        Ok(())
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

fn default_lists() -> Vec<FilterList> {
    vec![
        FilterList {
            name: "EasyList".into(),
            url: "https://easylist.to/easylist/easylist.txt".into(),
            enabled: true,
            rules: 0,
        },
        FilterList {
            name: "EasyPrivacy".into(),
            url: "https://easylist.to/easylist/easyprivacy.txt".into(),
            enabled: true,
            rules: 0,
        },
        FilterList {
            name: "uBlock Origin — Privacy".into(),
            url: "https://filters.adtidso.org/filters/15.txt".into(),
            enabled: true,
            rules: 0,
        },
        FilterList {
            name: "uBlock Origin — Badware".into(),
            url: "https://filters.adtidso.org/filters/7.txt".into(),
            enabled: true,
            rules: 0,
        },
    ]
}

fn sanitize(s: &str) -> String {
    s.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect()
}
