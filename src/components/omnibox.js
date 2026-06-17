// omnibox.js — URL bar + search suggestions.

import { tabs as tabApi, search, history, bookmarks } from '../lib/tauri-bridge.js';

let activeTabId = null;
let suggestions = [];
let selectedIdx = -1;
let debounceTimer = null;

export function initOmnibox() {
  const input = document.getElementById('omniboxInput');
  const list = document.getElementById('omniboxSuggestions');
  const reloadBtn = document.getElementById('omniboxReload');

  // Listen for tab activations to update the URL bar.
  document.addEventListener('lunar:tab-activated', async (e) => {
    activeTabId = e.detail;
    const t = await tabApi.getState(activeTabId);
    if (t) {
      input.value = t.url === 'lunar://newtab' ? '' : t.url;
      updateLock(t.url);
    }
  });
  document.addEventListener('lunar:omnibox-update', (e) => {
    input.value = e.detail.url === 'lunar://newtab' ? '' : e.detail.url;
    updateLock(e.detail.url);
  });

  input.addEventListener('focus', () => input.select());

  input.addEventListener('input', () => {
    clearTimeout(debounceTimer);
    const q = input.value.trim();
    if (!q) { hideSuggestions(); return; }
    debounceTimer = setTimeout(() => fetchSuggestions(q), 120);
  });

  input.addEventListener('keydown', (e) => {
    if (e.key === 'ArrowDown') { e.preventDefault(); moveSelection(1); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); moveSelection(-1); }
    else if (e.key === 'Enter') { e.preventDefault(); commit(input.value); }
    else if (e.key === 'Escape') { input.blur(); hideSuggestions(); }
  });

  reloadBtn.addEventListener('click', () => {
    if (activeTabId) tabApi.reload(activeTabId, false);
  });

  list.addEventListener('click', (e) => {
    const li = e.target.closest('li');
    if (!li) return;
    const idx = parseInt(li.dataset.idx);
    if (suggestions[idx]) commit(suggestions[idx].text);
  });

  // Click outside closes suggestions.
  document.addEventListener('click', (e) => {
    if (!e.target.closest('.omnibox')) hideSuggestions();
  });
}

async function fetchSuggestions(query) {
  // 1. Backend search suggest (Google).
  const sugg = await search.suggest(query).catch(() => []);
  // 2. Local history matches.
  const hist = await history.search(query, 4).catch(() => []);
  // 3. Bookmark matches.
  const bms = await bookmarks.search(query, 3).catch(() => []);

  suggestions = [
    ...sugg.map(s => ({ text: s.text, kind: 'suggestion' })),
    ...hist.map(h => ({ text: h.url, kind: 'history' })),
    ...bms.map(b => ({ text: b.url, kind: 'bookmark' })),
  ];
  selectedIdx = -1;
  renderSuggestions();
}

function renderSuggestions() {
  const list = document.getElementById('omniboxSuggestions');
  if (suggestions.length === 0) { hideSuggestions(); return; }
  list.innerHTML = suggestions.map((s, i) => `
    <li data-idx="${i}" ${i === selectedIdx ? 'aria-selected="true"' : ''}>
      <span class="suggestion-icon">${iconForKind(s.kind)}</span>
      <span class="suggestion-text">${escapeHtml(s.text)}</span>
      <span class="suggestion-kind">${s.kind}</span>
    </li>
  `).join('');
  list.hidden = false;
  document.getElementById('omnibox').setAttribute('aria-expanded', 'true');
}

function hideSuggestions() {
  document.getElementById('omniboxSuggestions').hidden = true;
  document.getElementById('omnibox').setAttribute('aria-expanded', 'false');
}

function moveSelection(delta) {
  if (suggestions.length === 0) return;
  selectedIdx = (selectedIdx + delta + suggestions.length) % suggestions.length;
  renderSuggestions();
  const sel = document.querySelector(`.omnibox__suggestions li[aria-selected="true"]`);
  sel?.scrollIntoView({ block: 'nearest' });
}

async function commit(value) {
  hideSuggestions();
  if (!activeTabId) {
    // No active tab → create one.
    const t = await tabApi.new(resolveUrl(value), null, false);
    document.dispatchEvent(new CustomEvent('lunar:tab-added', { detail: t }));
    activeTabId = t.id;
    return;
  }
  const url = resolveUrl(value);
  await tabApi.navigate(activeTabId, url);
  document.dispatchEvent(new CustomEvent('lunar:tab-navigated', { detail: { id: activeTabId, url } }));
}

function resolveUrl(input) {
  const v = input.trim();
  if (!v) return 'lunar://newtab';
  // lunar:// internal
  if (v.startsWith('lunar://')) return v;
  // URL with scheme
  if (/^https?:\/\//i.test(v)) return v;
  // Looks like a domain (has a dot, no spaces)
  if (/^[\w-]+(\.[\w-]+)+(\/.*)?$/.test(v)) return 'https://' + v;
  // Localhost
  if (/^localhost(:\d+)?(\/.*)?$/.test(v)) return 'http://' + v;
  // Otherwise treat as Google search
  return 'https://www.google.com/search?q=' + encodeURIComponent(v);
}

function updateLock(url) {
  const lock = document.getElementById('omniboxLock');
  if (!lock) return;
  const secure = url.startsWith('https://') || url.startsWith('lunar://') || url.startsWith('about:');
  lock.style.color = secure ? 'var(--success)' : 'var(--warning)';
  lock.title = secure ? 'Connection secure' : 'Connection not secure';
}

function iconForKind(kind) {
  switch (kind) {
    case 'history':  return '⟳';
    case 'bookmark': return '★';
    default:         return '🔍';
  }
}

function escapeHtml(s) {
  return String(s).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'})[c]);
}
