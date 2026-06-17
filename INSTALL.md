# Installation Guide — Lunar Browser

This guide walks you through building Lunar Browser from source on Windows, macOS, and Linux.

---

## 1. Prerequisites

### All platforms
- **Rust** (stable, 1.75+): https://rustup.rs
- **Node.js** 18+ and npm: https://nodejs.org

### Windows
- **Microsoft C++ Build Tools** (Visual Studio 2022 Installer → "Desktop development with C++")
- **WebView2 runtime** (pre-installed on Windows 11; on Windows 10 download from https://developer.microsoft.com/microsoft-edge/webview2/)

### macOS
- **Xcode Command Line Tools**: `xcode-select --install`
- No WebView runtime needed — uses system WKWebView.

### Linux (Debian/Ubuntu)
```bash
sudo apt update
sudo apt install -y \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev
```

### Linux (Fedora)
```bash
sudo dnf install -y \
  webkit2gtk4.1-devel \
  openssl-devel \
  curl \
  wget \
  file \
  libappindicator-gtk3-devel \
  librsvg2-devel \
  gcc
```

### Linux (Arch)
```bash
sudo pacman -S --needed \
  webkit2gtk-4.1 \
  base-devel \
  curl \
  wget \
  file \
  openssl \
  appmenu-gtk-module \
  libappindicator-gtk3 \
  librsvg
```

---

## 2. Clone & install

```bash
git clone <your-fork-url> lunar-browser
cd lunar-browser
npm install
```

This installs `@tauri-apps/cli` and the JS-side Tauri plugin wrappers.

---

## 3. Run in development

```bash
npm run dev
```

This launches the Tauri dev server. The app window will appear with hot-reload on Rust + frontend changes.

> ⚠️ First run will compile ~250 Rust dependencies — expect 5-10 minutes. Subsequent builds are incremental and fast (under 30s).

---

## 4. Build production binaries

```bash
npm run build
```

Output binaries land in `src-tauri/target/release/bundle/`:

| Platform | Output                                          |
|----------|-------------------------------------------------|
| Windows  | `bundle/msi/*.msi` and `bundle/nsis/*.exe`     |
| macOS    | `bundle/macos/*.app` and `bundle/dmg/*.dmg`    |
| Linux    | `bundle/deb/*.deb`, `bundle/appimage/*.AppImage` |

Install the binary on your platform as you would any other app.

---

## 5. Regenerate icons (optional)

If you change `src-tauri/icons/icon.png`, regenerate all sizes:

```bash
npm run icons
```

This requires ImageMagick or the `tauri icon` CLI (bundled with `@tauri-apps/cli`).

---

## 6. Verify memory target

Once running, open 20 tabs (any sites will do). Check the RAM meter at the bottom-left of the sidebar — it should stay below 1024 MB.

If it doesn't:
1. Open Settings → Memory.
2. Confirm strategy is **Aggressive**.
3. Confirm **Hard RAM budget** is 1024 MB.
4. Watch the meter for 60+ seconds — inactive tabs will be discarded and RAM should drop.

For extreme cases, lower the budget to 768 MB. The browser will simply discard more aggressively.

---

## 7. Troubleshooting

### "WebView2 runtime not found" (Windows)
Install the Evergreen Bootstrapper from https://developer.microsoft.com/microsoft-edge/webview2/.

### Build fails with `linker 'cc' not found` (Linux)
Install `build-essential` (Ubuntu) or `base-devel` (Arch) or `gcc` (Fedora).

### macOS: `ld: framework not found WebKit`
Make sure Xcode Command Line Tools are installed: `xcode-select --install`.

### Ad-block lists don't download
The first launch downloads EasyList + EasyPrivacy + uBlock filters (~5 MB total). If you're behind a proxy, set `HTTPS_PROXY` env var before launching. Lists are cached at `$DATA/LunarBrowser/adblock/` and re-downloaded every 7 days.

### App is dark even though system is light
Lunar defaults to Moonlit Dark. You can switch themes in Settings → Appearance, but v1 only ships the dark theme.

### Tabs keep getting discarded too aggressively
Switch memory strategy to **Balanced** (5-min idle threshold) or **Light** (30-min) in Settings → Memory.

---

## 8. Uninstall

Just delete the app bundle. Lunar Browser stores all user data in:
- **Windows**: `%APPDATA%\LunarBrowser\`
- **macOS**: `~/Library/Application Support/LunarBrowser/`
- **Linux**: `~/.local/share/LunarBrowser/`

Delete that directory to wipe bookmarks, history, settings, and cached ad-block lists.

---

*Questions? Open an issue on the project repo.*
