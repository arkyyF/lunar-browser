// newtab.js — New Tab page interactions.

const DEFAULT_SHORTCUTS = [
  { title: 'Google',   url: 'https://google.com',   icon: 'https://www.google.com/favicon.ico' },
  { title: 'YouTube',  url: 'https://youtube.com',  icon: 'https://www.youtube.com/favicon.ico' },
  { title: 'Wikipedia',url: 'https://wikipedia.org',icon: 'https://en.wikipedia.org/favicon.ico' },
  { title: 'GitHub',   url: 'https://github.com',   icon: 'https://github.com/favicon.ico' },
  { title: 'Reddit',   url: 'https://reddit.com',   icon: 'https://www.reddit.com/favicon.ico' },
];

export function initNewTab() {
  refreshNewTab(document.getElementById('webviewHostPrimary'));
}

export function refreshNewTab(host) {
  if (!host) return;
  const searchInput = host.querySelector('#newtabSearch');
  if (searchInput) {
    searchInput.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' && searchInput.value.trim()) {
        navigate(searchInput.value.trim());
      }
    });
  }
  const shortcuts = host.querySelector('#newtabShortcuts');
  if (shortcuts) {
    shortcuts.innerHTML = DEFAULT_SHORTCUTS.map(s => `
      <a class="shortcut" data-url="${escapeAttr(s.url)}" title="${escapeAttr(s.title)} — ${escapeAttr(s.url)}">
        <div class="shortcut__icon"><img src="${escapeAttr(s.icon)}" alt="" onerror="this.style.display='none'"></div>
        <div class="shortcut__label">${escapeHtml(s.title)}</div>
      </a>
    `).join('');
    shortcuts.querySelectorAll('.shortcut').forEach(el => {
      el.addEventListener('click', () => navigate(el.dataset.url));
    });
  }
}

function navigate(url) {
  // Find or create an active tab and navigate.
  document.dispatchEvent(new CustomEvent('lunar:omnibox-navigate', { detail: url }));
  const omniboxInput = document.getElementById('omniboxInput');
  if (omniboxInput) {
    omniboxInput.value = url;
    omniboxInput.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
  }
}

function escapeHtml(s) { return String(s).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'})[c]); }
function escapeAttr(s) { return escapeHtml(s); }
