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

# Get version if not provided
if (-not $Version) {
    $currentVersion = (Get-Content "$ProjectRoot\package.json" | ConvertFrom-Json).version
    Write-Host "Current version: $currentVersion" -ForegroundColor Yellow
    $Version = Read-Host "Enter new version "
    if (-not $Version) {
        Write-Err "Version is required"
        exit 1
    }
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
