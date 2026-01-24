# TVC - TV Calendar

A modern desktop application for tracking TV shows and movies, scheduling episodes, and managing your watch history.

## Features

### Core Functionality

- **Search & Track**: Search for TV shows and movies via TVDB/TMDB APIs and add them to your library
- **Calendar Views**: Multiple calendar views (Month, Week, Agenda, Tier) for organizing your content
- **Episode Scheduling**: Schedule episodes manually with multi-select and season scheduling
- **Watch Tracking**: Mark episodes and movies as watched with detailed tracking
- **Day Detail View**: Click any day to see and manage all scheduled content
- **Statistics Dashboard**: Track watch time, completion rates, and watch history

### Integrations

- **Sonarr & Radarr**: Import your library directly from Sonarr and Radarr servers
- **Plex Scrobbler**: Automatically mark content as watched when you watch it in Plex (requires Plex Pass)
- **Multiple Servers**: Add and manage multiple ARR servers

### Organization & Management

- **Ratings & Tier List**: Rate shows and movies with half-star precision and view them in a tier list
- **Drag & Drop Rating**: Drag items to rate them visually in the Tier view
- **Data Management**: Find duplicates, clean up orphaned episodes, optimize database
- **Backup & Restore**: Export and import your data for backup purposes
- **Change History**: Track all changes made to your library

### User Experience

- **Theme Customization**: Customize accent colors, font sizes, spacing, and more
- **Accessibility**: Full keyboard navigation, screen reader support, and ARIA labels
- **Auto-Updates**: Automatic update checks with in-app update notifications
- **System Tray**: Minimize to system tray (Windows, macOS, Linux)
- **Smart Search**: Auto-search as you type with result count and relevance sorting

### Media Information

- **Cast & Crew**: View cast and crew information for shows and movies
- **Trailers**: Watch trailers directly from the app
- **Posters & Images**: Beautiful poster displays throughout the app
- **Network Information**: See which networks your shows are on

## Tech Stack

- **Backend**: Tauri v2 (Rust)
- **Frontend**: Svelte 5 + TypeScript
- **Styling**: Tailwind CSS v4
- **Database**: SQLite (local storage)
- **APIs**: TVDB (TV shows), TMDB (movies)
- **Updates**: Tauri updater plugin with GitHub Releases

## Installation

Download the latest installer from [Releases](https://github.com/Frangus90/TVC/releases).

### System Requirements

- Windows 10/11, macOS 10.15+, or Linux (with GTK3)
- ~100MB disk space
- Internet connection for API access and updates

## Development

### Prerequisites

- Node.js 18+ (recommended: latest LTS)
- Rust (latest stable) - install via [rustup](https://rustup.rs/)
- npm or pnpm

### Setup

```bash
# Clone the repository
git clone https://github.com/Frangus90/TVC.git
cd TVC

# Install dependencies
npm install

# Run in development mode
npm run tauri dev
```

### Building

```bash
# Build for production
npm run tauri build
```

The built application will be in `src-tauri/target/release/`.

## Project Structure

```
TVC/
├── src/                    # Frontend (Svelte + TypeScript)
│   ├── lib/
│   │   ├── components/     # Svelte components
│   │   ├── stores/         # State management
│   │   ├── utils/          # Utility functions
│   │   └── types/          # TypeScript type definitions
│   └── App.svelte          # Main app component
├── src-tauri/              # Backend (Rust)
│   ├── src/
│   │   ├── commands/      # Tauri commands
│   │   ├── db/            # Database utilities
│   │   ├── tvdb/          # TVDB API client
│   │   ├── plex/          # Plex integration
│   │   └── arr/           # Sonarr/Radarr integration
│   └── migrations/        # Database migrations
└── static/                # Static assets
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

See [LICENSE](LICENSE) file for details.

