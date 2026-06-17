// main.js — Lunar Browser frontend entry point.
// Wires up all components and global keyboard shortcuts.

import { on } from './lib/tauri-bridge.js';
import { initTabStrip, getActiveTabId, setActiveTab } from './components/tabstrip.js';
import { initOmnibox, updateOmniboxForTab } from './components/omnibox.js';
import { initPalette, openPalette } from './components/palette.js';
import { initWorkspaces, getActiveWorkspace } from './components/workspaces.js';
import { initMenu } from './components/menu.js';
import { initDownloads } from './components/downloads.js';
import { initSettings } from './components/settings.js';
import { initWebviewHost, navigateActiveTab } from './components/webview-host.js';
import { initNewTab } from './components/newtab.js';
import { memory } from './lib/tauri-bridge.js';

async function boot() {
  // Detect OS via Tauri (or fallback to user-agent).
  const os = detectOs();
  document.body.dataset.os = os;

  // Initialize all components.
  await initWorkspaces();
  await initTabStrip();
  initOmnibox();
  initPalette();
  initMenu();
  initDownloads();
  initSettings();
  initWebviewHost();
  initNewTab();

  // Wire backend → frontend events.
  on('lunar://tab/discarded', (id) => {
    document.querySelector(`.tab-card[data-id="${id}"]`)?.setAttribute('data-discarded', 'true');
  });
  on('lunar://memory/stats', (stats) => updateRamMeter(stats));
  on('lunar://memory/gc-request', () => {
    // Drop any cached canvases / heavy DOM in the frontend.
    document.querySelectorAll('canvas[data-cache]').forEach(c => c.remove());
  });
  on('lunar://tab/reload', ({ id, bypass_cache }) => {
    const iframe = document.querySelector(`webview[data-tab-id="${id}"], iframe[data-tab-id="${id}"]`);
    if (iframe) {
      if (bypass_cache) {
        const src = iframe.src;
        iframe.src = 'about:blank';
        setTimeout(() => iframe.src = src, 50);
      } else {
        iframe.reload ? iframe.reload() : (iframe.src = iframe.src);
      }
    }
  });
  on('lunar://download/progress', ({ id, downloaded, total }) => {
    const bar = document.querySelector(`.download-item[data-id="${id}"] .download-item__progress-bar`);
    if (bar && total > 0) bar.style.width = `${(downloaded / total) * 100}%`;
  });
  on('lunar://download/finished', ({ id, status }) => {
    const bar = document.querySelector(`.download-item[data-id="${id}"] .download-item__progress-bar`);
    if (bar) bar.dataset.status = status;
  });

  // Global keyboard shortcuts.
  document.addEventListener('keydown', onGlobalKey);

  // Poll memory stats on a slow cadence (watchdog emits every 30s anyway).
  setInterval(async () => {
    try {
      const stats = await memory.stats();
      updateRamMeter(stats);
    } catch {}
  }, 5000);

  console.log('[Lunar] Browser ready.');
}

function detectOs() {
  const ua = navigator.userAgent.toLowerCase();
  if (ua.includes('mac')) return 'macos';
  if (ua.includes('win')) return 'windows';
  if (ua.includes('linux')) return 'linux';
  return 'unknown';
}

function updateRamMeter(stats) {
  const meter = document.getElementById('ramMeter');
  const fill = document.getElementById('ramMeterFill');
  const label = document.getElementById('ramMeterLabel');
  if (!meter || !fill || !label) return;
  const pct = Math.min(100, (stats.rss_mb / stats.budget_mb) * 100);
  fill.style.width = `${pct.toFixed(0)}%`;
  meter.dataset.over = stats.over_budget ? 'true' : 'false';
  label.textContent = `${stats.rss_mb} / ${stats.budget_mb} MB · ${stats.tabs_total} tabs`;
  label.title = `Strategy: ${stats.strategy}\nDiscarded: ${stats.tabs_discarded}\nPinned: ${stats.tabs_pinned}`;
}

function onGlobalKey(e) {
  const cmd = e.metaKey || e.ctrlKey;
  if (cmd && e.key === 't') { e.preventDefault(); newTab(); }
  else if (cmd && e.key === 'w') { e.preventDefault(); closeActiveTab(); }
  else if (cmd && e.key === 'r') { e.preventDefault(); reloadActiveTab(e.shiftKey); }
  else if (cmd && e.key === 'k') { e.preventDefault(); openPalette(); }
  else if (cmd && e.key === 'j') { e.preventDefault(); toggleDownloads(); }
  else if (cmd && e.shiftKey && e.key.toLowerCase() === 'n') { e.preventDefault(); openIncognito(); }
  else if (cmd && e.key === '\\') { e.preventDefault(); toggleSplit(); }
  else if (cmd && e.key >= '1' && e.key <= '9') { e.preventDefault(); switchToTabByIndex(parseInt(e.key) - 1); }
  else if (e.key === 'Escape') { closeAllPanels(); }
}

async function newTab() {
  const { tabs } = await import('./lib/tauri-bridge.js');
  const ws = getActiveWorkspace();
  const tab = await tabs.new('lunar://newtab', ws?.id ?? null, false);
  // Tab strip will pick this up via state poll. For snappy UX, push directly:
  document.dispatchEvent(new CustomEvent('lunar:tab-added', { detail: tab }));
  setActiveTab(tab.id);
  navigateActiveTab(tab);
  document.getElementById('omniboxInput')?.focus();
}

async function closeActiveTab() {
  const id = getActiveTabId();
  if (!id) return;
  const { tabs } = await import('./lib/tauri-bridge.js');
  await tabs.close(id);
  document.dispatchEvent(new CustomEvent('lunar:tab-closed', { detail: id }));
}

async function reloadActiveTab(bypassCache = false) {
  const id = getActiveTabId();
  if (!id) return;
  const { tabs } = await import('./lib/tauri-bridge.js');
  await tabs.reload(id, bypassCache);
}

async function openIncognito() {
  const { incognito } = await import('./lib/tauri-bridge.js');
  await incognito.openWindow();
}

function toggleDownloads() {
  document.getElementById('downloadsPanel').hidden = !document.getElementById('downloadsPanel').hidden;
}

function toggleSplit() {
  document.dispatchEvent(new CustomEvent('lunar:toggle-split'));
}

function switchToTabByIndex(idx) {
  document.dispatchEvent(new CustomEvent('lunar:switch-tab-by-index', { detail: idx }));
}

function closeAllPanels() {
  document.getElementById('cmdPalette').hidden = true;
  document.getElementById('downloadsPanel').hidden = true;
  document.getElementById('menuPanel').hidden = true;
  document.getElementById('settingsOverlay').hidden = true;
}

// Boot.
boot().catch(e => console.error('[Lunar] boot failed:', e));
