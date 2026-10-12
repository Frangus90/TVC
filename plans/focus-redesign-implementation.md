# Focus redesign implementation

## Intent

Implement the approved Focus design across TVC for its owner and friend, preserving existing workflows, saved data, preferences, integrations, and exports.

## Recovery checkpoint

- Original app: `ce0b9f201f6be39f1fc434e828c6bcbb774bbb11` (main, v0.14.8).
- Annotated backup tag: `codex/pre-focus-2026-10-12`, pushed to origin.
- Independent repository backup: `plans/tvc-pre-focus-2026-10-12.bundle` (main and backup tag with complete history).
- Working branch: `codex/focus-redesign`.
- Recovery: switch to main or the backup tag in a clean checkout; the bundle can be cloned into a new directory. Do not overwrite a working checkout with uncommitted work.
- This backup contains source/history only. Installed release data must never be accessed, copied, backed up, or modified.

## Constraints

- Development database `tvc_dev.db` or disposable test fixtures only.
- No migration or backend rewrite is expected for presentation changes.
- Preserve date helpers (DD.MM.YYYY and HH:mm), accent/font/compact/hide-poster/colourblind preferences, lazy loading, update checks, tray and notification behaviour.
- Preserve calendar scheduling/watch actions; library bulk/archive/unschedule actions; custom and untiered tiers and ordering; awards/prediction history; racing configuration; sync schedules/retry; backup/cleanup; Plex and Arr integrations.
- Only CHANGELOG.md is changed manually for the next version. Release script owns all other versions.

## Stages

1. Shared styling and app structure: Focus theme, labeled navigation, top bar, page heading patterns, separate Calendar and library pages. Validate Svelte diagnostics/build and checkpoint.
2. Libraries and details: poster cards, search, existing bulk/archive/remove/scheduling actions; cohesive detail and search dialogs. Validate interactions and checkpoint.
3. Supporting pages: Settings, Data, Statistics, Calendar modes, tiers, awards, racing; preserve actions and lifecycle. Validate regression tests and checkpoint.
4. Final polish: notifications, image exports, keyboard/focus, scaling and layout checks at 1080p, 4K, and scaled display sizes. Run full npm check and record results.

## Validation targets

- Frontend DB helper uses import.meta.env.DEV -> sqlite:tvc_dev.db.
- Debug Rust app identity/database uses the isolated dev profile.
- Rust audit/backup tests use sqlite::memory:; frontend regression scripts use mocked stores or disposable filesystem fixtures.
- No installed app launch until the target has been verified.

## Progress

- Research, recovery checkpoint, implementation, and validation complete on 12.10.2026.
- Stage 1: `019af75` — Focus theme, navigation, Calendar/library separation, and full-page Settings/Data/Statistics.
- Stage 2: `6590d85` — Library/detail/search workflows, dialogs, archive coverage, and shared-cache regression fixes.
- Stage 3: `dc4fd40` — Calendar modes, tiers, awards, racing, exports, and appearance preferences.
- Stage 4: final polish checkpoint — keyboard/modal focus, notification compatibility, wide-screen poster sizing, aligned headings, readable changelog, and this validation record.

## Result

- All nine main destinations use the Focus shell: Calendar, TV shows, Movies, Tier lists, Racing, Awards, Statistics, Data, and Settings.
- Libraries combine tracked and tier-only titles, support poster/list modes, filtering, saved-title search, and bulk archive/remove actions. Archived records stay in Data > Archive.
- Ctrl K searches saved titles, including tier-only titles, and respects hidden posters. It does not interrupt an active modal.
- Calendar titles open details; watch status has a separate explicit control. Month, Week, Agenda, day details, scheduling pickers, and right-click actions remain available.
- Existing custom tiers, year searches, prediction history/results, racing filters/configuration, sync schedules/retries, backups, and integration controls remain in place.
- Existing saved accent colours, font size, compact mode, hidden posters, and colourblind settings take precedence over the new defaults.
- Update, release-notes, migration, search, and detail dialogs participate in focus management. Migration progress and update installation remain non-dismissible.
- Existing version files and released migrations are unchanged. CHANGELOG.md contains a human-readable 0.15.0 Unreleased entry; the release script still owns version changes.

## Regression fixes found during implementation

- Restored startup loading for racing configuration after removing the old sidebar's library responsibilities.
- Refresh tier-list caches after title removal and legacy show remapping/deletion, so libraries and global search do not show stale records.
- Include archived tier-only titles in archive queries, preserving their tier-only state and custom tiers on restore. This changes two SELECT filters and requires no migration.
- Group racing sessions by their local calendar day using the shared date helper.
- Hide irrelevant period navigation in Agenda, whose existing range is today through the upcoming 60 days.
- Prevent Ctrl K from layering search above an unresolved confirmation; coordinate automatic notices with blocking dialogs.

## Validation results

- `npm run check`: passed — Svelte 0 errors / 0 warnings, 25 frontend tests passed, 84 Rust tests passed / 1 live Wikipedia test ignored, production build passed.
- Re-ran frontend tests and production build after final modal and CSS polish: passed. Final Svelte diagnostics: 0 errors / 0 warnings.
- Added three frontend regressions covering successful/failed tier-only removal and cache refresh after legacy remapping, deletion, and completed migration.
- `git diff --check`: passed. Git bundle verification: complete history, valid backup.
- `cargo fmt --check` reports existing formatting differences across untouched Rust files. The two changed SQL filter lines follow their surrounding style; no unrelated Rust reformat was applied.

### Visual and interaction checks

Used the real App.svelte and lazy-loaded components with isolated in-memory Tauri/SQL/network fixtures. The preview accepts only the mocked dev DB name; it opens no database files, contacts no TMDB/Plex/Arr services, and cancels native file dialogs.

- Viewports: 1920 x 1080, 3840 x 2160, and 2560 x 1440 (scaled 4K). No document/main horizontal overflow in the measured library layouts. Wide-screen posters receive more room.
- Font size 150%, compact spacing, and hidden posters: controls remain usable; hidden posters also apply to global search.
- Verified library filters, show/movie details, bulk-selection confirmation/cancellation, title search, Ctrl K modal isolation, and Escape focus restoration.
- Verified calendar Month/Week/Agenda, opening movie details without changing watch status, separate watched toggle, and episode scheduling picker.
- Verified multi-year tier highlighting, custom tier labels, untiered items, and moving a title through its tier menu. Existing drag/drop handlers are retained and reviewed; pointer dragging was not exercised in this browser pass.
- Verified awards prediction history and correct/wrong/pending states, racing filters and configuration navigation, statistics Overview/Watch History, all Settings sections, and Data Sync/Archive.
- Verified sync schedule changes/retries and restoring an archived tier-only title with its custom tier intact in disposable fixtures.
- Browser console had no warnings/errors after these interactions.
- Local proof images: `plans/focus-redesign-screenshots/focus-movies-1080p.jpg`, `focus-movies-4k.jpg`, and `focus-calendar-1080p.jpg`.

## Practical limits and next manual check

- The installed release app and its database were never launched or accessed. No real library data was changed or backed up.
- Native file/export dialogs, updater installation, live TMDB sync, live Plex/Arr connections, and native tray behaviour were not exercised in the browser preview. Their existing handlers remain and the build/regression checks pass.
- Test the branch with the normal dev app and its own `tvc_dev.db` before merging/releasing. Expected external failure cases remain unavailable services, missing credentials, or incomplete metadata; existing error handling applies.
- This task creates no release installer and does not merge the redesign into main.
