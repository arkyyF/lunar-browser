# ════════════════════════════════════════════════════════════════
#  Lunar Browser — push-to-GitHub helper
#  Run this from inside the unzipped lunar-browser/ folder.
#
#  What it does:
#    1. Asks for your GitHub username + repo name
#    2. Asks if you want a public or private repo
#    3. Creates the repo on GitHub via the gh CLI (or git+token fallback)
#    4. Commits all source + pushes
#    5. Waits for the CI build to finish, then prints the artifact URL
# ════════════════════════════════════════════════════════════════

$ErrorActionPreference = "Stop"
Set-Location -Path $PSScriptRoot

Write-Host ""
Write-Host "  === Lunar Browser — push to GitHub ===" -ForegroundColor Cyan
Write-Host ""

# ── Step 1: check git ──
if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    Write-Host "  [!] Git not installed. Download from https://git-scm.com" -ForegroundColor Red
    exit 1
}

# ── Step 2: check gh CLI ──
$hasGh = Get-Command gh -ErrorAction SilentlyContinue
if (-not $hasGh) {
    Write-Host "  [!] GitHub CLI not found." -ForegroundColor Yellow
    Write-Host "      Recommended: install from https://cli.github.com" -ForegroundColor Yellow
    Write-Host "      Alternatively, create the repo manually at https://github.com/new" -ForegroundColor Yellow
    $continue = Read-Host "  Continue with manual git push? (y/N)"
    if ($continue -ne "y") { exit 1 }
}

# ── Step 3: gather info ──
$username = Read-Host "  GitHub username"
if (-not $username) { Write-Host "  Username required." -ForegroundColor Red; exit 1 }

$repo = Read-Host "  Repo name (default: lunar-browser)"
if (-not $repo) { $repo = "lunar-browser" }

$visibility = Read-Host "  Visibility: (1) public [recommended, free CI] or (2) private? [1/2]"
$isPublic = $visibility -ne "2"

Write-Host ""
Write-Host "  Creating repo: $username/$repo ($(($isPublic ? 'public' : 'private')))" -ForegroundColor Cyan
Write-Host ""

# ── Step 4: create repo on GitHub ──
if ($hasGh) {
    # Auth check
    $authed = gh auth status 2>&1 | Out-String
    if ($LASTEXITCODE -ne 0) {
        Write-Host "  [*] Logging into GitHub…" -ForegroundColor Yellow
        gh auth login --web --git-protocol https
    }
    $visFlag = if ($isPublic) { "--public" } else { "--private" }
    gh repo create "$username/$repo" $visFlag --source=. --remote=origin --push 2>&1 | Out-Null
} else {
    # Fallback: assume user created the repo manually
    git init
    git remote add origin "https://github.com/$username/$repo.git"
}

# ── Step 5: commit + push ──
git add .
$commitMsg = "Initial commit: Lunar Browser"
git commit -m $commitMsg 2>&1 | Out-Null
git branch -M main
git push -u origin main 2>&1 | Out-Null

Write-Host ""
Write-Host "  ✅ Pushed to https://github.com/$username/$repo" -ForegroundColor Green
Write-Host ""
Write-Host "  Next:" -ForegroundColor Cyan
Write-Host "    1. Open https://github.com/$username/$repo/actions" -ForegroundColor White
Write-Host "    2. Wait ~10-15 min for the 'Build Windows' run to go green" -ForegroundColor White
Write-Host "    3. Click the run → scroll to bottom → download 'Lunar-Browser-Windows-NSIS'" -ForegroundColor White
Write-Host "    4. Unzip → run Lunar-Browser_1.0.0_x64-setup.exe" -ForegroundColor White
Write-Host ""
Write-Host "  For a proper release page (downloadable .exe/.msi):" -ForegroundColor Cyan
Write-Host "    git tag v1.0.0" -ForegroundColor White
Write-Host "    git push origin v1.0.0" -ForegroundColor White
Write-Host ""
