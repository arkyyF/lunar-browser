// downloads.js — Downloads panel.

import { downloads as dlApi } from '../lib/tauri-bridge.js';

export function initDownloads() {
  const panel = document.getElementById('downloadsPanel');
  const closeBtn = document.getElementById('downloadsClose');
  const btn = document.getElementById('downloadsBtn');

  btn.addEventListener('click', async (e) => {
    e.stopPropagation();
    panel.hidden = !panel.hidden;
    if (!panel.hidden) await refresh();
  });

  closeBtn.addEventListener('click', () => panel.hidden = true);

  document.addEventListener('click', (e) => {
    if (!e.target.closest('#downloadsPanel') && !e.target.closest('#downloadsBtn')) {
      panel.hidden = true;
    }
  });

  // Refresh on download events.
  document.addEventListener('lunar:download-finished', () => refresh());
}

async function refresh() {
  const list = document.getElementById('downloadsList');
  const items = await dlApi.list();
  if (items.length === 0) {
    list.innerHTML = '<li style="padding: 24px; text-align: center; color: var(--text-tertiary);">No downloads yet</li>';
    return;
  }
  list.innerHTML = items.map(d => {
    const pct = d.total_bytes > 0 ? Math.min(100, (d.downloaded_bytes / d.total_bytes) * 100) : 0;
    return `
      <li class="download-item" data-id="${d.id}">
        <div class="download-item__icon">📥</div>
        <div class="download-item__meta">
          <div class="download-item__name">${escapeHtml(d.filename)}</div>
          <div class="download-item__progress">
            <div class="download-item__progress-bar" data-status="${d.status}" style="width: ${pct}%"></div>
          </div>
          <div class="download-item__status">
            ${formatBytes(d.downloaded_bytes)} / ${d.total_bytes > 0 ? formatBytes(d.total_bytes) : '?'} · ${d.status}
          </div>
        </div>
        <div class="download-item__actions">
          ${actionButtons(d)}
        </div>
      </li>
    `;
  }).join('');

  // Wire action buttons.
  list.querySelectorAll('.download-item').forEach(item => {
    const id = item.dataset.id;
    item.querySelectorAll('button[data-act]').forEach(btn => {
      btn.addEventListener('click', () => handleAction(btn.dataset.act, id));
    });
  });
}

function actionButtons(d) {
  const buttons = [];
  if (d.status === 'InProgress') buttons.push(`<button data-act="pause" class="iconbtn" title="Pause">⏸</button>`);
  if (d.status === 'Paused')     buttons.push(`<button data-act="resume" class="iconbtn" title="Resume">▶</button>`);
  if (d.status === 'InProgress' || d.status === 'Paused') {
    buttons.push(`<button data-act="cancel" class="iconbtn" title="Cancel">✕</button>`);
  }
  return buttons.join('');
}

async function handleAction(act, id) {
  try {
    if (act === 'pause')  await dlApi.pause(id);
    if (act === 'resume') await dlApi.resume(id);
    if (act === 'cancel') await dlApi.cancel(id);
    await refresh();
  } catch (e) {
    console.error('[Lunar] download action failed:', e);
  }
}

function formatBytes(b) {
  if (b === 0) return '0 B';
  const k = 1024;
  const sizes = ['B','KB','MB','GB'];
  const i = Math.floor(Math.log(b) / Math.log(k));
  return parseFloat((b / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}

function escapeHtml(s) {
  return String(s).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'})[c]);
}
