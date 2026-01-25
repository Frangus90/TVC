# Changelog

All notable changes to TVC will be documented in this file.

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
