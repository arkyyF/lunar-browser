// palette.js — Cmd+K command palette.
// Sources: open tabs, bookmarks, history, command actions, URL bar.

import { tabs as tabApi, bookmarks, history, settings, incognito, memory } from '../lib/tauri-bridge.js';

let open = false;
let results = [];
let selectedIdx = 0;
let debounceTimer = null;

export function initPalette() {
  const palette = document.getElementById('cmdPalette');
  const input = document.getElementById('paletteInput');
  const list = document.getElementById('paletteResults');
  const backdrop = document.getElementById('paletteBackdrop');

  document.getElementById('cmdPaletteBtn').addEventListener('click', openPalette);
  backdrop.addEventListener('click', closePalette);

  input.addEventListener('input', () => {
    clearTimeout(debounceTimer);
    const q = input.value.trim();
    if (!q) { results = defaultCommands(); render(); return; }
    debounceTimer = setTimeout(() => search(q), 100);
  });

  input.addEventListener('keydown', (e) => {
    if (e.key === 'ArrowDown') { e.preventDefault(); move(1); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); move(-1); }
    else if (e.key === 'Enter') { e.preventDefault(); execute(results[selectedIdx]); }
    else if (e.key === 'Escape') closePalette();
  });

  list.addEventListener('click', (e) => {
    const li = e.target.closest('li');
    if (!li) return;
    selectedIdx = parseInt(li.dataset.idx);
    execute(results[selectedIdx]);
  });
}

export function openPalette() {
  if (open) return;
  open = true;
  const palette = document.getElementById('cmdPalette');
  palette.hidden = false;
  const input = document.getElementById('paletteInput');
  input.value = '';
  results = defaultCommands();
  selectedIdx = 0;
  render();
  setTimeout(() => input.focus(), 50);
}

export function closePalette() {
  open = false;
  document.getElementById('cmdPalette').hidden = true;
}

async function search(query) {
  const [allTabs, bms, hist] = await Promise.all([
    tabApi.list(),
    bookmarks.search(query),
    history.search(query, 8),
  ]);

  // URL bar entry — top priority.
  results = [];
  if (looksLikeUrl(query)) {
    results.push({ type: 'url', title: `Open "${query}"`, subtitle: 'Navigate to URL', icon: '🌐', action: async () => {
      const id = await getOrCreateActiveTab();
      await tabApi.navigate(id, query);
    }});
  } else {
    results.push({ type: 'search', title: `Search Google for "${query}"`, subtitle: 'Web search', icon: '🔍', action: async () => {
      const id = await getOrCreateActiveTab();
      await tabApi.navigate(id, 'https://www.google.com/search?q=' + encodeURIComponent(query));
    }});
  }

  // Matching tabs.
  for (const t of allTabs) {
    if ((t.title + t.url).toLowerCase().includes(query.toLowerCase())) {
      results.push({ type: 'tab', title: t.title, subtitle: t.url, icon: '📑', action: async () => {
        await tabApi.activate(t.id);
      }});
    }
  }

  // Bookmarks.
  for (const b of bms) {
    results.push({ type: 'bookmark', title: b.title, subtitle: b.url, icon: '★', action: async () => {
      const id = await getOrCreateActiveTab();
      await tabApi.navigate(id, b.url);
    }});
  }

  // History.
  for (const h of hist) {
    results.push({ type: 'history', title: h.title || h.url, subtitle: h.url, icon: '⟳', action: async () => {
      const id = await getOrCreateActiveTab();
      await tabApi.navigate(id, h.url);
    }});
  }

  // Matching commands.
  for (const c of defaultCommands()) {
    if (c.title.toLowerCase().includes(query.toLowerCase())) {
      results.push(c);
    }
  }

  selectedIdx = 0;
  render();
}

function defaultCommands() {
  return [
    { type: 'cmd', title: 'New Tab', subtitle: '⌘T', icon: '➕', action: () => document.dispatchEvent(new KeyboardEvent('keydown', { key: 't', metaKey: true })) },
    { type: 'cmd', title: 'New Incognito Window', subtitle: '⌘⇧N', icon: '🕵', action: () => incognito.openWindow() },
    { type: 'cmd', title: 'Toggle Split View', subtitle: '⌘\\', icon: '⬜⬜', action: () => document.dispatchEvent(new CustomEvent('lunar:toggle-split')) },
    { type: 'cmd', title: 'Reload Tab', subtitle: '⌘R', icon: '↻', action: () => document.dispatchEvent(new KeyboardEvent('keydown', { key: 'r', metaKey: true })) },
    { type: 'cmd', title: 'Force Garbage Collect', subtitle: 'Free memory', icon: '🧹', action: () => memory.forceGc() },
    { type: 'cmd', title: 'Open Settings', subtitle: 'Preferences', icon: '⚙', action: () => document.dispatchEvent(new CustomEvent('lunar:open-settings')) },
    { type: 'cmd', title: 'Clear Browsing History', subtitle: 'Wipe all history', icon: '🗑', action: () => history.clear() },
    { type: 'cmd', title: 'Close All Incognito Windows', subtitle: '', icon: '🚪', action: () => incognito.closeAll() },
  ];
}

function render() {
  const list = document.getElementById('paletteResults');
  if (results.length === 0) {
    list.innerHTML = '<li style="color: var(--text-tertiary); justify-content: center;">No results</li>';
    return;
  }
  list.innerHTML = results.map((r, i) => `
    <li data-idx="${i}" ${i === selectedIdx ? 'aria-selected="true"' : ''}>
      <span class="result-icon">${r.icon || '›'}</span>
      <div>
        <div>${escapeHtml(r.title)}</div>
        ${r.subtitle ? `<div style="font-size:11px;color:var(--text-faint);">${escapeHtml(r.subtitle)}</div>` : ''}
      </div>
      <span class="result-meta">${r.type}</span>
    </li>
  `).join('');
}

function move(delta) {
  if (results.length === 0) return;
  selectedIdx = (selectedIdx + delta + results.length) % results.length;
  render();
  document.querySelector(`.palette__results li[aria-selected="true"]`)?.scrollIntoView({ block: 'nearest' });
}

async function execute(result) {
  if (!result) return;
  closePalette();
  try { await result.action(); }
  catch (e) { console.error('[Lunar] palette action failed:', e); }
}

async function getOrCreateActiveTab() {
  const all = await tabApi.list();
  const active = all.find(t => !t.discarded) || all[0];
  if (active) {
    await tabApi.activate(active.id);
    return active.id;
  }
  const t = await tabApi.new('lunar://newtab');
  return t.id;
}

function looksLikeUrl(s) {
  if (/^https?:\/\//i.test(s)) return true;
  if (/^[\w-]+(\.[\w-]+)+(\/.*)?$/.test(s) && !s.includes(' ')) return true;
  if (/^localhost(:\d+)?/.test(s)) return true;
  return false;
}

function escapeHtml(s) {
  return String(s).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'})[c]);
}
