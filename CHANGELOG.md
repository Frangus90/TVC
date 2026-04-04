# Changelog

All notable changes to TVC will be documented in this file.

## [0.10.0]

### Decoupled Tier List System

- **Tier-Only Entries**: Rank shows and movies in tier lists without tracking/following them — no episode syncing, no calendar clutter
- **Customizable Tiers**: Fully customizable tier names, colors, drag-to-reorder, add/remove — Tiermaker-style
- **Tier Presets**: Choose between 10-Star, 5-Star, or fully Custom tier layouts
- **Manual Entries**: Add shows/movies not found in TVDB/TMDB as manual entries with negative IDs
- **Smart Rating Widget**: Star ratings for star presets, tier picker dropdown for custom presets — adapts automatically
- **Tier Search Modal**: Dedicated "Add to Tier List" modal with TVDB/TMDB search and manual entry modes
- **Tiermaker Import**: Paste HTML from any Tiermaker list to bulk-import items — auto-creates matching tiers, batch-searches TMDB/TVDB, and supports manual matching for unresolved items
- **Context Menus**: Move items between tiers, promote/demote between tracked and tier-only, remove from tier list
- **Detail Modal Integration**: Rate shows/movies from their detail modals; tier-only items hide calendar-specific controls
- **Backup Support**: Tier data included in backup/restore with backwards-compatible JSON format
- **Tier List Settings Tab**: New settings tab for managing tiers, presets, and Tiermaker imports

## [0.9.5]

### Unified Settings

- **Single Settings Modal**: Consolidated all 5 settings panels (Appearance, Notifications, Racing, Plex, Arr) into one modal with vertical tab navigation
- **Sidebar Cleanup**: Replaced separate Arr and Plex buttons with a single Settings gear button
- **Appearance Overhaul**: Reorganized into Theme, Layout, and Accessibility sections with accent color presets, cleaner toggles, and a Reset to Defaults button
- **Collapsible Racing Categories**: Racing settings categories (Formula, Endurance, etc.) are now collapsible with series counts
- **Consistent Navigation**: All settings entry points (sidebar, notification center, race calendar) now open the unified modal on the correct tab

## [0.9.4]

### Accessibility

- Fixed missing `aria-label` attributes on all toggle buttons in Notification Settings and Racing Settings
- Fixed notification popup using a non-interactive element for click handling

## [0.9.3]

### In-App Notification System

- **Bespoke Notifications**: Custom in-app notification popups replace easy-to-miss Windows notifications — fully styled, positioned, and configurable
- **Notification Center**: Bell icon in the header with unread badge, dropdown panel to browse, filter, and manage all notifications
- **Notification Sounds**: Four synthesized sounds (Chime, Bell, Soft, Alert) via Web Audio API — no external files needed
- **Tray Notifications**: When the app is minimized to the system tray, notifications automatically fall back to OS notifications so you never miss an event
- **Per-Category Toggles**: Enable or disable notifications independently for Racing, Plex, Premieres, Updates, and System events
- **Display Settings**: Configure popup position (4 corners), auto-dismiss duration, max visible popups, and sound volume
- **Plex Integration**: Plex scrobble events (episodes and movies) now trigger in-app notifications alongside the existing scrobble tracking
- **Racing Integration**: Racing session reminders now route through the unified notification system with tray fallback
- **Dev Testing**: Development-only panel to fire test notifications for each type individually or all at once

## [0.9.2]

### Bug Fixes

- **Timezone Fix**: Session times from ICS feeds with TZID offsets (e.g. MotoGP, Moto2, Moto3) are now correctly converted to UTC so the calendar always displays times in your local timezone instead of the venue's local time
- **Calendar Refresh Fix**: Refreshing racing data (or toggling series) now immediately updates the calendar view instead of requiring a month navigation or series removal/re-add

## [0.9.1]

### Bug Fixes

- **Config Save Fix**: Fixed notification config failing to save due to parameter name mismatch between frontend and backend
- **Config Data Loss Fix**: Fixed `UPDATE` vs `INSERT OR REPLACE` in racing config — previously wiped `last_refreshed` and `created_at` on every save
- **Color Reset Fix**: Fixed crash when resetting a series color to default (was sending `null` to a non-optional parameter)
- **Startup Race Condition**: Fixed racing scheduler querying the database before migrations finished, causing "no such table" errors on first launch

### Improvements

- **Change All Lead Times**: Replaced the non-functional "Default Lead Time" setting with a "Change All Lead Times" action that actually updates all enabled series at once
- **Responsive Filter Chips**: Series filter chips in the calendar now wrap to multiple lines instead of getting crushed at smaller window sizes
- **Code Cleanup**: Removed unused `get_upcoming_events` function (dead code)

## [0.9.0]

### Racing Calendar

- **Motorsport Tracking**: A dedicated Racing tab with its own month-view calendar for tracking motorsport sessions (practice, qualifying, sprint, race)
- **21 Pre-configured Series**: Formula 1, MotoGP, WEC, IndyCar, NASCAR, WRC, Formula E, and more — all ready to enable with one click
- **ICS Calendar Feeds**: Full season schedules pulled from public ICS/iCalendar feeds with manual refresh
- **Desktop Notifications**: Get notified before sessions start with configurable per-series lead times
- **Series Customization**: Override brand colors and ICS feed URLs per series
- **Filter by Series**: Click series chips above the calendar to filter the view to a single series
- **Session Details**: Click any day to see a detailed breakdown of all sessions with times, circuits, and series info

## [0.8.2]

### What's New

- **Clickable Version Number**: The version number in the sidebar footer is now a button — click it to see what changed in the latest update
- **What's New Modal**: Shows the changelog for the current version with section headers and bullet points, plus an expandable list of older versions
- **Auto-Show After Updates**: When you update to a new version, the What's New modal automatically opens on first launch so you never miss new features
- **Unseen Indicator**: A sparkle icon and accent dot appear next to the version number when there are unread changes
- **Dynamic Version Display**: The version number is now fetched from the app instead of being hardcoded

## [0.8.1]

### Add Button Feedback

- **Visual Confirmation**: When adding a movie or show from search results, the `+` button now shows a spinner while adding, then turns into a green checkmark once added
- Already-tracked items show the green checkmark immediately when search results load
- Prevents accidental double-adds by disabling the button during and after the add operation

## [0.8.0]

### Calendar Content Filter

- **Show/Movie Filter**: Added an All / Shows / Movies toggle in the header to filter what's displayed on the calendar
- Works across Month, Week, and Agenda views — hidden on Tier view since it has its own tabs
- Filter persists when switching between calendar views
- No re-fetch needed — switching is instant

### Data Management Cleanup

- **Removed History Tab**: Removed the unused History tab from Data Management — activity is already tracked in the Plex and Stats tabs
- Cleaned up related backend commands, database queries, and frontend code

## [0.7.9]

### Live Date & Time Tracking

- **Real-Time Clock**: The app now tracks the current system time with a 60-second tick, so date-dependent UI (aired vs upcoming) stays accurate even if the app is left open for days
- **Automatic Day Rollover**: At midnight the calendar automatically navigates to the new day and refreshes all episode and movie data

### Bug Fixes

- **Fixed unused imports in PlexSettings**: Removed unused `ExternalLink` import and `config` template variable
- **Fixed broken type re-exports**: Corrected `PeriodStatistics` to `PeriodStats` and removed non-existent `Toast` type export

## [0.7.8]

### Tier List Rankings

- **Tier View Overhaul**: Redesigned tier view with descriptive labels (Masterpiece, Excellent, Great, etc.) alongside star ratings
- **Click-to-Rate Menu**: Hover over any poster to reveal a 3-dot menu for quickly changing or removing ratings without drag-and-drop
- **Within-Tier Ordering**: Drag posters horizontally within the same tier to reorder them — order is saved and persists
- **Sidebar Sync**: Switching between Shows/Movies in the tier view now syncs the sidebar tab automatically
- **Improved Empty State**: Helpful icon and instructions shown when no items are rated yet
- **Show Names on Posters**: Item names now display below posters in the tier view for easier identification

### Bug Fixes

- **Fixed Duplicate System Tray Icons**: Removed redundant declarative tray icon config that caused two icons to appear when minimized

## [0.7.7]

### Single Instance

- **Prevent Multiple Windows**: The app now enforces single-instance mode - if you try to open TVC while it's already running, the existing window will be focused instead of opening a duplicate
- Uses `tauri-plugin-single-instance` to detect and handle duplicate launches
- When a second instance is launched, the existing window is shown, unminimized, and brought to focus

## [0.7.6]

### Date & Time Formatting

- **App-wide defaults**: All dates and times now use **D.M.YYYY** (e.g. 29.01.2026), **24-hour clock**, and your **PC's local timezone**
- **Plex Scrobbler**: Fixed scrobble timestamps using 12-hour clock, wrong timezone, and incorrect date format—now matches app defaults
- **Show modal**: Episode air dates and show first-aired dates in the TV show modal now display as D.M.YYYY instead of YYYY-MM-DD
- **Calendar, statistics, data management**: Day headers, watch history, relative dates, and "last synced" labels all use the new formatting
- **Centralized formatting**: New `dateFormat` utility ensures consistent parsing (including DB UTC timestamps) and display across the app

### Bug Fixes

- **Plex Scrobbler**: Scrobble log timestamps now show correct timezone (PC local) and 24-hour time

### Code Quality

- **Rust**: Resolved compiler warnings—unused variables in Plex webhook handler, dead code in validation helpers, removed redundant port check

## [0.7.5]

### Bug Fixes

- **Fixed Export Crash**: Fixed a critical bug that caused the app to crash when exporting data in release builds
- The issue was caused by a type mismatch between the database schema (REAL/f64 for ratings) and the backup structs (i32)
- All rating fields in backup/export structures now correctly use f64 to match the database schema
- Export functionality now works correctly in both development and production builds

## [0.7.4]

### Code Quality & Performance Improvements

- **Centralized Logging**: Replaced 105+ console statements with a structured logging system
- **Error Handling**: Standardized error handling patterns across the application
- **Type Safety**: Fixed all type safety issues, replaced `any` types with proper TypeScript interfaces
- **Database Optimization**: Consolidated duplicate database connections into a shared utility
- **Request Deduplication**: Prevents duplicate API calls when multiple components request the same data
- **Input Validation**: Added validation layer for all user inputs with sanitization
- **Accessibility**: Enhanced ARIA labels, keyboard navigation, and screen reader support
- **Error Boundary**: Added error boundary component to catch and handle unhandled errors gracefully
- **Centralized Configuration**: All configurable values now in one place for easier maintenance

### New Features

- **Exit Button**: Added exit button in header that fully closes the app (not just minimize to tray)
- **Confirmation Dialog**: Exit button shows a warning dialog to prevent accidental full closure

### Developer Experience

- **Type Organization**: Centralized type definitions in dedicated module
- **Documentation**: Added JSDoc comments to complex functions and utilities
- **Code Organization**: Better structure with dedicated utility modules

## [0.7.3]

### Sonarr & Radarr Integration

- Connect to your Sonarr and Radarr servers to import your library
- Add multiple servers and test connections before saving
- Browse your Sonarr library and import TV shows directly into TVC
- Browse your Radarr library and import movies directly into TVC
- See which items are already tracked before importing
- Select multiple items at once for bulk import
- Import results show how many items were imported, skipped, or failed
- Access ARR settings from the sidebar footer

### Plex Scrobbler

- Automatically mark shows and movies as watched when you watch them in Plex
- Built-in webhook server receives playback events from Plex
- Configure the webhook port (default: 9876)
- View recent scrobbles to see what was automatically tracked
- Smart title matching to link Plex content with your tracked shows and movies
- See which scrobbles were successfully matched and which weren't
- Requires Plex Pass subscription for webhook support
- Access Plex settings from the sidebar footer

## [0.7.2]

### Drag & Drop Rating

- Drag shows and movies to rate them in the Tier view
- Drag items from the sidebar and drop them on any tier row to set their rating
- Drag posters between tiers to change their rating
- Drop items on the "Unrate" zone at the bottom to remove their rating
- Click and hold to start dragging, release to drop

## [0.7.1]

### Half-Star Ratings

- You can now give half-star ratings to shows and movies
- Click the left half of a star for a half-star rating (0.5, 1.5, 2.5, etc.)
- Click the right half of a star for a full-star rating (1, 2, 3, etc.)
- Click the same rating again to clear it

### Rankings & Tier List

- New "Rank" tab in the sidebar shows your rated shows and movies grouped by star rating
- See your average rating and count of rated items at a glance
- New "Tier" view in the header alongside Month, Week, and Agenda
- The Tier view displays a visual tier list with poster thumbnails organized by rating
- Click any poster to open its detail page
- Switch between Shows and Movies using sub-tabs in both views

## [0.7.0]

### Theme Settings

- You can now hide TV and movie posters in the sidebar if you prefer a cleaner look
- When you enable both compact spacing and hidden posters, the sidebar becomes a simple text list
- The "Hide Posters" option also works in the episode scheduler and show/movie picker
- Find the "Hide Posters" toggle in Theme Settings

### Better User Experience

- **Smoother Dialogs**: Confirmation dialogs now match the app's design and look much nicer
- **Faster Search**: Search now starts automatically as you type, so you don't need to press Enter. You'll also see how many results were found
- **Loading Indicators**: When the app is loading your shows and movies, you'll see helpful loading animations instead of blank screens
- **Helpful Empty States**: When you don't have any shows or movies yet, you'll see friendly messages with buttons to get started
- **Visual Feedback**: Buttons and interactive elements now have smooth animations and better visual feedback when you hover or click them
- **Calendar Improvements**: Hovering over calendar items shows more information, and everything feels more responsive
- **Error Messages**: If something goes wrong, you'll see clear error messages with a "Retry" button to try again

### Episode Scheduler

- The episode scheduler is now much bigger, making it easier to use when you're tracking lots of shows
- The show and movie picker is also larger, so you can see more options at once without scrolling as much

### Search Improvements

- Search modals are now much larger, giving you more room to browse results
- The search window automatically adjusts its size based on how many results you get
- More results means a bigger window (up to a reasonable limit), so you can see more at once

## [0.6.9]

### Calendar Changes

- Movies won't automatically show up on your calendar when you add them
- You need to manually schedule movies for them to appear on the calendar
- Digital release dates are still saved and shown in movie details, but they won't clutter your calendar

## [0.6.8]

### Data Management

- Before cleaning up episodes, you can now see exactly which ones will be deleted
- The preview shows the show name, season, episode number, and episode title
- You can preview both orphaned episodes and episodes that haven't aired yet

## [0.6.7]

### Bug Fixes

- Fixed update notifications not showing release notes properly
- Release notes now display with proper formatting (headers, lists, etc.)

## [0.6.6]

### Backup & Restore

- You can now export all your data (shows, episodes, movies) to a backup file
- Import from a backup file to restore your data if needed
- Find this in Data Management → Overview tab

## [0.6.5]

### Network Display

- TV shows now show which network they're on in the calendar (e.g., "Fallout | Prime Video")
- This appears in all calendar views (Month, Week, and Agenda)

### Sync Improvements

- Fixed the refresh button so it works every time you use it
- Added a "Sync All Shows" button in Data Management to update all your shows at once
- Syncing now properly updates network information

## [0.6.4]

### Update Notifications

- Update notifications now match the app's design
- Release notes display with proper formatting
- You can see the download progress with a percentage

## [0.6.3]

### Trailers

- Watch trailers for movies and TV shows right from the app
- Trailers open in your browser

### Cast & Crew

- See who's in your favorite TV shows
- See cast and crew for movies
- All in the new "Extra Info" tab when viewing show or movie details

### Statistics Dashboard

- Track your total watch time
- See how many episodes and movies you've watched
- Check completion rates for your shows
- View your watch history timeline

### Data Management

- Find and merge duplicate shows if you accidentally added the same show twice
- Clean up episodes that are no longer needed
- Optimize your database to keep things running smoothly
- See a history of all changes you've made

### Bug Fixes

- Fixed an issue with statistics not displaying correctly

## [0.6.1]

- Improved how you select episodes in the episode picker
- You can now unschedule movies from your calendar
- Better handling of scheduled movies

## [0.6.0]

- Added support for tracking movies
- Search for movies, add them to your list, and schedule them
- Mark movies as watched just like TV episodes

## [0.5.7]

- Mark entire seasons as watched with one click
- Mark entire shows as watched at once
- Improved interface for batch operations
