# Keep lock files aligned with the versions already selected by the release script.
param(
    [string]$ProjectRoot = (Split-Path -Parent $PSScriptRoot)
)

$ErrorActionPreference = "Stop"
Push-Location $ProjectRoot
try {
    # Preserve locked third-party versions; refresh the workspace package version offline.
    cargo update --workspace --offline --manifest-path src-tauri/Cargo.toml
    if ($LASTEXITCODE -ne 0) { throw "Could not synchronize Cargo.lock. Release stopped." }

    # npm updates both root version entries in package-lock.json without installing
    # dependencies, running lifecycle scripts, or creating a Git commit/tag.
    $packageVersion = (Get-Content package.json -Raw | ConvertFrom-Json).version
    npm version $packageVersion --allow-same-version --no-git-tag-version --ignore-scripts
    if ($LASTEXITCODE -ne 0) { throw "Could not synchronize package-lock.json. Release stopped." }
} finally {
    Pop-Location
}
