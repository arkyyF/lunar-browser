// menu.js — hamburger menu panel.

import { adblock, privacy, incognito, history } from '../lib/tauri-bridge.js';

export function initMenu() {
  const btn = document.getElementById('menuBtn');
  const panel = document.getElementById('menuPanel');

  btn.addEventListener('click', (e) => {
    e.stopPropagation();
    panel.hidden = !panel.hidden;
    closeOtherPanels(panel);
  });

  panel.addEventListener('click', async (e) => {
    const li = e.target.closest('li[data-action]');
    if (!li) return;
    const action = li.dataset.action;
    panel.hidden = true;
    await runAction(action);
  });

  document.addEventListener('click', (e) => {
    if (!e.target.closest('#menuPanel') && !e.target.closest('#menuBtn')) {
      panel.hidden = true;
    }
  });

  // Reflect ad-block / privacy status.
  refreshStatus();
}

async function refreshStatus() {
  const status = await adblock.status();
  document.getElementById('adblockStatus').textContent = status.enabled ? 'On' : 'Off';
  // Privacy strict state lives in settings; lazy-read on toggle.
}

async function runAction(action) {
  switch (action) {
    case 'new-tab':
      document.dispatchEvent(new KeyboardEvent('keydown', { key: 't', metaKey: true }));
      break;
    case 'new-incognito':
      await incognito.openWindow();
      break;
    case 'bookmarks':
      // For v1 we route to a special lunar://bookmarks URL via active tab.
      // (A dedicated panel could be added later.)
      toast('Bookmarks: use ⌘K to search bookmarks from the command palette.');
      break;
    case 'history':
      toast('History: use ⌘K to search history from the command palette.');
      break;
    case 'downloads':
      document.getElementById('downloadsPanel').hidden = false;
      break;
    case 'split':
      document.dispatchEvent(new CustomEvent('lunar:toggle-split'));
      break;
    case 'adblock-toggle': {
      const status = await adblock.status();
      const next = !status.enabled;
      await adblock.toggle(next);
      document.getElementById('adblockStatus').textContent = next ? 'On' : 'Off';
      toast(`Ad-block ${next ? 'enabled' : 'disabled'}`);
      break;
    }
    case 'privacy-toggle': {
      const cur = document.getElementById('privacyStatus').textContent === 'On';
      const next = !cur;
      await privacy.setStrict(next);
      document.getElementById('privacyStatus').textContent = next ? 'On' : 'Off';
      toast(`Strict privacy ${next ? 'enabled' : 'disabled'}`);
      break;
    }
    case 'settings':
      document.dispatchEvent(new CustomEvent('lunar:open-settings'));
      break;
    case 'about':
      toast('Lunar Browser 1.0 — Built with Tauri + Rust. Browse lighter, browse quieter.');
      break;
  }
}

function closeOtherPanels(except) {
  document.getElementById('downloadsPanel').hidden = (except.id !== 'downloadsPanel');
}

function toast(msg) {
  const t = document.getElementById('toast');
  t.textContent = msg;
  t.hidden = false;
  setTimeout(() => { t.hidden = true; }, 3000);
}
