# TVC Release Script
# Automates: version bump, build, signing, and GitHub release

param(
    [string]$Version,
    [string]$Notes
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)

# Colors for output
function Write-Step { param($msg) Write-Host "`n>> $msg" -ForegroundColor Cyan }
function Write-Success { param($msg) Write-Host $msg -ForegroundColor Green }
function Write-Err { param($msg) Write-Host $msg -ForegroundColor Red }
function Write-Info { param($msg) Write-Host $msg -ForegroundColor Yellow }

# Check latest GitHub release
Write-Step "Checking GitHub releases..."
$latestRelease = $null
$allReleases = @()
try {
    $latestRelease = gh release view --json tagName,publishedAt,name 2>$null | ConvertFrom-Json
    $allReleases = gh release list --limit 5 --json tagName,publishedAt 2>$null | ConvertFrom-Json
} catch {
    Write-Info "Could not fetch GitHub releases (this is OK for first release)"
}

if ($latestRelease) {
    Write-Host "Latest GitHub release: " -NoNewline
    Write-Host $latestRelease.tagName -ForegroundColor Green -NoNewline
    Write-Host " (published: $($latestRelease.publishedAt.Substring(0,10)))"

    if ($allReleases.Count -gt 1) {
        Write-Host "Recent releases: $($allReleases.tagName -join ', ')" -ForegroundColor DarkGray
    }
} else {
    Write-Info "No existing releases found on GitHub"
}

# Get version if not provided
if (-not $Version) {
    $currentVersion = (Get-Content "$ProjectRoot\package.json" | ConvertFrom-Json).version
    Write-Host "`nCurrent local version: " -NoNewline
    Write-Host $currentVersion -ForegroundColor Yellow
    $Version = Read-Host "Enter version to release"
    if (-not $Version) {
        Write-Err "Version is required"
        exit 1
    }
}

# Check if this version/tag already exists
$tagExists = $false
$releaseExists = $false
try {
    $existingRelease = gh release view "v$Version" --json tagName 2>$null | ConvertFrom-Json
    if ($existingRelease) {
        $releaseExists = $true
        $tagExists = $true
    }
} catch {
    # Release doesn't exist, check if tag exists
    $tagCheck = git tag -l "v$Version" 2>$null
    if ($tagCheck) {
        $tagExists = $true
    }
}

if ($releaseExists) {
    Write-Host "`n" -NoNewline
    Write-Err "Release v$Version already exists on GitHub!"
    Write-Host "Options:"
    Write-Host "  1. Delete the existing release and re-release"
    Write-Host "  2. Cancel and choose a different version"
    $choice = Read-Host "`nEnter choice (1/2)"

    if ($choice -eq "1") {
        Write-Step "Deleting existing release v$Version..."
        gh release delete "v$Version" --yes 2>&1 | Out-Null
        # Also delete the tag so we can recreate it
        git tag -d "v$Version" 2>&1 | Out-Null
        git push origin --delete "v$Version" 2>&1 | Out-Null
        Write-Success "  Deleted existing release and tag"
    } else {
        Write-Host "Cancelled."
        exit 0
    }
} elseif ($tagExists) {
    Write-Info "`nTag v$Version exists but has no release (was deleted)"
    Write-Host "The release will be uploaded to the existing tag."
}

# Get release notes if not provided
if (-not $Notes) {
    Write-Host "`nEnter release notes (press Enter twice to finish):" -ForegroundColor Yellow
    $notesLines = @()
    while ($true) {
        $line = Read-Host
        if ($line -eq "" -and $notesLines.Count -gt 0 -and $notesLines[-1] -eq "") {
            break
        }
        $notesLines += $line
    }
    $Notes = ($notesLines | Where-Object { $_ -ne "" }) -join "`n"
    if (-not $Notes) {
        $Notes = "v$Version release"
    }
}

Write-Host "`n========================================" -ForegroundColor Magenta
Write-Host "  TVC Release v$Version" -ForegroundColor Magenta
Write-Host "========================================" -ForegroundColor Magenta
Write-Host "`nRelease notes:`n$Notes`n"

$confirm = Read-Host "Proceed with release? (y/n)"
if ($confirm -ne "y") {
    Write-Host "Cancelled."
    exit 0
}

# Step 1: Update version in all files
Write-Step "Updating version to $Version..."

# package.json
$packageJson = Get-Content "$ProjectRoot\package.json" -Raw
$packageJson = $packageJson -replace '"version": "[^"]*"', "`"version`": `"$Version`""
Set-Content "$ProjectRoot\package.json" $packageJson -NoNewline
Write-Success "  Updated package.json"

# Cargo.toml - only update [package] version, not dependencies
$cargoPath = "$ProjectRoot\src-tauri\Cargo.toml"
$cargoLines = Get-Content $cargoPath
$inPackage = $false
$newCargoLines = @()

foreach ($line in $cargoLines) {
    if ($line -match '^\[package\]') {
        $inPackage = $true
    } elseif ($line -match '^\[') {
        $inPackage = $false
    }

    if ($inPackage -and $line -match '^version = "') {
        $newCargoLines += "version = `"$Version`""
    } else {
        $newCargoLines += $line
    }
}

Set-Content $cargoPath ($newCargoLines -join "`n")
Write-Success "  Updated Cargo.toml"

# tauri.conf.json
$tauriConf = Get-Content "$ProjectRoot\src-tauri\tauri.conf.json" -Raw
$tauriConf = $tauriConf -replace '"version": "[^"]*"', "`"version`": `"$Version`""
Set-Content "$ProjectRoot\src-tauri\tauri.conf.json" $tauriConf -NoNewline
Write-Success "  Updated tauri.conf.json"

# Sidebar.svelte (version display)
$sidebar = Get-Content "$ProjectRoot\src\lib\components\layout\Sidebar.svelte" -Raw
$sidebar = $sidebar -replace '>v[\d\.]+</p>', ">v$Version</p>"
Set-Content "$ProjectRoot\src\lib\components\layout\Sidebar.svelte" $sidebar -NoNewline
Write-Success "  Updated Sidebar.svelte"

# Step 2: Build with signing
Write-Step "Building with signing..."

$keyPath = "$env:USERPROFILE\.tauri\tvc-pwd.key"
if (-not (Test-Path $keyPath)) {
    Write-Err "Signing key not found at $keyPath"
    exit 1
}

$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content $keyPath -Raw
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "tvc123"

Push-Location $ProjectRoot
try {
    npm run tauri build
    if ($LASTEXITCODE -ne 0) {
        throw "Build failed"
    }
} finally {
    Pop-Location
}

Write-Success "  Build complete!"

# Step 3: Create latest.json
Write-Step "Creating latest.json..."

$bundlePath = "$ProjectRoot\src-tauri\target\release\bundle\nsis"
$exeName = "TVC_${Version}_x64-setup.exe"
$sigFile = "$bundlePath\$exeName.sig"

if (-not (Test-Path $sigFile)) {
    Write-Err "Signature file not found: $sigFile"
    exit 1
}

$signature = (Get-Content $sigFile -Raw).Trim()

# Format notes for JSON (escape special characters properly)
$escapedNotes = "v$Version - " + ($Notes -replace '\\', '\\\\' -replace '"', '\"' -replace "`r`n", ' - ' -replace "`n", ' - ')

# Build JSON manually to avoid newline issues
$latestJson = '{
  "version": "' + $Version + '",
  "notes": "' + $escapedNotes + '",
  "pub_date": "' + (Get-Date -Format 'yyyy-MM-ddTHH:mm:ssZ') + '",
  "platforms": {
    "windows-x86_64": {
      "url": "https://github.com/Frangus90/TVC/releases/download/v' + $Version + '/' + $exeName + '",
      "signature": "' + $signature + '"
    }
  }
}'

# Write without BOM (UTF8 with BOM breaks JSON parsing)
[System.IO.File]::WriteAllText("$bundlePath\latest.json", $latestJson)
Write-Success "  Created latest.json"

# Step 4: Create GitHub release
Write-Step "Creating GitHub release v$Version..."

# Write release notes to a temp file to avoid escaping issues
$releaseNotesFile = "$env:TEMP\tvc-release-notes.md"
$releaseNotesContent = "## What's New`r`n`r`n$Notes"
Set-Content -Path $releaseNotesFile -Value $releaseNotesContent -Encoding UTF8

Push-Location $ProjectRoot
try {
    $result = gh release create "v$Version" `
        "$bundlePath\$exeName" `
        "$bundlePath\$exeName.sig" `
        "$bundlePath\latest.json" `
        --title "v$Version" `
        --notes-file $releaseNotesFile 2>&1

    if ($LASTEXITCODE -ne 0) {
        Write-Err "gh output: $result"
        throw "GitHub release creation failed"
    }

    Write-Host $result
} finally {
    Pop-Location
    Remove-Item $releaseNotesFile -ErrorAction SilentlyContinue
}

Write-Host "`n========================================" -ForegroundColor Green
Write-Host "  Release v$Version complete!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host "`nRelease URL: https://github.com/Frangus90/TVC/releases/tag/v$Version"
Write-Host "`nUsers with older versions will now see the update popup!"
