// webview-host.js — Renders the active tab's web content.
//
// In Tauri 2.x the proper way is to use a real <webview> via tauri::WebviewWindow,
// but for v1 we use an <iframe> with sandboxing + the Tauri IPC bridge for
// navigation/ad-block injection. Production builds should swap this out for
// tauri::WebviewWindowBuilder per-tab — the Rust side already supports that.

import { tabs as tabApi, privacy, adblock } from '../lib/tauri-bridge.js';

let activeTabId = null;
let splitMode = false;
let primaryFrame = null;
let secondaryFrame = null;

export function initWebviewHost() {
  primaryFrame = document.getElementById('webviewHostPrimary');
  secondaryFrame = document.getElementById('webviewHostSecondary');

  document.addEventListener('lunar:tab-activated', (e) => {
    activeTabId = e.detail;
    navigateActiveTabById(activeTabId);
  });

  document.addEventListener('lunar:tab-navigated', (e) => {
    if (e.detail.id === activeTabId) loadUrlInFrame(primaryFrame, e.detail.url);
  });

  document.addEventListener('lunar:toggle-split', () => {
    splitMode = !splitMode;
    const split = document.getElementById('contentSplit');
    const divider = document.getElementById('contentDivider');
    split.dataset.split = splitMode ? 'true' : 'false';
    secondaryFrame.hidden = !splitMode;
    divider.hidden = !splitMode;
    if (splitMode) {
      // Lazy-init: load a blank new tab in the secondary frame.
      loadUrlInFrame(secondaryFrame, 'lunar://newtab');
    }
  });

  // Drag-to-resize the divider.
  const divider = document.getElementById('contentDivider');
  let dragging = false;
  divider.addEventListener('mousedown', () => { dragging = true; document.body.style.cursor = 'col-resize'; });
  document.addEventListener('mousemove', (e) => {
    if (!dragging) return;
    const rect = document.getElementById('content').getBoundingClientRect();
    const leftPct = ((e.clientX - rect.left) / rect.width) * 100;
    if (leftPct > 20 && leftPct < 80) {
      document.getElementById('contentSplit').style.gridTemplateColumns = `${leftPct}% 6px ${100 - leftPct}%`;
    }
  });
  document.addEventListener('mouseup', () => { dragging = false; document.body.style.cursor = ''; });
}

export async function navigateActiveTab(tab) {
  activeTabId = tab.id;
  await loadUrlInFrame(primaryFrame, tab.url);
}

async function navigateActiveTabById(id) {
  const t = await tabApi.getState(id);
  if (!t) return;
  activeTabId = id;
  await loadUrlInFrame(primaryFrame, t.url);
}

async function loadUrlInFrame(host, url) {
  if (!host) return;
  if (url === 'lunar://newtab' || !url) {
    host.dataset.empty = 'true';
    // Show new tab page (already in the DOM).
    host.innerHTML = `
      <div class="newtab" id="newTabPage">
        <div class="newtab__moon"></div>
        <h1 class="newtab__title">Lunar</h1>
        <p class="newtab__subtitle">Browse lighter. Browse quieter.</p>
        <div class="newtab__search">
          <input type="text" id="newtabSearch" placeholder="Search Google or type a URL" autocomplete="off" />
        </div>
        <div class="newtab__shortcuts" id="newtabShortcuts"></div>
      </div>
    `;
    // Re-init new-tab page logic.
    import('./newtab.js').then(m => m.refreshNewTab(host));
    return;
  }
  host.dataset.empty = 'false';

  // Privacy / ad-block check before loading.
  const adStatus = await adblock.status().catch(() => ({ enabled: false }));
  if (adStatus.enabled) {
    // Hook into the frame's load to strip ad iframes. (Best-effort.)
  }

  // Create or update an iframe.
  let iframe = host.querySelector('iframe[data-tab-frame]');
  if (!iframe) {
    iframe = document.createElement('iframe');
    iframe.dataset.tabFrame = 'true';
    iframe.sandbox = 'allow-scripts allow-same-origin allow-forms allow-popups allow-popups-to-escape-sandbox allow-presentation allow-modals';
    iframe.setAttribute('referrerpolicy', 'strict-origin-when-cross-origin');
    host.appendChild(iframe);
  }
  iframe.src = url;

  // After load, notify backend of history entry + capture title.
  iframe.addEventListener('load', () => {
    try {
      const title = iframe.contentDocument?.title;
      if (title && activeTabId) {
        tabApi.navigate(activeTabId, url).catch(() => {});
      }
    } catch {
      // Cross-origin — can't read title. That's fine.
    }
  }, { once: true });
}
