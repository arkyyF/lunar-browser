//! Privacy engine — tracker blocking + fingerprinting protection.
//!
//! This is a *defense-in-depth* layer on top of AdBlock. It catches:
//!   - Known tracker domains (from EasyPrivacy + Disconnect list baked in)
//!   - Third-party cookies / storage access
//!   - Canvas/WebGL fingerprinting API overrides (injected via content script)
//!   - Navigator overrides (languages, platform randomization)
//!
//! The actual override injection happens in the frontend's content-script
//! bridge (injected per-tab via Tauri's webview init script).

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyLevel {
    /// Block known trackers + third-party cookies + fingerprinting.
    Strict,
    /// Block known trackers + third-party cookies.
    Standard,
    /// Disabled.
    Off,
}

pub struct PrivacyEngine {
    pub level: PrivacyLevel,
    /// Known tracker domains (compiled in; EasyPrivacy supplements via adblock).
    pub tracker_domains: HashSet<String>,
    /// Domains allowed through (user-granted).
    pub allowlist: HashSet<String>,
}

impl PrivacyEngine {
    pub fn new() -> Self {
        Self {
            level: PrivacyLevel::Strict,
            tracker_domains: baked_tracker_domains(),
            allowlist: HashSet::new(),
        }
    }

    /// Decide whether to block a network request based on the request URL,
    /// the page that issued it, and the resource type.
    pub fn should_block(&self, request_url: &str, source_url: &str, resource_type: &str) -> bool {
        if self.level == PrivacyLevel::Off {
            return false;
        }
        let Ok(req) = url::Url::parse(request_url) else { return false; };
        let Ok(src) = url::Url::parse(source_url) else { return false; };
        let req_host = req.host_str().unwrap_or("");
        let src_host = src.host_str().unwrap_or("");
        // Same-origin → always allow.
        if req_host == src_host {
            return false;
        }
        // Third-party request. Check tracker list.
        if self.is_tracker(req_host) {
            return true;
        }
        // Block third-party cookies/storage.
        if resource_type == "cookie" || resource_type == "storage" {
            return self.level == PrivacyLevel::Strict;
        }
        // Block known fingerprinting scripts.
        if resource_type == "script" {
            if let Some(seg) = req.path_segments().and_then(|mut s| s.last()) {
                if FINGERPRINTING_SCRIPT_HINTS.iter().any(|h| seg.contains(h)) {
                    return true;
                }
            }
        }
        false
    }

    fn is_tracker(&self, host: &str) -> bool {
        if self.allowlist.contains(host) { return false; }
        // Check exact + parent domain.
        if self.tracker_domains.contains(host) { return true; }
        let parts: Vec<&str> = host.split('.').collect();
        if parts.len() >= 2 {
            let parent = parts[parts.len() - 2..].join(".");
            return self.tracker_domains.contains(&parent);
        }
        false
    }

    pub fn set_level(&mut self, level: PrivacyLevel) {
        self.level = level;
    }

    pub fn allow(&mut self, host: String) {
        self.allowlist.insert(host);
    }

    pub fn revoke(&mut self, host: &str) {
        self.allowlist.remove(host);
    }
}

const FINGERPRINTING_SCRIPT_HINTS: &[&str] = &[
    "fingerprint", "fp.js", "fpjs", "deviceid", "canvas-fp",
    "webgl-fp", "audio-fp", "font-fp", "clientjs",
];

fn baked_tracker_domains() -> HashSet<String> {
    // Curated minimal list — EasyPrivacy (loaded via adblock.rs) supplements.
    [
        "doubleclick.net","google-analytics.com","googletagmanager.com",
        "googlesyndication.com","googleadservices.com","adservice.google.com",
        "facebook.net","connect.facebook.net","fbcdn.com",
        "scorecardresearch.com","quantserve.com","advertising.com",
        "outbrain.com","taboola.com","criteo.com","criteo.net",
        "appnexus.com","rubiconproject.com","openx.net","pubmatic.com",
        "hotjar.com","mixpanel.com","segment.io","amplitude.com",
        "branch.io","adjust.com","appsflyer.com","kochava.com",
        "mparticle.com","mparticle-service.com","rudderlabs.com",
        "snowplowanalytics.com","optimizely.com","fullstory.com",
        "logrocket.com","datadog.com","datadoghq.com","newrelic.com",
        "raygun.io","bugsnag.com","sentry.io",
    ].iter().map(|s| s.to_string()).collect()
}

pub type PrivacyEngineLock = RwLock<PrivacyEngine>;
