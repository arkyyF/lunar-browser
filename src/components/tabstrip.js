// tabstrip.js — Vertical tab strip rendering + interactions.

import { tabs as tabApi, workspaces } from '../lib/tauri-bridge.js';

let activeTabId = null;
let allTabs = [];

export function getActiveTabId() { return activeTabId; }
export function setActiveTab(id) {
  activeTabId = id;
  document.querySelectorAll('.tab-card[data-id]').forEach(el => {
    el.dataset.active = (el.dataset.id === id) ? 'true' : 'false';
  });
  const t = allTabs.find(t => t.id === id);
  if (t) updateOmniboxForTab(t);
}

export async function initTabStrip() {
  const strip = document.getElementById('tabStrip');
  const newBtn = document.getElementById('newTabBtn');
  newBtn.addEventListener('click', () => document.dispatchEvent(new KeyboardEvent('keydown', { key: 't', metaKey: true, ctrlKey: navigator.userAgent.includes('Mac') })));

  // Initial load.
  allTabs = await tabApi.list();
  renderTabs();

  // Listen for tab events from main.js
  document.addEventListener('lunar:tab-added', (e) => {
    allTabs.push(e.detail);
    renderTabs();
    setActiveTab(e.detail.id);
  });
  document.addEventListener('lunar:tab-closed', (e) => {
    allTabs = allTabs.filter(t => t.id !== e.detail);
    renderTabs();
    // Activate the most recent remaining tab.
    const next = allTabs.sort((a,b) => new Date(b.last_active) - new Date(a.last_active))[0];
    if (next) setActiveTab(next.id);
  });
  document.addEventListener('lunar:switch-tab-by-index', (e) => {
    const t = allTabs[e.detail];
    if (t) activateTab(t.id);
  });

  // Click on tab activates it; click on close closes.
  strip.addEventListener('click', (e) => {
    const card = e.target.closest('.tab-card[data-id]');
    if (!card) return;
    if (e.target.closest('.tab-card__close')) {
      e.stopPropagation();
      closeTab(card.dataset.id);
    } else {
      activateTab(card.dataset.id);
    }
  });

  // Middle-click closes.
  strip.addEventListener('mousedown', (e) => {
    if (e.button !== 1) return;
    const card = e.target.closest('.tab-card[data-id]');
    if (card) {
      e.preventDefault();
      closeTab(card.dataset.id);
    }
  });

  // Right-click context menu (pin, mute, discard, duplicate).
  strip.addEventListener('contextmenu', (e) => {
    const card = e.target.closest('.tab-card[data-id]');
    if (!card) return;
    e.preventDefault();
    showContextMenu(card.dataset.id, e.clientX, e.clientY);
  });
}

function renderTabs() {
  const strip = document.getElementById('tabStrip');
  // Remove existing tab cards (keep the new-tab button).
  strip.querySelectorAll('.tab-card[data-id]').forEach(el => el.remove());

  // Sort: pinned first, then by last_active desc.
  const sorted = [...allTabs].sort((a, b) => {
    if (a.pinned !== b.pinned) return b.pinned - a.pinned;
    return new Date(b.last_active) - new Date(a.last_active);
  });

  for (const t of sorted) {
    const card = document.createElement('button');
    card.className = 'tab-card';
    card.dataset.id = t.id;
    card.dataset.active = (t.id === activeTabId) ? 'true' : 'false';
    card.dataset.discarded = t.discarded ? 'true' : 'false';
    card.dataset.pinned = t.pinned ? 'true' : 'false';
    card.title = `${t.title} — ${t.url}`;
    card.innerHTML = `
      <span class="tab-card__favicon ${t.favicon ? '' : 'tab-card__favicon--default'}">
        ${t.favicon ? `<img src="${escapeAttr(t.favicon)}" alt="" onerror="this.parentElement.classList.add('tab-card__favicon--default'); this.remove();">` : ''}
      </span>
      <span class="tab-card__title">${escapeHtml(t.title)}</span>
      <span class="tab-card__close" title="Close">✕</span>
    `;
    strip.insertBefore(card, document.getElementById('newTabBtn'));
  }
}

async function activateTab(id) {
  // If discarded, restore it first.
  const t = allTabs.find(t => t.id === id);
  if (t?.discarded) {
    await tabApi.restore(id);
    t.discarded = false;
    renderTabs();
  }
  await tabApi.activate(id);
  setActiveTab(id);
  document.dispatchEvent(new CustomEvent('lunar:tab-activated', { detail: id }));
}

async function closeTab(id) {
  await tabApi.close(id);
  document.dispatchEvent(new CustomEvent('lunar:tab-closed', { detail: id }));
}

function showContextMenu(id, x, y) {
  // Minimal context menu — could be expanded into a proper component.
  const t = allTabs.find(t => t.id === id);
  if (!t) return;
  const actions = [
    { label: t.pinned ? 'Unpin Tab' : 'Pin Tab', fn: () => tabApi.pin(id, !t.pinned).then(() => { t.pinned = !t.pinned; renderTabs(); }) },
    { label: t.muted ? 'Unmute Tab' : 'Mute Tab', fn: () => tabApi.mute(id, !t.muted).then(() => { t.muted = !t.muted; }) },
    { label: 'Discard Tab', fn: () => tabApi.discard(id).then(() => { t.discarded = true; renderTabs(); }) },
    { label: 'Duplicate Tab', fn: () => tabApi.new(t.url, t.workspace_id, false) },
    { label: 'Close Tab', fn: () => closeTab(id) },
  ];
  document.dispatchEvent(new CustomEvent('lunar:context-menu', { detail: { x, y, actions } }));
}

// Update omnibox when active tab changes — called by main setActiveTab.
function updateOmniboxForTab(tab) {
  document.dispatchEvent(new CustomEvent('lunar:omnibox-update', { detail: tab }));
}

// ───── HTML escape helpers ─────
function escapeHtml(s) {
  return String(s).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'})[c]);
}
function escapeAttr(s) {
  return escapeHtml(s);
}
