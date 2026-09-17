# TVC - TV Calendar

A modern desktop application for tracking TV shows and movies, scheduling episodes, and managing your watch history.

## Features

### Core Functionality

- **Search & Track**: Search for TV shows and movies via the TMDB API and add them to your library
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
- **European Dates & Times**: Dates use `DD.MM.YYYY` and times use the 24-hour clock (`HH:mm`), in your computer's local timezone

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
- **APIs**: TMDB (TV shows and movies)
- **Updates**: Tauri updater plugin with GitHub Releases

## Installation

Download the latest installer from [Releases](https://github.com/Frangus90/TVC/releases).

### System Requirements

- Windows 10/11, macOS 10.15+, or Linux (with GTK3)
- ~100MB disk space
- Internet connection for API access and updates

## Development

Debug builds use the separate `com.tvc.app.dev` identity, a **TVC — Development** window title, and `tvc_dev.db`. They can run beside the release app. On the first launch after this change, TVC copies an existing **development** database into the new development profile using a consistent SQLite snapshot; it never overwrites an existing new profile. The release database is never used as a source.

Run `npm run check` before releasing. It checks Svelte templates and TypeScript, runs frontend regressions and offline Rust tests against disposable databases, and builds the frontend. The release script runs this command before changing versions or Git state. After installing new Rust dependencies, run `cargo fetch --locked --manifest-path src-tauri/Cargo.toml` once before using the offline checks.

Newly synced episodes follow their current air date unless you set a custom date. Clearing an episode's custom schedule restores its air date. Older saved schedules are retained because an automatic date and an intentional override cannot reliably be distinguished; use **Use air dates** in the day view to reset those you want to follow TMDB.

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

### Date and Time Display

Use the shared helpers in `src/lib/utils/dateFormat.ts` for all user-facing dates and times, including image exports:

- Full dates: `DD.MM.YYYY`, with leading zeros (e.g. `09.01.2026`).
- Times: `HH:mm`, using the 24-hour clock (e.g. `17:05`). Combined: `09.01.2026 17:05`.
- Use the computer's local timezone, regardless of its regional date format. Date-only values retain their calendar date; UTC timestamps are converted to local time.
- Relative labels (`Today`, `Tomorrow`, `3 hours ago`) and compact calendar headings can remain contextual.

Do not use locale-dependent `toLocaleString()` or `toLocaleDateString()` for date/time display. Keep database values, API payloads, grouping keys, and sortable filenames in their existing ISO formats; format them only when displaying them.

### Backup and Restore

Data Management → Export saves a versioned JSON backup. Format **3.0** includes:

- Shows, episodes, and movies, including watch dates, custom schedules, ratings, notes, tags, archived status, and tier placement/order.
- Custom tiers.
- Awards, categories, nominees, results, and saved predictions, including original titles, source keys, Wikipedia page names, and pick timestamps.
- Plex title corrections, scrobble logs, and change history (including historical references to removed titles).

Exports read all included tables from one database snapshot. Import shows counts and scope before confirmation, validates supported versions and relationships, and rolls back the replacement if a record fails. Import cannot run alongside a bulk TMDB sync. Restart TVC after a successful import to reload all restored state.

**Excluded:** app preferences, integration configuration/credentials, Sonarr/Radarr import links, racing data/preferences, notifications, and cached cast/crew. Existing preferences, credentials, racing data, and notifications stay unchanged on import. Cached cast/crew and import links are cleared; metadata refresh and integration import rebuild them. Sync reports are cleared, and enabled sync schedules start their next interval from the restore time.

Older **1.0** (TVDB, including files without a version) and **2.0** (TMDB) library backups remain supported. They keep existing awards/predictions, but cannot restore Plex title corrections, scrobble logs, or change history; import explicitly warns that these library-related records will be cleared. Export a new backup before replacing a library if you need to preserve them. Legacy TVDB imports still perform provider remapping and a separate metadata refresh; unmatched shows and episodes are reported.

Backup files contain viewing history and personal notes in plain text. Integration credentials are not included.

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
│   │   ├── tmdb/          # TMDB API client
│   │   ├── plex/          # Plex integration
│   │   └── arr/           # Sonarr/Radarr integration
│   └── migrations/        # Database migrations
└── static/                # Static assets
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

See [LICENSE](LICENSE) file for details.

