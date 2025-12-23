# Toggle Dev Mode for Dummy Update Testing
# This script allows you to temporarily show/hide the dummy update feature
# for testing purposes

$filePath = Join-Path $PSScriptRoot "..\src\lib\components\DataManagement.svelte"

if (-not (Test-Path $filePath)) {
    Write-Host "Error: DataManagement.svelte not found at $filePath" -ForegroundColor Red
    exit 1
}

# Read the file
$content = Get-Content $filePath -Raw

# Check current state
$isDevMode = $content -match 'showDummyUpdate = import\.meta\.env\.DEV && true'
$isProdMode = $content -match 'showDummyUpdate = false'

Write-Host ""
Write-Host "Current state:" -ForegroundColor Cyan
if ($isDevMode) {
    Write-Host "  Dev mode: ENABLED (feature visible)" -ForegroundColor Green
} elseif ($isProdMode) {
    Write-Host "  Dev mode: DISABLED (feature hidden)" -ForegroundColor Yellow
} else {
    Write-Host "  Unknown state" -ForegroundColor Red
}
Write-Host ""

# Prompt user
$choice = Read-Host "Enter 'D' for Dev mode (show feature) or 'P' for Production mode (hide feature)"

if ($choice -eq "D" -or $choice -eq "d") {
    # Set to dev mode
    $content = $content -replace 'showDummyUpdate = (import\.meta\.env\.DEV && true|false)', 'showDummyUpdate = import.meta.env.DEV && true'
    Set-Content -Path $filePath -Value $content -NoNewline
    Write-Host "Switched to DEV mode - dummy update feature will be visible" -ForegroundColor Green
} elseif ($choice -eq "P" -or $choice -eq "p") {
    # Set to production mode
    $content = $content -replace 'showDummyUpdate = (import\.meta\.env\.DEV && true|false)', 'showDummyUpdate = false'
    Set-Content -Path $filePath -Value $content -NoNewline
    Write-Host "Switched to PRODUCTION mode - dummy update feature will be hidden" -ForegroundColor Yellow
} else {
    Write-Host "Invalid choice. No changes made." -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "File updated successfully!" -ForegroundColor Green

