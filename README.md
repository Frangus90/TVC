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

