//! Built-in ad-block engine (in-house, dependency-free).
//!
//! Default filter lists (downloaded async on first launch):
//!   - EasyList (https://easylist.to/easylist/easylist.txt)
//!   - EasyPrivacy (https://easylist.to/easylist/easyprivacy.txt)
//!   - uBlock Origin filters
//!
//! Lists are cached at $DATA/LunarBrowser/adblock/*.txt and re-downloaded
//! every 7 days. v1 uses simple pattern matching (substring + domain).
//! v2 may swap in the full `adblock` crate once its rmp-serde dep is fixed.

use anyhow::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::AppHandle;
use tauri::Manager;

pub struct AdBlockEngine {
    pub enabled: bool,
    pub lists: Vec<FilterList>,
    pub last_update: Option<chrono::DateTime<chrono::Utc>>,
    /// Compiled rules: each is (pattern, kind) where kind is 'domain' or 'substring'.
    pub rules: Vec<BlockRule>,
    /// Raw rule count for display.
    pub rule_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterList {
    pub name: String,
    pub url: String,
    pub enabled: bool,
    pub rules: usize,
}

#[derive(Debug, Clone)]
pub struct BlockRule {
    pub pattern: String,
    pub kind: RuleKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleKind {
    /// Match if the URL contains this substring (case-insensitive).
    Substring,
    /// Match if the URL's host equals or ends with this domain.
    Domain,
    /// Match if the URL's path starts with this prefix.
    PathPrefix,
}

impl AdBlockEngine {
    pub fn new() -> Self {
        Self {
            enabled: true,
            lists: default_lists(),
            last_update: None,
            rules: Vec::new(),
            rule_count: 0,
        }
    }

    pub fn should_block(&self, url: &str, _source_url: &str, _resource_type: &str) -> bool {
        if !self.enabled {
            return false;
        }
        let url_lower = url.to_lowercase();
        let host = url::Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(String::from))
            .unwrap_or_default();
        let path = url::Url::parse(url)
            .ok()
            .and_then(|u| u.path().parse().ok())
            .unwrap_or_default();

        for rule in &self.rules {
            let p = &rule.pattern;
            match rule.kind {
                RuleKind::Substring => {
                    if url_lower.contains(&p.to_lowercase()) {
                        return true;
                    }
                }
                RuleKind::Domain => {
                    if host == p || host.ends_with(&format!(".{}", p)) {
                        return true;
                    }
                }
                RuleKind::PathPrefix => {
                    if path.starts_with(p) {
                        return true;
                    }
                }
            }
        }
        false
    }

    pub async fn warm_up(self: Arc<RwLock<Self>>, app: AppHandle) -> Result<()> {
        let data_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("./data"));
        let adblock_dir = data_dir.join("adblock");
        std::fs::create_dir_all(&adblock_dir)?;

        let mut combined_rules: Vec<BlockRule> = Vec::new();
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
                    if let Some(rule) = parse_rule(line) {
                        combined_rules.push(rule);
                    }
                }
            }
        }

        log::info!("adblock: loaded {} rules from {} lists", combined_rules.len(), lists.len());

        let mut guard = self.write();
        guard.rules = combined_rules.clone();
        guard.rule_count = combined_rules.len();
        guard.lists = lists;
        guard.last_update = Some(chrono::Utc::now());
        Ok(())
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

/// Parse a single EasyList-style rule into a BlockRule.
/// Supports:
///   - `||example.com^`  → Domain rule
///   - `||example.com`   → Domain rule
///   - `/path/prefix*`   → PathPrefix rule
///   - `substring`       → Substring rule (default)
///   - `@@||...`         → exception (ignored in v1, treated as no-op)
fn parse_rule(line: &str) -> Option<BlockRule> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('!') || line.starts_with('[') {
        return None;
    }
    // Skip exception rules (would need allowlist support — v2 feature).
    if line.starts_with("@@") {
        return None;
    }
    // Skip rules with $ options for now (resource type filters).
    // We'll match the URL part only.
    let line = line.split('$').next().unwrap_or(line);

    // `||example.com^` → domain block
    if let Some(rest) = line.strip_prefix("||") {
        let domain = rest.trim_end_matches('^').to_lowercase();
        if domain.is_empty() {
            return None;
        }
        return Some(BlockRule { pattern: domain, kind: RuleKind::Domain });
    }
    // `|https://...` → URL prefix (treat as substring for simplicity)
    if let Some(rest) = line.strip_prefix('|') {
        return Some(BlockRule { pattern: rest.to_lowercase(), kind: RuleKind::Substring });
    }
    // `/path/prefix` → path prefix
    if line.starts_with('/') && line.len() > 1 {
        return Some(BlockRule { pattern: line.to_string(), kind: RuleKind::PathPrefix });
    }
    // Everything else → substring match
    if line.len() < 3 {
        return None; // skip very short rules (too noisy)
    }
    Some(BlockRule { pattern: line.to_lowercase(), kind: RuleKind::Substring })
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
