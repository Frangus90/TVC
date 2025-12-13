# TVC - TV Calendar

A desktop app for tracking TV shows and scheduling episodes.

## Features

- **Search & Track Shows**: Find TV shows via TVDB API and add them to your library
- **Calendar Views**: Month, Week, and Agenda views for your scheduled episodes
- **Episode Scheduling**: Schedule episodes manually with multi-select and season scheduling
- **Day Detail View**: Click any day to see and manage all episodes scheduled
- **Watch Tracking**: Mark episodes as watched
- **Auto-Updates**: App checks for updates on startup and prompts to install

## Tech Stack

- **Backend**: Tauri v2 (Rust)
- **Frontend**: Svelte 5 + TypeScript
- **Styling**: Tailwind CSS v4
- **Database**: SQLite (local storage)
- **Updates**: Tauri updater plugin with GitHub Releases

## Installation

Download the latest installer from [Releases](https://github.com/Frangus90/TVC/releases).

## Development

### Prerequisites

- Node.js 18+
- Rust (latest stable)
- pnpm or npm

### Setup

```bash
# Install dependencies
npm install

# Run in development mode
npm run tauri dev
```

### Building

```bash
npm run tauri build
```

The installer will be in `src-tauri/target/release/bundle/nsis/`.

## Releasing Updates

### 1. Update Version

Update version in three files:
- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`

### 2. Build with Signing

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content "C:\Users\aleks\.tauri\tvc-pwd.key" -Raw
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD="tvc123"
npm run tauri build
```

### 3. Create latest.json

Create `latest.json` in the bundle folder with:

```json
{
  "version": "X.X.X",
  "notes": "Release notes here",
  "pub_date": "2024-12-13T00:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "url": "https://github.com/Frangus90/TVC/releases/download/vX.X.X/TVC_X.X.X_x64-setup.exe",
      "signature": "<contents of .sig file>"
    }
  }
}
```

### 4. Create GitHub Release

```bash
gh release create vX.X.X \
  TVC_X.X.X_x64-setup.exe \
  TVC_X.X.X_x64-setup.exe.sig \
  latest.json \
  --title "vX.X.X - Title" \
  --notes "Release notes"
```

## Project Structure

```
TVC/
├── src/                    # Frontend (Svelte)
│   ├── lib/
│   │   ├── components/     # UI components
│   │   ├── stores/         # Svelte stores (state management)
│   │   └── services/       # API services
│   └── App.svelte          # Main app component
├── src-tauri/              # Backend (Rust)
│   ├── src/
│   │   ├── lib.rs          # Tauri setup & commands
│   │   ├── db/             # Database operations
│   │   └── tvdb/           # TVDB API client
│   ├── migrations/         # SQLite migrations
│   └── tauri.conf.json     # Tauri configuration
└── package.json
```

## License

Private project.
