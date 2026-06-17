// settings.js — Settings overlay.

import { settings as sApi, memory } from '../lib/tauri-bridge.js';

let current = null;

export function initSettings() {
  const overlay = document.getElementById('settingsOverlay');
  const backdrop = document.getElementById('settingsBackdrop');
  const closeBtn = document.getElementById('settingsClose');

  document.addEventListener('lunar:open-settings', () => open());
  backdrop.addEventListener('click', () => close());
  closeBtn.addEventListener('click', () => close());
}

export async function open() {
  const overlay = document.getElementById('settingsOverlay');
  current = await sApi.get();
  render();
  overlay.hidden = false;
}

export function close() {
  document.getElementById('settingsOverlay').hidden = true;
}

async function render() {
  const body = document.getElementById('settingsBody');
  body.innerHTML = `
    <section class="settings__section">
      <h3>General</h3>
      ${row('Search engine', select('default_search_engine', current.default_search_engine, [
        ['google','Google'], ['duckduckgo','DuckDuckGo'], ['brave','Brave Search'], ['bing','Bing'],
      ]), 'default_search_engine')}
      ${row('Homepage', textInput('homepage', current.homepage), 'homepage')}
      ${row('Restore last session on startup', toggle('restore_last_session', current.restore_last_session), 'restore_last_session')}
    </section>

    <section class="settings__section">
      <h3>Appearance</h3>
      ${row('Theme', select('theme', current.theme, [
        ['moonlit-dark','Moonlit Dark (default)'], ['aurora-neon','Aurora Neon'], ['minimal-light','Minimal Light'],
      ]), 'theme')}
      ${row('Accent color', colorInput('accent_color', current.accent_color), 'accent_color')}
      ${row('Vertical tab strip', toggle('vertical_tabs', current.vertical_tabs), 'vertical_tabs')}
    </section>

    <section class="settings__section">
      <h3>Privacy &amp; Security</h3>
      ${row('Block ads', toggle('block_ads', current.block_ads), 'block_ads')}
      ${row('Block trackers', toggle('block_trackers', current.block_trackers), 'block_trackers')}
      ${row('Strict privacy mode', toggle('strict_privacy', current.strict_privacy), 'strict_privacy')}
      ${row('Send Do Not Track', toggle('do_not_track', current.do_not_track), 'do_not_track')}
      ${row('HTTPS-only mode', toggle('https_only', current.https_only), 'https_only')}
    </section>

    <section class="settings__section">
      <h3>Memory</h3>
      ${row('Memory strategy', select('memory_strategy', current.memory_strategy, [
        ['aggressive','Aggressive (1GB / 20 tabs)'], ['balanced','Balanced (1.5GB / 25 tabs)'], ['light','Light (2.5GB / 40 tabs)'],
      ]), 'memory_strategy')}
      ${row('Hard RAM budget (MB)', numberInput('memory_budget_mb', current.memory_budget_mb, 256, 8192), 'memory_budget_mb', 'Process RSS will be force-trimmed when this is exceeded.')}
    </section>

    <section class="settings__section">
      <h3>Features</h3>
      ${row('Enable extensions', toggle('enable_extensions', current.enable_extensions), 'enable_extensions')}
      ${row('Enable split view', toggle('enable_split_view', current.enable_split_view), 'enable_split_view')}
      ${row('Command palette (⌘K)', toggle('command_palette_enabled', current.command_palette_enabled), 'command_palette_enabled')}
    </section>

    <section class="settings__section">
      <h3>Downloads</h3>
      ${row('Download directory', textInput('download_dir', current.download_dir), 'download_dir')}
    </section>

    <div style="display: flex; justify-content: space-between; padding-top: 16px;">
      <button class="iconbtn" id="settingsReset" style="border: 1px solid var(--border-soft); padding: 8px 16px;">Reset to defaults</button>
    </div>
  `;

  // Wire up toggles.
  body.querySelectorAll('.toggle').forEach(t => {
    t.addEventListener('click', async () => {
      const key = t.dataset.key;
      const next = t.dataset.on !== 'true';
      t.dataset.on = next ? 'true' : 'false';
      await sApi.set(key, next);
      current[key] = next;
      // If memory strategy changed, push to backend memory config too.
      if (key === 'memory_strategy') {
        const budgets = { aggressive: 1024, balanced: 1536, light: 2560 };
        await memory.setBudgetMb(budgets[next] ?? 1024, next);
      }
    });
  });

  // Wire up selects + inputs (commit on change).
  body.querySelectorAll('select, input[type="text"], input[type="number"], input[type="color"]').forEach(el => {
    el.addEventListener('change', async () => {
      const key = el.dataset.key;
      let value = el.value;
      if (el.type === 'number') value = parseInt(value);
      await sApi.set(key, value);
      current[key] = value;
    });
  });

  // Reset button.
  body.querySelector('#settingsReset').addEventListener('click', async () => {
    await sApi.reset();
    current = await sApi.get();
    render();
  });
}

function row(label, control, key, hint = '') {
  return `
    <div class="settings__row">
      <div>
        <div class="settings__row-label">${escapeHtml(label)}</div>
        ${hint ? `<div class="settings__row-hint">${escapeHtml(hint)}</div>` : ''}
      </div>
      <div>${control}</div>
    </div>
  `;
}

function toggle(key, on) {
  return `<div class="toggle" data-key="${key}" data-on="${on}" role="switch" aria-checked="${on}" tabindex="0"></div>`;
}

function select(key, value, options) {
  return `<select class="select" data-key="${key}">${options.map(([v,l]) => `<option value="${v}" ${v === value ? 'selected' : ''}>${escapeHtml(l)}</option>`).join('')}</select>`;
}

function textInput(key, value) {
  return `<input type="text" data-key="${key}" value="${escapeAttr(value)}" class="select" style="min-width: 280px;">`;
}

function numberInput(key, value, min, max) {
  return `<input type="number" data-key="${key}" value="${value}" min="${min}" max="${max}" class="select">`;
}

function colorInput(key, value) {
  return `<input type="color" data-key="${key}" value="${value}" class="select" style="height: 32px; padding: 2px;">`;
}

function escapeHtml(s) { return String(s).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'})[c]); }
function escapeAttr(s) { return escapeHtml(s); }
