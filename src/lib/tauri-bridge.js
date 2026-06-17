// tauri-bridge.js — Thin wrapper around @tauri-apps/api invoke + event listen.
// All frontend code calls these helpers instead of importing @tauri-apps/api
// directly, so we can mock in dev if needed.

const { invoke } = window.__TAURI__?.core ?? {};
const { listen } = window.__TAURI__?.event ?? {};

if (!invoke) {
  console.warn('[Lunar] Tauri API not available — running in browser-dev mode. Backend calls will be no-ops.');
}

export async function call(cmd, args = {}) {
  if (!invoke) return mockResponse(cmd, args);
  try {
    return await invoke(cmd, args);
  } catch (e) {
    console.error(`[Lunar] command "${cmd}" failed:`, e);
    throw e;
  }
}

export async function on(event, handler) {
  if (!listen) return () => {};
  return listen(event, (e) => handler(e.payload, e));
}

// ───── Tab API ─────
export const tabs = {
  new: (url, workspaceId = null, incognito = false) => call('tab_new', { url, workspaceId, incognito }),
  close: (id) => call('tab_close', { id }),
  activate: (id) => call('tab_activate', { id }),
  reload: (id, bypassCache = false) => call('tab_reload', { id, bypassCache }),
  navigate: (id, url) => call('tab_navigate', { id, url }),
  goBack: (id) => call('tab_go_back', { id }),
  goForward: (id) => call('tab_go_forward', { id }),
  pin: (id, pinned) => call('tab_pin', { id, pinned }),
  mute: (id, muted) => call('tab_mute', { id, muted }),
  discard: (id) => call('tab_discard', { id }),
  restore: (id) => call('tab_restore', { id }),
  list: () => call('tab_list'),
  getState: (id) => call('tab_get_state', { id }),
  split: (id, withId) => call('tab_split', { id, with: withId }),
};

// ───── Workspace API ─────
export const workspaces = {
  create: (name, icon = 'moon', color = '#C0C8D0') => call('workspace_create', { name, icon, color }),
  switch: (id) => call('workspace_switch', { id }),
  list: () => call('workspace_list'),
  delete: (id) => call('workspace_delete', { id }),
};

// ───── Bookmark API ─────
export const bookmarks = {
  add: (url, title, favicon = null, folderId = null) => call('bookmark_add', { url, title, favicon, folderId }),
  remove: (id) => call('bookmark_remove', { id }),
  list: (folderId = null) => call('bookmark_list', { folderId }),
  search: (query) => call('bookmark_search', { query }),
  createFolder: (name, parentId = null) => call('bookmark_folder_create', { name, parentId }),
};

// ───── History API ─────
export const history = {
  add: (url, title = null, favicon = null, incognito = false) => call('history_add', { url, title, favicon, incognito }),
  list: (limit = 100, offset = 0) => call('history_list', { limit, offset }),
  search: (query, limit = 50) => call('history_search', { query, limit }),
  clear: () => call('history_clear'),
  remove: (id) => call('history_remove', { id }),
};

// ───── Downloads API ─────
export const downloads = {
  start: (url, saveDir = null, tabId = null) => call('downloads_start', { url, saveDir, tabId }),
  pause: (id) => call('downloads_pause', { id }),
  resume: (id) => call('downloads_resume', { id }),
  cancel: (id) => call('downloads_cancel', { id }),
  list: () => call('downloads_list'),
};

// ───── Ad-block / Privacy ─────
export const adblock = {
  status: () => call('adblock_status'),
  reload: () => call('adblock_reload_lists'),
  toggle: (enabled) => call('adblock_toggle', { enabled }),
};

export const privacy = {
  block: (requestUrl, sourceUrl, resourceType) => call('privacy_block_request', { requestUrl, sourceUrl, resourceType }),
  setStrict: (strict) => call('privacy_set_strict', { strict }),
};

// ───── Incognito ─────
export const incognito = {
  openWindow: (url = null) => call('incognito_open_window', { url }),
  closeAll: () => call('incognito_close_all'),
};

// ───── Extensions ─────
export const extensions = {
  install: (manifestJson) => call('extensions_install', { manifestJson }),
  list: () => call('extensions_list'),
  toggle: (id, enabled) => call('extensions_toggle', { id, enabled }),
  remove: (id) => call('extensions_remove', { id }),
};

// ───── Memory ─────
export const memory = {
  stats: () => call('memory_stats'),
  forceGc: () => call('memory_force_gc'),
  setBudgetMb: (mb, strategy = null) => call('memory_set_budget_mb', { mb, strategy }),
};

// ───── Settings ─────
export const settings = {
  get: () => call('settings_get'),
  set: (key, value) => call('settings_set', { key, value }),
  reset: () => call('settings_reset'),
};

// ───── Search ─────
export const search = {
  suggest: (query) => call('search_suggest', { query }),
};

// ───── Mock responses for browser-dev mode ─────
function mockResponse(cmd, args) {
  switch (cmd) {
    case 'tab_list': return [];
    case 'tab_new': return { id: 'mock-tab-' + Date.now(), url: args.url, title: 'New Tab', pinned: false, muted: false, loading: false, can_go_back: false, can_go_forward: false, incognito: false, workspace_id: null, created_at: new Date().toISOString(), last_active: new Date().toISOString(), discarded: false, history_stack: { back: [], forward: [], current: args.url, scroll_y: 0 }, split_with: null, estimated_bytes: 0, favicon: null };
    case 'workspace_list': return [{ id: 'default', name: 'Default', icon: 'moon', color: '#C0C8D0', position: 0, created_at: new Date().toISOString() }];
    case 'memory_stats': return { rss_mb: 0, virtual_mb: 0, budget_mb: 1024, tabs_total: 0, tabs_discarded: 0, tabs_pinned: 0, over_budget: false, strategy: 'Aggressive' };
    case 'settings_get': return { default_search_engine: 'google', homepage: 'lunar://newtab', restore_last_session: true, block_ads: true, block_trackers: true, strict_privacy: true, memory_strategy: 'aggressive', memory_budget_mb: 1024, download_dir: '~/Downloads', theme: 'moonlit-dark', accent_color: '#C0C8D0', enable_extensions: true, enable_split_view: true, vertical_tabs: true, command_palette_enabled: true, do_not_track: true, https_only: true, extras: {} };
    case 'adblock_status': return { enabled: true, lists: [], last_update: null };
    case 'downloads_list': return [];
    case 'history_list': return [];
    case 'history_search': return [];
    case 'bookmark_list': return [];
    case 'bookmark_search': return [];
    case 'search_suggest': return [];
    default: return null;
  }
}
