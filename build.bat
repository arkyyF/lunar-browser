@echo off
REM ════════════════════════════════════════════════════════════════
REM  Lunar Browser — one-click Windows build script
REM  Produces:
REM    src-tauri\target\release\bundle\nsis\Lunar-Browser_1.0.0_x64-setup.exe
REM    src-tauri\target\release\bundle\msi\Lunar-Browser_1.0.0_x64_en-US.msi
REM    src-tauri\target\release\lunar-browser.exe   (portable)
REM ════════════════════════════════════════════════════════════════
setlocal EnableDelayedExpansion
cd /d "%~dp0"

echo.
echo  === Lunar Browser build ===
echo.

REM ── Step 1: check Rust ──
where cargo >nul 2>nul
if errorlevel 1 (
    echo  [!] Rust not found. Installing via rustup…
    echo      Visit https://rustup.rs if this fails.
    powershell -NoProfile -ExecutionPolicy Bypass -Command "Invoke-WebRequest -Uri https://win.rustup.rs/x86_64 -OutFile rustup-init.exe; .\rustup-init.exe -y --default-toolchain stable --profile default"
    call "%USERPROFILE%\.cargo\env.bat"
    where cargo >nul 2>nul || goto :error_rust
)
echo  [OK] Rust found: & cargo --version

REM ── Step 2: check Node ──
where node >nul 2>nul
if errorlevel 1 (
    echo  [!] Node.js not found. Please install Node 18+ from https://nodejs.org
    pause
    exit /b 1
)
echo  [OK] Node found: & node --version

REM ── Step 3: check for VS Build Tools (required for rust MSVC linker) ──
where link.exe >nul 2>nul
if errorlevel 1 (
    echo  [!] MSVC linker not found on PATH.
    echo      Install "Visual Studio 2022 Build Tools" with the
    echo      "Desktop development with C++" workload from:
    echo      https://visualstudio.microsoft.com/visual-cpp-build-tools/
    echo.
    echo      After install, re-run this script.
    pause
    exit /b 1
)
echo  [OK] MSVC linker available

REM ── Step 4: install JS deps ──
if not exist "node_modules" (
    echo  [*] Installing JS dependencies…
    call npm install || goto :error
) else (
    echo  [OK] node_modules present
)

REM ── Step 5: build ──
echo  [*] Building Lunar Browser (release). First build takes ~10-15 min…
echo.
call npm run build || goto :error

REM ── Step 6: locate outputs ──
echo.
echo  ════════════════════════════════════════════════════════════════
echo   ✅ BUILD COMPLETE
echo  ════════════════════════════════════════════════════════════════
echo.

set "FOUND=0"
for %%P in (
    "src-tauri\target\release\bundle\nsis\*.exe"
    "src-tauri\target\release\bundle\msi\*.msi"
    "src-tauri\target\release\lunar-browser.exe"
) do (
    if exist "%%~P" (
        echo   • %%~P
        set "FOUND=1"
    )
)

if "!FOUND!"=="0" (
    echo  [!] No build outputs found. Check the log above for errors.
    pause
    exit /b 1
)

echo.
echo  Open the NSIS .exe to install Lunar Browser, or run the
echo  portable lunar-browser.exe directly.
echo.
pause
exit /b 0

:error_rust
echo.
echo  [✗] Rust install failed. Please install manually from https://rustup.rs
pause
exit /b 1

:error
echo.
echo  [✗] Build failed. See errors above.
pause
exit /b 1
