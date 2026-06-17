# How to get your Lunar Browser `.exe`

You have **two paths**. Pick whichever is easier for you.

---

## Path A: GitHub Actions (recommended — zero local install)

This is the easiest. GitHub's servers build the `.exe` for you on a real Windows machine. You just download it.

### Steps

1. **Create a free GitHub account** if you don't have one: https://github.com/signup

2. **Create a new repo** at https://github.com/new
   - Name it `lunar-browser`
   - Set it to **Public** (so you can use free CI minutes)
   - Don't initialize with README

3. **Upload the source.** From inside the unzipped `lunar-browser/` folder:
   ```bash
   git init
   git add .
   git commit -m "Initial commit: Lunar Browser"
   git branch -M main
   git remote add origin https://github.com/YOUR_USERNAME/lunar-browser.git
   git push -u origin main
   ```
   
   *(No git? Download GitHub Desktop: https://desktop.github.com)*

4. **Wait for the build.** Go to your repo on GitHub → **Actions** tab → click the latest "Build Windows" run. It'll take ~10–15 min. When it's green:

5. **Download the `.exe`.** Scroll to the bottom of the run → **Artifacts** → download `Lunar-Browser-Windows-NSIS`. Unzip → run `Lunar-Browser_1.0.0_x64-setup.exe`.

That's it. You have your installer.

### Want a proper release page with downloads?

Once the build is green, tag a release:
```bash
git tag v1.0.0
git push origin v1.0.0
```

This triggers the `Release` workflow, which builds for Windows + macOS + Linux and posts a proper GitHub Release page (like https://github.com/tauri-apps/tauri/releases) with all the installers attached. Anyone can download from there.

---

## Path B: Build locally with one click (Windows)

If you want to build it on your own machine:

1. **Unzip** `lunar-browser-source.zip` somewhere.

2. **Double-click `build.bat`**. The script will:
   - Install Rust for you if missing
   - Install npm deps
   - Compile everything
   - Print the path to your `.exe` at the end

3. You'll need:
   - **Node.js 18+**: https://nodejs.org
   - **Visual Studio 2022 Build Tools** (Desktop C++ workload): https://visualstudio.microsoft.com/visual-cpp-build-tools/
   
   The script will warn you if either is missing and tell you what to install.

First build takes ~10–15 minutes (compiles ~250 Rust crates). Subsequent builds are incremental (~30s).

### Output files

After a successful build:
```
src-tauri\target\release\bundle\nsis\Lunar-Browser_1.0.0_x64-setup.exe   ← installer
src-tauri\target\release\bundle\msi\Lunar-Browser_1.0.0_x64_en-US.msi    ← MSI installer
src-tauri\target\release\lunar-browser.exe                              ← portable
```

---

## Which path should I pick?

| Situation | Recommended path |
|-----------|------------------|
| "I just want the .exe, minimal hassle" | **Path A** (GitHub Actions) |
| "I want to actually develop / modify the code" | **Path B** (local build) |
| "I'm on a Mac/Linux but want a Windows .exe" | **Path A** (GitHub Actions) |
| "I have no internet but have a Windows PC with dev tools" | **Path B** (local build) |
| "I want releases that other people can download" | **Path A** + push a `v1.0.0` tag |

---

## Troubleshooting

### CI build fails with "MSVC linker not found"
The `windows-latest` GitHub runner has this pre-installed. If you see this error, your `tauri-action` is out of date — bump it to `@v0` latest.

### Local build: "link.exe not found"
Install VS 2022 Build Tools with the "Desktop development with C++" workload. Reboot. Re-run `build.bat`.

### Local build: "webkit2gtk not found" (you're on Linux, not Windows)
On Linux you need:
```bash
sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev
```
Then `npm run build` will produce a `.deb` + `.AppImage` instead of an `.exe`.

### Build succeeds but the app crashes on launch
Open a Command Prompt, run `lunar-browser.exe` directly — it'll print errors to the console. Most common cause: missing WebView2 runtime (Windows 11 has it; older Windows 10 needs https://developer.microsoft.com/microsoft-edge/webview2/).

### The app opens but is white / blank
Your GPU driver may be incompatible with WebView2's hardware acceleration. Run with the flag: `lunar-browser.exe --disable-gpu`.

---

## Need help?

If you hit a snag, paste the error log into the chat and I'll help you debug.
