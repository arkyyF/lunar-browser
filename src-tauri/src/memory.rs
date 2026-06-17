//! Memory budget enforcement + tab auto-discard scheduler.
//!
//! Aggressive policy (default):
//!   - max_active_tabs: 20  → soft cap, evicts oldest non-pinned when exceeded
//!   - discard_after_secs: 60 → background tabs released after 60s idle
//!   - hard_budget_mb: 1024 → if process RSS exceeds this, force-discard
//!     until under budget; if still over, ask JS layer to GC
//!   - gc_interval_secs: 30  → check budget every 30s

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use sysinfo::{MemoryRefreshKind, ProcessRefreshKind, RefreshKind, System};
use tauri::Emitter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub max_active_tabs: usize,
    pub discard_after_secs: u64,
    pub hard_budget_mb: u64,
    pub gc_interval_secs: u64,
    pub strategy: MemoryStrategy,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryStrategy {
    Aggressive,
    Balanced,
    Light,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self::aggressive()
    }
}

impl MemoryConfig {
    pub fn aggressive() -> Self {
        Self {
            max_active_tabs: 20,
            discard_after_secs: 60,
            hard_budget_mb: 1024,
            gc_interval_secs: 30,
            strategy: MemoryStrategy::Aggressive,
        }
    }

    pub fn balanced() -> Self {
        Self {
            max_active_tabs: 25,
            discard_after_secs: 300,
            hard_budget_mb: 1536,
            gc_interval_secs: 60,
            strategy: MemoryStrategy::Balanced,
        }
    }

    pub fn light() -> Self {
        Self {
            max_active_tabs: 40,
            discard_after_secs: 1800,
            hard_budget_mb: 2560,
            gc_interval_secs: 120,
            strategy: MemoryStrategy::Light,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub rss_mb: u64,
    pub virtual_mb: u64,
    pub budget_mb: u64,
    pub tabs_total: usize,
    pub tabs_discarded: usize,
    pub tabs_pinned: usize,
    pub over_budget: bool,
    pub strategy: MemoryStrategy,
}

pub fn spawn_watchdog(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut sys = System::new();
        let pid = sysinfo::get_current_pid().unwrap_or(sysinfo::Pid::from(0));
        let interval = Duration::from_secs(state.memory_config.read().gc_interval_secs);
        loop {
            tokio::time::sleep(interval).await;
            sys.refresh_processes_specifics(
                sysinfo::ProcessesToUpdate::All,
                true,
                ProcessRefreshKind::nothing().with_memory(),
            );
            let rss_mb = sys
                .process(pid)
                .map(|p| p.memory() / 1024) // KB → MB
                .unwrap_or(0);
            let budget = state.memory_config.read().hard_budget_mb;
            if rss_mb > budget {
                log::warn!("memory over budget: {}MB > {}MB", rss_mb, budget);
                force_discard_until_under(&state, &mut sys, pid, budget).await;
                // Emit a GC signal to the frontend (drops cached images, etc.)
                if let Some(handle) = state.app_handle.lock().as_ref() {
                    let _ = handle.emit("lunar://memory/gc-request", rss_mb);
                }
            }
            // Broadcast updated stats.
            let stats = collect_stats(&state, &sys, pid).await;
            if let Some(handle) = state.app_handle.lock().as_ref() {
                let _ = handle.emit("lunar://memory/stats", &stats);
            }
        }
    });
}

pub fn spawn_discard_scheduler(state: Arc<AppState>) {
    tokio::spawn(async move {
        loop {
            let idle_secs = state.memory_config.read().discard_after_secs;
            tokio::time::sleep(Duration::from_secs(15)).await;
            let now = chrono::Utc::now();
            let active_id = state.tabs.active_tab_id.lock().await.clone();
            let mut to_discard = Vec::new();
            {
                let tabs = state.tabs.tabs.lock().await;
                for t in tabs.values() {
                    if t.pinned || t.discarded {
                        continue;
                    }
                    if active_id.as_deref() == Some(&t.id) {
                        continue;
                    }
                    let idle = (now - t.last_active).num_seconds().max(0) as u64;
                    if idle >= idle_secs {
                        to_discard.push(t.id.clone());
                    }
                }
            }
            for id in to_discard {
                state.tabs.discard(&id).await;
                if let Some(handle) = state.app_handle.lock().as_ref() {
                    let _ = handle.emit("lunar://tab/discarded", &id);
                }
            }
        }
    });
}

async fn force_discard_until_under(
    state: &Arc<AppState>,
    sys: &mut System,
    pid: sysinfo::Pid,
    budget_mb: u64,
) {
    // Order tabs by oldest active time. Drop until under budget or none left.
    let mut order: Vec<(String, chrono::DateTime<chrono::Utc>)> = {
        let tabs = state.tabs.tabs.lock().await;
        tabs.values()
            .filter(|t| !t.pinned && !t.discarded)
            .map(|t| (t.id.clone(), t.last_active))
            .collect()
    };
    order.sort_by_key(|(_, ts)| *ts);
    for (id, _) in order {
        sys.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::All,
            false,
            ProcessRefreshKind::nothing().with_memory(),
        );
        let rss = sys.process(pid).map(|p| p.memory() / 1024).unwrap_or(0);
        if rss <= budget_mb {
            return;
        }
        state.tabs.discard(&id).await;
        log::info!("force-discarded tab {} to relieve RAM (now {}MB)", id, rss);
        if let Some(handle) = state.app_handle.lock().as_ref() {
            let _ = handle.emit("lunar://tab/discarded", &id);
        }
    }
}

async fn collect_stats(state: &Arc<AppState>, sys: &System, pid: sysinfo::Pid) -> MemoryStats {
    let rss_mb = sys.process(pid).map(|p| p.memory() / 1024).unwrap_or(0);
    let virtual_mb = sys.process(pid).map(|p| p.virtual_memory() / 1024).unwrap_or(0);
    let budget_mb = state.memory_config.read().hard_budget_mb;
    let (total, discarded, pinned) = {
        let tabs = state.tabs.tabs.lock().await;
        let total = tabs.len();
        let discarded = tabs.values().filter(|t| t.discarded).count();
        let pinned = tabs.values().filter(|t| t.pinned).count();
        (total, discarded, pinned)
    };
    MemoryStats {
        rss_mb,
        virtual_mb,
        budget_mb,
        tabs_total: total,
        tabs_discarded: discarded,
        tabs_pinned: pinned,
        over_budget: rss_mb > budget_mb,
        strategy: state.memory_config.read().strategy,
    }
}
