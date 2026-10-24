# Library watch actions and consistent poster rows

## Scope

For 0.15.1, add an eye button to each title in the TV Shows and Movies library pages, in poster and list/hidden-poster views. Match the existing whole-show action: mark all saved episodes, including specials and future episodes already listed. Preserve existing watch dates, schedules and rankings. A show with no saved episodes must explain that episodes need syncing instead of claiming success.

Also align poster cards regardless of tier-name length: use a dedicated tier row and a separate action row below it, with consistent title/metadata/status space and bounded long labels. Keep the Focus look and existing list controls.

The follow-up also places the matching whole-title watch toggle beside **Remove** in both detail modals, including tier-only entries, while preserving season/episode controls and the ability to mark titles as unwatched. Library eye buttons also toggle both ways: fully watched titles become unwatched, and partially watched shows mark all saved episodes watched. The icon, hover text and success message name the next action or saved result.

Stay on `codex/focus-redesign`. Never access the installed release database or sidecars; use only development/disposable data. No migrations, app version changes or release publication.

## Plan

1. Expose saved watched status in the existing library records, reuse watch-history writes, and preserve truthful results and matching shared caches on success/failure.
2. Add accessible eye actions, busy feedback and saved-state feedback. Separate tier badges and action rows; keep same-width/height poster images and consistent row alignment.
3. Add regressions for failure handling, wrong-detail races, empty shows, retained watch dates, and record status. Check both libraries/views and long tier labels at 1080p/4K using isolated fixtures, update the changelog and run project checks.

## Validation

- Implemented watched/unwatched eye toggles in the library, a visible Watched badge and confirmed saved watched status for tracked and tier-only titles. Both detail modals expose the watched/unwatched toggle immediately beside Remove, outside tracking-only branches.
- Whole-show updates cover every saved episode, preserve existing timestamps and schedules, and reject empty episode sets with a useful message. A rejected save keeps the modal content and recovery instructions available. Pending whole-title saves block duplicate and conflicting watch/refresh/removal actions.
- Calendar, detail, episode-picker and library summaries update for the affected title. A late result for one show cannot change a different show's detail or picker. Movie updates include tier-only and archived caches.
- Poster titles reserve two lines; metadata/status/tier rows have consistent space, long tier names truncate with a full-text hover label, and action icons have their own aligned row. A minimum width in rem keeps all four movie actions inside small compact cards at larger text sizes. List actions stay vertically centred.
- Final `npm run check`: **0 Svelte errors/warnings; 98 frontend tests passed; 130 Rust tests passed; 1 live Wikipedia test ignored; production frontend build passed**.
- Nine new frontend regressions cover confirmed success, failed writes, cache consistency, preserved tiers/ranks, pending/different-title races and library/modal guards. Native memory tests cover watched summaries, empty shows, specials/future/unknown dates, timestamp preservation and isolation from other shows.
- Real compiled UI tested with isolated memory fixtures: keyboard eye action marks a tier-only movie without opening details; shows mark all saved episodes; whole-title modal toggles mark/unmark both seasons; tier-only show action works in list view; movie action works with posters hidden. Both modal watch controls are immediately before Remove.
- Both libraries checked at **1920×1080** and **3840×2160**, including **Small posters + Compact spacing + 150% text + colourblind colours**, long names, S tiers, long custom tiers, no tiers and missing posters. Measured action-row and card-height differences within each grid row were **0 px**; tier-row differences were below **0.04 px** (layout rounding). No icon or document horizontal overflow remained.
- Empty-show failure stays unwatched and leaves its detail modal intact with the existing matching/tracking instructions. The preview console recorded only this deliberately exercised save failure; no unexpected errors/warnings.
- Local ignored proofs: `focus-redesign-screenshots/library-watched-aligned-movies.jpg`, `show-detail-whole-title-watched.jpg`, and `movie-detail-whole-title-watched.jpg`.
- Independent store/backend/modal review and `git diff --check` passed. Only in-memory/disposable test data was used. No installed release database, sidecars, released migration, app version or recovery backup was accessed or changed.

## Eye toggle follow-up

- Library eye controls now toggle watched and unwatched in poster, list and hidden-poster views, matching the detail windows. Watched titles stay clickable; the crossed-out eye and hover text offer **Mark as unwatched**. Partially watched shows still mark all saved episodes watched.
- Captured media type and intended watched state stay fixed during a pending save. Existing busy guards, truthful failure feedback, saved episode scope and cache updates remain in place.
- `npm run check` passed: **0 Svelte errors/warnings; 100 frontend tests; 130 Rust tests; 1 live Wikipedia test ignored; production frontend build passed**. The isolated handler suite passed **11/11**, including fully watched tracked/tier-only shows and movies, partial shows, failures and overlapping clicks.
- Compiled UI with memory fixtures confirmed a tier-only movie toggles watched → unwatched in poster view and a whole tier-only show toggles back to unwatched in list view. The icon, hover label, badge and success feedback follow the saved state. Proof: `focus-redesign-screenshots/library-eye-toggle.jpg`. The installed release database was never accessed.

## Limits and release handoff

Shows require saved episode information before there is anything to mark. This action does not fetch episodes or start tracking a title automatically. All saved episodes, including future dates, are covered to match the existing whole-show behaviour; season and episode controls remain available for narrower changes.

UI execution used memory fixtures; native database commands were tested separately against disposable/in-memory pools. No live TMDB requests, installed app, installer or release script was run. These additions remain on the Focus branch for **0.15.1 Unreleased**.
