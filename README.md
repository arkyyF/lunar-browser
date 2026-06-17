# 🌙 Lunar Browser

> A memory-efficient, privacy-first web browser built on Tauri + Rust.
> Target: **~1 GB RAM with 20 tabs open**, on Windows / macOS / Linux.

Lunar Browser is built on the principle that a browser should be calm, quiet, and lightweight. It uses an aggressive tab suspension strategy, a built-in ad-blocker and tracker blocker, and a dark "moonlit" theme that gets out of your way.

---

## ✨ Features

### Memory efficiency (the headline)
- **Aggressive tab suspension** — inactive tabs are released after 60 seconds.
- **Single live renderer** — only the currently visible tab holds a live WebView; everything else is a lightweight snapshot.
- **Hard RAM budget** — process RSS is monitored every 30s; if it exceeds the budget (default 1024 MB) tabs are force-discarded until under budget.
- **Pinned tab exemption** — pinned tabs stay loaded but release their renderer when not active.
- **Live RAM meter** — sidebar widget shows current RSS vs budget at all times.

### Privacy
- **Built-in ad-block** — EasyList + EasyPrivacy + uBlock filters, cached locally and auto-updated every 7 days.
- **Tracker blocking** — baked-in tracker domain list + EasyPrivacy supplements.
- **Strict privacy mode** — blocks third-party cookies/storage + fingerprinting scripts.
- **Incognito windows** — isolated cookie jars, no history persistence, wiped on close.
- **Do Not Track + HTTPS-only** — enabled by default.

### Productivity
- **Vertical tab strip** (Edge/Zen-style) with pinning, mute, discard, duplicate.
- **Workspaces** — switchable groups of tabs, each with its own color + icon.
- **Split view** — two tabs side by side, drag to resize.
- **Command palette** (⌘K) — search tabs, bookmarks, history, run commands, open URLs.
- **Omnibox** with Google search suggestions + history + bookmark matching.
- **Bookmarks + folders** with full-text search.
- **Browsing history** with search (capped at 10,000 entries, auto-trimmed).
- **Download manager** with pause / resume / cancel.
- **Extension host** (MV3 subset — content scripts only in v1).

### Design
- **Moonlit Dark theme** — deep navy/charcoal base, silver-moon accent (#C0C8D0), serif logo.
- Calm, premium, distraction-free.

---

## 🏗 Architecture

```
┌──────────────────────────────────────────────────────────┐
│                    Frontend (HTML/CSS/JS)                │
│   ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐  │
│   │ TabStrip │ │ Omnibox  │ │ Palette  │ │ Settings │  │
│   └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘  │
│        └────────────┴────────────┴────────────┘        │
│                         │ Tauri IPC                     │
└─────────────────────────┼────────────────────────────────┘
                          ▼
┌──────────────────────────────────────────────────────────┐
│                   Rust Backend (src-tauri)               │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐   │
│  │  Tab Mgr │ │ Memory   │ │ AdBlock  │ │ Privacy  │   │
│  │          │ │ Watchdog │ │  Engine  │ │  Engine  │   │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘   │
│       └────────────┴────────────┴────────────┘         │
│                         │                               │
│                ┌────────┴────────┐                      │
│                │   SQLite (WAL)  │                      │
│                │  bookmarks/     │                      │
│                │  history/dl/sess│                      │
│                └─────────────────┘                      │
└──────────────────────────────────────────────────────────┘
                          ▼
┌──────────────────────────────────────────────────────────┐
│       OS WebView (WebView2 / WKWebView / WebKitGTK)      │
└──────────────────────────────────────────────────────────┘
```

### Memory budget breakdown (20 tabs, Aggressive)
| Component              | RAM (typical) |
|------------------------|---------------|
| Rust process base      | ~25 MB        |
| SQLite + caches        | ~30 MB        |
| Ad-block engine        | ~40 MB        |
| Active tab WebView     | ~250-500 MB   |
| Suspended tabs (20 × ~5KB snapshot) | ~100 KB |
| Frontend UI            | ~30 MB        |
| **Total**              | **~400-650 MB** (well under 1 GB target) |

Compare to Chrome on 20 tabs: ~3-4 GB.

---

## 📁 Project structure

```
lunar-browser/
├── package.json                  # npm scripts (dev/build/icons)
├── README.md                     # this file
├── INSTALL.md                    # build & install instructions
├── src/                          # Frontend (vanilla JS, no bundler)
│   ├── index.html                # App shell
│   ├── main.js                   # Entry point + global shortcuts
│   ├── styles/
│   │   ├── main.css              # Base structural styles
│   │   └── moonlit-dark.css      # Theme colors (deep navy + silver-moon)
│   ├── lib/
│   │   └── tauri-bridge.js       # Tauri invoke + event wrappers
│   └── components/
│       ├── tabstrip.js           # Vertical tab strip
│       ├── omnibox.js            # URL bar + search suggestions
│       ├── palette.js            # Cmd+K command palette
│       ├── workspaces.js         # Workspace chips
│       ├── menu.js               # Hamburger menu
│       ├── downloads.js          # Downloads panel
│       ├── settings.js           # Settings overlay
│       ├── webview-host.js       # WebView lifecycle
│       └── newtab.js             # New Tab page
└── src-tauri/                    # Rust backend
    ├── Cargo.toml                # Rust deps
    ├── tauri.conf.json           # Tauri config (windows/bundle/csp)
    ├── build.rs                  # Tauri build script
    ├── capabilities/
    │   └── main.json             # Tauri 2.x permissions
    ├── icons/                    # Generated app icons (PNG/ICO/ICNS)
    └── src/
        ├── main.rs               # Binary entry
        ├── lib.rs                # App setup + plugin registration
        ├── state.rs              # Global AppState
        ├── tab.rs                # Tab model + lifecycle
        ├── memory.rs             # RAM budget + auto-discard scheduler
        ├── storage.rs            # SQLite + migrations
        ├── bookmarks.rs          # Bookmark store
        ├── history.rs            # History store
        ├── downloads.rs          # Download manager
        ├── adblock.rs            # Ad-block engine (wraps `adblock` crate)
        ├── privacy.rs            # Tracker blocking + fingerprinting protection
        ├── incognito.rs          # Incognito window lifecycle
        ├── extensions.rs         # MV3 extension host (content scripts only)
        ├── workspace.rs          # Workspaces
        ├── settings.rs           # User settings (KV store)
        └── commands.rs           # All #[tauri::command] functions
```

---

## ⌨️ Keyboard shortcuts

| Action                     | Shortcut            |
|----------------------------|---------------------|
| New tab                    | `⌘T` / `Ctrl+T`     |
| Close tab                  | `⌘W` / `Ctrl+W`     |
| Reload tab                 | `⌘R` / `Ctrl+R`     |
| Reload (bypass cache)      | `⌘⇧R` / `Ctrl+Shift+R` |
| Command palette            | `⌘K` / `Ctrl+K`     |
| Downloads panel            | `⌘J` / `Ctrl+J`     |
| New incognito window       | `⌘⇧N` / `Ctrl+Shift+N` |
| Toggle split view          | `⌘\` / `Ctrl+\`     |
| Switch to tab #1–9         | `⌘1` – `⌘9`         |
| Close all panels           | `Esc`               |

---

## 🚀 Getting started

See **[INSTALL.md](./INSTALL.md)** for full build instructions. Quick version:

```bash
# 1. Install prerequisites (Rust + Node + OS-specific WebView)
# 2. Install JS deps
cd lunar-browser
npm install

# 3. Run in dev mode
npm run dev

# 4. Build production binary
npm run build
```

Output binaries land in `src-tauri/target/release/bundle/`:
- **Windows**: `.msi` or `.exe` (NSIS installer)
- **macOS**: `.app` + `.dmg`
- **Linux**: `.deb`, `.AppImage`

---

## 🔧 Configuration

All settings are stored in `$DATA/LunarBrowser/lunar.db` (SQLite) and editable via the in-app Settings panel (Menu → Settings, or via the command palette).

Key tunables:
| Setting               | Default     | Description                              |
|-----------------------|-------------|------------------------------------------|
| `memory_strategy`     | `aggressive`| `aggressive` / `balanced` / `light`      |
| `memory_budget_mb`    | `1024`      | Hard cap on process RSS                  |
| `block_ads`           | `true`      | Enable built-in ad-block                 |
| `block_trackers`      | `true`      | Enable tracker blocking                  |
| `strict_privacy`      | `true`      | Block 3rd-party cookies + fingerprinting |
| `https_only`          | `true`      | Upgrade HTTP → HTTPS                     |
| `default_search_engine` | `google`  | `google` / `duckduckgo` / `brave` / `bing` |
| `vertical_tabs`       | `true`      | Sidebar tabs vs top tab strip            |

---

## 🛣 Roadmap

### v1 (this release) ✅
- Aggressive memory strategy hitting the 1 GB / 20-tab target
- Vertical tabs + workspaces + split view
- Built-in ad-block + tracker blocking + incognito
- Bookmarks / history / downloads
- Command palette
- Moonlit Dark theme
- Cross-platform (Windows / macOS / Linux)

### v2 (planned)
- Sync (bookmarks + tabs + history) via a self-hostable backend
- Chrome extension API surface (background service workers)
- Vertical tabs collapse / expand animation
- Reader mode
- Built-in note-taking panel
- Container tabs (per-container cookie jars)
- Custom theme support
- Tab groups (visual grouping inside a workspace)
- Per-site permissions panel (camera / mic / notifications / location)
- WebRTC IP leak protection

### v3 (long-term)
- Lunar Sync cloud service (end-to-end encrypted)
- Mobile companion (iOS / Android via Tauri Mobile)
- Built-in VPN integration (WireGuard)
- Local LLM-powered tab summarization + "ask the page" feature

---

## 🤝 Contributing

PRs welcome. Please run `cargo fmt` + `cargo clippy` before submitting.

Areas that particularly need help:
- Extension host API surface (background workers, `chrome.*` API shims)
- Per-site permissions UI
- macOS-specific titlebar polish (traffic light positioning)
- Linux WebKitGTK memory profiling

---

## 📜 License

MIT © Lunar Browser Contributors.

The bundled ad-block filter lists (EasyList, EasyPrivacy, uBlock Origin filters) are licensed under their respective licenses — see each list's header for details.

---

## 🙏 Acknowledgements

- [Tauri](https://tauri.app) — the framework that made this possible
- [adblock](https://crates.io/crates/adblock) Rust crate — Brave's filter engine
- [EasyList](https://easylist.to) — the ad-block filter list maintainers
- [uBlock Origin](https://github.com/gorhill/uBlock) — filter list authors

> *Browse lighter. Browse quieter.* 🌙
