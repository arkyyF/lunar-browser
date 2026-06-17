// workspaces.js — Sidebar workspace chips.

import { workspaces as wsApi } from '../lib/tauri-bridge.js';

let activeWorkspaceId = null;
let workspaces = [];

export function getActiveWorkspace() {
  return workspaces.find(w => w.id === activeWorkspaceId) || workspaces[0];
}

export async function initWorkspaces() {
  const container = document.getElementById('workspaces');
  workspaces = await wsApi.list();
  if (workspaces.length > 0) activeWorkspaceId = workspaces[0].id;
  render();

  container.addEventListener('click', (e) => {
    const chip = e.target.closest('.workspace-chip[data-id]');
    if (!chip) return;
    activeWorkspaceId = chip.dataset.id;
    document.dispatchEvent(new CustomEvent('lunar:workspace-switched', { detail: activeWorkspaceId }));
    render();
  });

  // Add-workspace chip.
  const addChip = document.createElement('button');
  addChip.className = 'workspace-chip workspace-chip__add';
  addChip.innerHTML = '+ New';
  addChip.addEventListener('click', async () => {
    const name = prompt('Workspace name:');
    if (!name) return;
    const colors = ['#C0C8D0', '#7AA2D9', '#7FD1B9', '#F2C94C', '#E07A6B', '#B89AE8'];
    const color = colors[Math.floor(Math.random() * colors.length)];
    const ws = await wsApi.create(name, 'moon', color);
    workspaces.push(ws);
    activeWorkspaceId = ws.id;
    render();
  });
  container.appendChild(addChip);
}

function render() {
  const container = document.getElementById('workspaces');
  // Remove all non-add chips.
  container.querySelectorAll('.workspace-chip[data-id]').forEach(el => el.remove());

  for (const ws of workspaces) {
    const chip = document.createElement('button');
    chip.className = 'workspace-chip';
    chip.dataset.id = ws.id;
    chip.dataset.active = (ws.id === activeWorkspaceId) ? 'true' : 'false';
    chip.innerHTML = `
      <span class="workspace-chip__dot" style="background: ${ws.color}; box-shadow: 0 0 6px ${ws.color}33;"></span>
      <span>${escapeHtml(ws.name)}</span>
    `;
    container.insertBefore(chip, container.lastElementChild);
  }
}

function escapeHtml(s) {
  return String(s).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'})[c]);
}
