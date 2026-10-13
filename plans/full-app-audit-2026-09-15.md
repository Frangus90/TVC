# Full App Audit — Bugs, Issues, and Feature Opportunities

**Date:** 15.09.2026

**Baseline:** TVC 0.14.4, main, commit 23cb8ea

**Last updated:** 13.10.2026

**Status:** Original concrete audit fixes are implemented. The Data follow-up below is implemented on `codex/focus-redesign`; other optional ideas remain a wishlist. Automated and isolated-browser checks passed. Native desktop/live-integration validation remains separate.

**Usage context (owner clarification):** TVC is primarily a personal desktop app for the owner and one friend. The repository is on GitHub, but no wider user base is known. The priorities below reflect that scope.

## Summary

### Focus Data follow-up — 13.10.2026

Implemented on `codex/focus-redesign`, with correctness fixes saved before the presentation/features stage. Main and the original recovery backup remain unchanged.

- Cleanup now protects personal/legacy records, uses consistent counts/previews, confirms removal, and distinguishes a committed deletion from a failed optimisation or display refresh. Matching ignores stale searches; duplicate/restore operations refresh persistent library and calendar views.
- TMDB status requires both libraries for whole-library success. Sync preserves the original full-run summary through retries, explains skipped titles, retains five recent runs, and can stop after the current title without starting the next library.
- Automatic local backups are implemented and default Off, with five managed files retained after a successful new backup. Manual/unrelated/other-profile files remain untouched. Optional v4 portable settings transfer Appearance, General/calendar, Notifications/quiet hours, tier preset and Racing choices; older formats keep existing preferences. Credentials, computer-specific paths, startup registration and connection/feed configuration remain excluded.
- Overview, Sync, Backups, Archive and Maintenance have their own tabs. Archive adds title/year filtering, metadata, details and restore progress. Advanced tools remain collapsed.
- Validation: 75 frontend tests, 119 Rust tests passed, 0 Svelte errors/warnings, successful build; isolated 1080p/4K UI checks include 150% app text. No installed database was accessed. Native dialogs and live services remain separate manual checks.

See [data-page-improvements.md](C:/Users/aleks/Desktop/Project/TVC/plans/data-page-improvements.md) for scope and safeguards. Earlier dated status/evidence below describes the historical implementation.

**Additional review — 17.09.2026:** The [visual UI/UX audit](C:/Users/aleks/Desktop/Project/TVC/plans/ui-ux-audit-2026-09-17.md) records ten additional open findings and six optional improvements, with screenshots and interaction evidence. It covers layout clipping, modal focus, sidebar removal confirmation, text scaling, statistics failure states, and everyday usability. Completion statements in this technical audit refer to its original scope; they do not mean the application has no remaining UI issues.

The follow-ups now address F01–F18 and the concrete issues in section 2. They protect saved data, separate the development profile, correct calendar/watch history and Plex matching, make racing and awards failures visible, and add a local release check. Backup v3.0 includes predictions, title corrections, and personal history. See the validation section for tested behavior and remaining native/live-service checks.

The original audit recorded **18 primary findings: five P1, twelve P2, and one P3**, followed by smaller issues, engineering improvements, and feature proposals. Priority indicates practical urgency for these two users, not just theoretical impact. Original evidence, code line references, and reproductions describe the baseline above and may no longer match the edited source; dated status notes record subsequent fixes. The recommended response is sized to a small personal project.

- **P1:** Protect saved data, maintain dev/release isolation, or prevent crashes in a feature being used.
- **P2:** Correct broken workflows, misleading results, and integration reliability.
- **P3:** Optional hardening, maintainability, or lower-impact gaps; address when convenient or when exposure changes.
- **Reproduced:** Demonstrated with isolated code, mocked requests, or disposable SQLite data. This does not mean the entire desktop workflow was exercised.
- **Source-confirmed:** The relevant execution path is present in current code; the live desktop or external-service scenario was not run.
- **Needs validation:** A targeted follow-up is needed before treating the proposed user-facing outcome as established.

### How the two-person scope changes the recommendations

- Data loss, incorrect picks, wrong watch dates, crashes, and dev/release mixing remain important even with one user. Prioritize preserving the data both people have already entered.
- Fix integration-specific issues according to actual use. The Plex findings matter when Plex scrobbling is enabled; racing work can wait if neither person uses it. Usage has not been assumed from the presence of a feature.
- F06 moves from P1 to P3: exploiting the release-note issue requires control over the owner's update feed. Escaping the text is still a sensible small fix; broader CSP hardening can wait behind everyday bugs.
- A local check command, targeted regression tests, and a short manual dev smoke pass are sufficient starting points. CI, a broad desktop automation suite, and extensive historical-version support are optional investments.
- Feature ideas are a menu for the owner and friend, not a commitment to build every item. Choose one that improves their routine and keep the first version small.
- GitHub publication alone does not establish a public-service threat model. Revisit security and operational priorities if distribution grows, untrusted imports become common, or network services are exposed beyond localhost.

### Sync follow-up — 16.09.2026 (historical status)

| Item | Status on 16.09.2026 | Scope at that point |
| --- | --- | --- |
| F01 — Dev/release isolation | Partially resolved | SQL preload now uses the build-specific database. Shared app identity/single-instance behavior and an isolated desktop startup/coexistence check remain open. |
| F02 — Failed season deletes saved state | Resolved for the reported failure path | Failed or incomplete season downloads abort before replacement; episode-state snapshots and replacement share a transaction. Regression tests pass. General reconciliation of episodes removed by the provider is not implemented. |
| F08 — Automatic versus custom dates | Open; related case improved | An explicitly cleared schedule now stays cleared on refresh. New episodes still copy the air date into scheduled_date, and legacy automatic dates remain indistinguishable from overrides. |
| F17 — Hidden sync failures | Partially resolved | TV/movie bulk sync now saves counts, per-title errors, progress, and retry results. Racing/awards reporting and separate aggregate last-success/freshness tracking remain open. |
| Sync results/retry and automatic schedules | Implemented for TMDB libraries | Data Management > Sync includes independent Off/daily/weekly/monthly schedules, Sync All, per-library refresh, latest results, and retry failed. No multi-run history or cross-source dashboard. |

Schedules default to Off, run while TVC is open (including in the tray), and catch up after reopening. Movie refresh also bypasses its metadata cache. See [library-sync.md](C:/Users/aleks/Desktop/Project/TVC/plans/library-sync.md) for implementation scope and validation.

### Backup/restore follow-up — 17.09.2026

`git push origin main` confirmed GitHub already contained recovery commit `cf70820` (v0.14.5) before these edits. Work continues on `codex/audit-data-reliability`.

- **F03 resolved for database replacement:** Supported versions, unique IDs, and relationships are checked; every failed insert aborts; foreign-key verification runs before commit. Backup export uses one read transaction. Restore and bulk TMDB sync share a lock.
- **F04 resolved for the documented personal-data scope:** v3.0 includes library/watch state, tiers/order, ceremony/category/nominee source snapshots and predictions, Plex title corrections, scrobble logs, and change history. The dialog previews counts and explains omissions for older backups.
- **Explicit limits:** Preferences, credentials, racing/notification data, integration import links, and cast/crew caches are excluded. Existing preferences/credentials/racing/notifications remain; links/caches are reset. Legacy v1.0 provider remapping and post-commit metadata refresh still operate separately and can report unmatched episodes. This work does not establish atomicity of that network migration.

See [audit-data-reliability.md](C:/Users/aleks/Desktop/Project/TVC/plans/audit-data-reliability.md) and the README's Backup and Restore section for scope and validation.

### Remaining fixes implemented — 17.09.2026

All concrete fixes from this report are implemented. Optional product ideas in section 4 remain a wishlist.

| Scope | Result |
| --- | --- |
| F01 | Debug uses `com.tvc.app.dev`, a separate data directory and **TVC — Development** window title. A consistent SQLite snapshot carries over only an existing legacy `tvc_dev.db`, without overwriting the new profile. |
| F02 | Complete refreshes retain provider-removed episodes with watch dates, watched state, schedules, ratings, or tags. Failed/truncated refreshes still roll back. |
| F05–F07 / F11 | Structured Unicode-safe Plex parsing, unique identity matching, idempotent watched dates, and escaped release notes. |
| F08–F10 | New episodes follow air dates; explicit overrides remain; range queries use the visible date and exclude archived shows. Migration 019 clears import links on title deletion. Duplicate merges preserve metadata/references transactionally. |
| F12–F14 / F18 | Validated transactional racing feeds, stable event IDs, settings-aware reminders, local-day/DST boundaries, and HTTP connect/total deadlines. |
| F15–F17 | Latest-request guards, visible failed writes, and persisted TMDB/racing/awards reports with targeted retries and distinct attempt/complete-success timestamps. |
| Additional issues | Backend prediction validation, canonical dates, complete final-day statistics, modal focus/keyboard handling, a Svelte render boundary, and accurate notification controls. |
| Local engineering | Rust numbered migrations own the schema; only equivalent LF/CRLF checksum repair is allowed; `npm run check` runs Svelte diagnostics, frontend and Rust tests, and the production frontend build before releases. Compatible npm dependency updates remove the reported advisories. |

**Deliberate limits:** Existing non-null episode schedules cannot be reliably classified as automatic versus custom, so they remain unchanged. Clearing an override restores automatic air-date behavior. Backups retain the documented exclusions and separate legacy network migration. Empty/invalid racing feeds preserve cached events and report an error. Last complete success advances only after a complete normal sync, not after retrying a subset. Native OS reminders, side-by-side installed apps, and live providers were not exercised.

See [remaining-audit-fixes.md](C:/Users/aleks/Desktop/Project/TVC/plans/remaining-audit-fixes.md) for the completion checklist.

### Scope and safety

Reviewed the frontend stores and major views, Rust commands, SQLite migrations and connection setup, TMDB, Sonarr/Radarr, Plex, racing feeds and reminders, awards, notifications, settings, updates, and release tooling.

**No installed development or release database was opened, copied, inspected, or modified during this work.** SQL tests used in-memory databases, with one dev-profile carryover test using a disposable temporary directory containing only a test `tvc_dev.db`. Browser previews mocked IPC/SQL/file responses. Tauri startup and live integrations were not exercised.

The original audit only added this report. The 2026-09-16 follow-up changed application source and SQL preload configuration; it did not edit released migrations, versions, or the changelog. Frontend builds generated their normal ignored output.

This is a broad source audit with targeted reproductions, not an exhaustive GUI, accessibility, load, security penetration, or live-service test. There was no release installation, update installation, live Plex webhook, live Sonarr/Radarr import, or live Wikipedia/TMDB request.

## 1. Primary findings

| ID | Priority | Area | Finding | Original evidence | Current status |
| --- | --- | --- | --- | --- | --- |
| F01 | P1 | Environment | Dev startup preloads the release database | Source-confirmed | Implemented; native coexistence smoke test pending |
| F02 | P1 | TV sync | A failed season fetch can erase saved episode state | Source + isolated reproduction | Resolved; unmatched episodes with personal state retained |
| F03 | P1 | Restore | Restore can commit partial data and accept unsupported versions | Source + isolated reproduction | Resolved for database replacement; legacy network migration remains separate |
| F04 | P1 | Backup | Backup omits predictions and other user state | Source-confirmed | Resolved for documented v3.0 personal-data scope; exclusions explicit |
| F05 | P1 | Plex | UTF-8 slicing can crash the release app | Isolated Rust reproduction | Resolved |
| F06 | P3 | Updates | Remote release notes reach an HTML sink without escaping | Parser reproduction + source; requires feed control | Resolved; CSP redesign optional |
| F07 | P2 | Watch history | Bulk watched actions overwrite previous watch dates | SQL reproduction | Resolved |
| F08 | P2 | Calendar | Automatic air dates become persistent manual schedules | Source + isolated reproduction | Resolved for automatic dates; ambiguous legacy overrides preserved |
| F09 | P2 | Archive | Archived TV shows return to the calendar after reload | Actual query reproduced | Resolved |
| F10 | P2 | Library | Imported shows and movies cannot be deleted normally | Schema/query reproduction | Resolved with new migration 019 |
| F11 | P2 | Plex | Matching can mark the wrong title as watched | Query reproduction + source | Resolved |
| F12 | P2 | Racing reminders | Pending reminders do not consistently follow settings | Source-confirmed | Implemented and eligibility-tested; native delivery unverified |
| F13 | P2 | Racing refresh | Invalid feed content can replace valid cached events | Source-confirmed | Resolved |
| F14 | P2 | Racing calendar | Range boundaries and UTC grouping produce missing/wrong-day events | Source + date reproduction | Resolved |
| F15 | P2 | UI state | Older requests can overwrite the current selection | Mocked store reproduction | Resolved in audited request paths |
| F16 | P2 | UI errors | Failed additions are presented as successful | Mocked store reproduction + source | Resolved in audited mutation paths |
| F17 | P2 | Sync status | Partial/total failures are reported as ordinary completed syncs | Source-confirmed | Resolved for latest reports/retries and complete-sync freshness |
| F18 | P2 | Networking | Racing and awards requests have no application timeout | Source + installed dependency verification | Resolved |

### F01 — Dev startup preloads the release database

**Status (17.09.2026):** Implemented. Debug identity, SQL preload, migration-repair path, and development window title are now separate. A disposable WAL-mode copy test verifies consistent carryover of only the old dev database and no overwrite of an existing new profile. Native debug/release coexistence has not been run. Original baseline evidence follows.

**Original trigger:** Start the debug app using the audit-baseline configuration.

**Original evidence:** `tauri.conf.json` preloaded sqlite:tvc.db unconditionally, while `db/mod.rs` selected tvc_dev.db for debug builds and `lib.rs` registered migrations under that debug connection string.

The installed tauri-plugin-sql 2.3.1 implementation connects every configured preload during setup. Its connection wrapper can also create a missing database. Registering migrations for the dev filename does not redirect the separate release preload.

**Impact:** Debug startup can open the release database despite the explicit separation rule. This audit does not claim that a debug migration actually modified release data. It establishes that the configured connection crosses the boundary.

The shared com.tvc.app identifier also gives debug and release the same Windows single-instance mutex; the installed plugin derives the mutex from that identifier. That interferes with running both versions side by side.

**Fix:** Use a coherent dev configuration for the preload, app identity, and database paths, or remove the unconditional preload and initialize only the selected database. Audit the hardcoded identifier in migration repair when changing identity.

**Acceptance:** In an isolated test profile containing disposable sentinel databases, debug startup opens only its dev target and applies all migrations there. A separate release test instance can coexist. Never verify this by opening the user's real release database.

**Effort:** Small–medium.

### F02 — A partial TMDB episode fetch can delete watch history and schedules

**Status (17.09.2026):** Resolved. Failed/incomplete downloads abort before writes; refresh is transactional and preserves per-episode personal state. Unmatched episodes removed from an otherwise complete provider response now remain when they contain watched/date/rating/tag/schedule state. Regression tests cover preservation and rollback. Original baseline evidence follows.

**Original trigger:** Refresh a show with multiple seasons when one season request fails, for example because of a timeout, 429, or 5xx response.

**Original evidence:** `get_tv_episodes` logged failed season requests and returned the successfully fetched subset as an ordinary successful result. `sync_show_episodes` then deleted every existing episode for that show and reinserted only that subset.

The saved-state map is only consulted for returned episodes. State for an omitted season is never reinserted. A subsequent successful refresh cannot recover the deleted watched flags, watch dates, ratings, tags, or custom schedules.

**Original reproduction:** Seed two seasons in an in-memory database, snapshot their state, and execute the baseline replacement strategy with only season one in the fetched result. The episode count drops from two to one; season two's saved state disappears.

**Fix:** Make incomplete fetching explicit. Either fail the refresh before any writes or update only seasons known to have fetched successfully. Retain unmatched episodes with user state until a complete response and a deliberate reconciliation policy are available. Capture and restore state inside a consistent transaction.

**Acceptance:** Inject failure for one season and assert all existing episode state remains intact. Verify that the error reaches the UI and that a retry updates metadata without resetting state.

**Effort:** Medium. This also matters for callers involved in legacy TVDB migration.

### F03 — Restore can commit partial data and misinterpret unsupported versions

**Status (17.09.2026): Resolved for database replacement.** `backup.rs` validates versions and relationships before replacement, rejects all failed rows/collisions, and checks foreign keys before committing. Tests exercise the production export/restore functions with malformed inputs and injected failures after deletion. v1.0 still has a separate provider migration/refresh after the transaction; that live-network phase is not claimed to be atomic or validated here. Original evidence follows.

**Trigger:** Import a version 2.0 backup with a duplicate show/episode ID or another insert error.

**Evidence:** [backup.rs:531](C:/Users/aleks/Desktop/Project/TVC/src-tauri/src/commands/backup.rs:531) and [backup.rs:590](C:/Users/aleks/Desktop/Project/TVC/src-tauri/src/commands/backup.rs:590) collect show/episode insertion errors and continue. The transaction still commits. The returned ImportResult does not include those errors; the event carrying them is emitted only for legacy imports. [DataManagement.svelte:173](C:/Users/aleks/Desktop/Project/TVC/src/lib/components/DataManagement.svelte:173) displays an imported-count message.

The old library has already been cleared in the same transaction. A transaction alone does not make restore atomic if errors are intentionally swallowed.

A second validation gap is [backup.rs:333](C:/Users/aleks/Desktop/Project/TVC/src-tauri/src/commands/backup.rs:333): every version other than 2.0 is treated as TVDB-based legacy data. A future version or typo can therefore start inappropriate ID remapping instead of being rejected.

**Reproduction:** With the current delete/insert/continue strategy, a duplicate-ID insert fails, the remaining row commits, and the old show is absent. This used disposable SQLite data; the actual import command was not invoked.

**Fix:** Validate supported versions, uniqueness, references, and counts before replacement. Roll back a normal restore on any failed row. If partial recovery is offered separately, show a preview and return explicit skipped/error details.

**Acceptance:** Malformed and unknown-version backups leave the original disposable database unchanged. A valid export/restore round trip preserves every included record and relationship.

**Effort:** Medium.

### F04 — Backup does not cover all user data

**Status (17.09.2026): Resolved for the first useful scope described below.** v3.0 preserves the existing library/tier state plus awards/prediction snapshots, Plex corrections, scrobbles, and change history. A serialized export/import/export round trip against fresh in-memory databases compares all included fields and IDs, including Unicode and historical records referring to deleted titles. Credentials, preferences, racing/notification state, caches, and integration import links remain explicit exclusions in the README and import UI. Older backup omissions are shown before confirmation. Original evidence follows.

**Evidence:** [BackupData](C:/Users/aleks/Desktop/Project/TVC/src-tauri/src/commands/backup.rs:16) and [export_database](C:/Users/aleks/Desktop/Project/TVC/src-tauri/src/commands/backup.rs:165) cover shows, episodes, movies, and tiers. They do not include awards/predictions, settings, racing preferences, integration configuration, or Plex title mappings.

Restore also deletes title mappings, integration import mappings, scrobble logs, and change history without exporting/restoring those tables. The UI describes the operation as replacing all current data.

**Impact:** A backup used on a fresh installation cannot recover predictions or much of the app configuration. Restoring on an existing installation can erase title corrections and history while leaving unrelated existing settings/awards in place. The result is neither a full snapshot nor a clearly scoped library-only restore.

This is particularly relevant to the user's saved Emmy predictions: the PNG export is useful evidence, but it is not a restorable structured backup.

**Fix:** Define and label backup scope. First preserve irreplaceable user-owned state: predictions, watch history, custom schedules, ratings, notes, tier ordering, and title corrections. A versioned JSON backup is sufficient. Rebuild caches separately. Preserve source identities and prediction snapshots so old picks remain interpretable after upstream edits. Exporting every preference or integration credential is a later convenience; it is not required for the first useful fix. Clearly state any exclusions.

**Acceptance:** Restore into an empty disposable database and compare all user data included in the documented scope, including watch history, custom schedules, tier ordering, predictions, and title corrections. Confirm excluded preferences or credentials are clearly identified. Add round-trip checks for those settings if their export is implemented later.

**Effort:** Medium–large.

### F05 — A normal Unicode Plex payload can crash the release app

**Status (17.09.2026):** Resolved. The unsafe preview and byte-to-string slicing are removed. Multipart parsing uses structured byte fields and safely ignores binary thumbnail content. Unicode/malformed-payload tests pass. Original baseline evidence follows.

**Evidence:** [handler.rs:35](C:/Users/aleks/Desktop/Project/TVC/src-tauri/src/plex/handler.rs:35) builds a lossy UTF-8 string and slices it at byte 200. Rust strings can only be sliced at character boundaries. A similar unsafe context slice exists at [handler.rs:192](C:/Users/aleks/Desktop/Project/TVC/src-tauri/src/plex/handler.rs:192).

**Reproduction:** A string containing 199 ASCII bytes followed by an accented character and more text panics when evaluated with the same preview slice. The preview runs before payload validation and is no longer used for logging.

**Impact:** Legitimate international metadata or multipart content can hit the boundary. [Cargo.toml:42](C:/Users/aleks/Desktop/Project/TVC/src-tauri/Cargo.toml:42) sets release panic behavior to abort, so this is an application termination risk, not just a rejected webhook.

**Fix:** Remove the unused preview. Use character-safe extraction for any remaining diagnostic context and parse multipart content as structured bytes.

**Acceptance:** Valid Unicode payloads and malformed payloads with Unicode at boundary offsets never panic. Assert the handler returns an appropriate response.

**Effort:** Small.

### F06 — Release-note text is rendered as unescaped HTML

**Status (17.09.2026):** Resolved. Both release-note HTML sinks use shared escaping before limited bold/code formatting. Tests confirm HTML/event-handler input stays inert. A broader CSP redesign remains optional. Original baseline evidence follows.

**Adjusted priority: P3 for the current two-person use.** The baseline exploit required feed control. Escaping is now implemented; a broader security overhaul remains optional for this scope.

**Evidence:** [UpdateModal.svelte:14](C:/Users/aleks/Desktop/Project/TVC/src/lib/components/UpdateModal.svelte:14) interpolates input into generated HTML without escaping it, then renders the result through [the HTML sink](C:/Users/aleks/Desktop/Project/TVC/src/lib/components/UpdateModal.svelte:130). The input comes from the remote updater body.

**Reproduction:** Executing the current parser with a harmless HTML marker preserves that tag in its output.

**Impact and boundary:** Malicious feed content could become active markup in the app's privileged webview. This requires influence over the update feed; ordinary film/show metadata does not use this path. The configured script policy allows inline scripts, and the main window has SQL execution capabilities. Arbitrary unrestricted filesystem access was not established by this audit.

**Fix:** For the current limited formatting, escape source text before formatting and apply the same rule to bundled What's New content. Introduce a sanitizing parser only if richer HTML is actually needed. Treat production CSP tightening as a separate optional follow-up with compatibility checks.

**Acceptance:** HTML and event-handler payloads render inertly; supported headings, bullets, bold text, and code still work. Verify in a disposable app environment after F01.

**Effort:** Small–medium.

### F07 — Marking a season/show watched rewrites earlier viewing dates

**Status (17.09.2026):** Resolved. Manual, bulk, movie, and Plex watched writes share idempotent timestamp rules: retain an existing watched date, fill it on first watch, and clear it when unwatched. Regression tests cover repeated and partially watched batches. Original baseline evidence follows.

**Evidence:** [mark_season_watched](C:/Users/aleks/Desktop/Project/TVC/src-tauri/src/commands/episodes.rs:361) and [mark_show_watched](C:/Users/aleks/Desktop/Project/TVC/src-tauri/src/commands/episodes.rs:407) assign the current timestamp to all matching episodes, including already watched episodes. Plex's mark-watched helpers do the same on repeated scrobbles.

**Reproduction:** An episode watched on 2026-07-01 receives the audit execution time after the bulk update.

**Impact:** Finishing a season makes earlier episodes appear newly watched. History ordering and time-based statistics become inaccurate.

**Fix:** Preserve an existing watch date when setting an already watched item to watched. Treat a rewatch as an explicit separate event if that feature is added.

**Acceptance:** Bulk-completing a partly watched season changes only previously unwatched episodes. Duplicate webhook delivery is idempotent.

**Effort:** Small.

### F08 — Automatic air dates are stored as custom schedules

**Status (17.09.2026):** Resolved for new/automatic dates. New episodes store no custom override and follow their air date. Existing non-null schedules are preserved because intent cannot be recovered reliably. Clearing an override uses the air date again; the bulk action now says Use air dates. Effective-date range and refresh tests pass. Original baseline evidence follows.

**Original evidence:** [episodes.rs:216](C:/Users/aleks/Desktop/Project/TVC/src-tauri/src/commands/episodes.rs:216) preserves the existing scheduled date for known episodes and copies the provider air date for newly inserted episodes. On later refreshes, that automatically filled value is still treated as a user override. Calendar views prefer scheduled_date over aired.

**Example:** Initial sync stores both dates as September 15. TMDB moves the episode to September 17. Refresh updates aired but retains scheduled_date as September 15, so the calendar stays on the old day.

**Reproduction:** The current fallback rule retained September 15 against a new September 17 air date.

**Fix:** Separate an explicit user schedule from the automatic release date. Use the air date only at display/query time for entries without a custom schedule. Treat existing rows carefully because old automatic values and intentional overrides are not reliably distinguishable.

**Acceptance:** Provider date changes move automatically scheduled entries; genuine custom dates survive refresh. Test an explicitly cleared override as well.

**Effort:** Medium, including legacy-data handling.

### F09 — Archived TV shows reappear in calendar queries

**Status (17.09.2026):** Resolved. Both backend and frontend fallback calendar queries exclude archived and tier-only shows. Disposable tests verify archive/unarchive and cross-month custom dates. Original baseline evidence follows.

**Evidence:** [get_episodes_for_range](C:/Users/aleks/Desktop/Project/TVC/src-tauri/src/commands/episodes.rs:103) filters tier-only shows but not archived shows. The archive store removes episodes from local state immediately, but a later reload queries them again. Movie calendar queries correctly filter archived movies.

**Reproduction:** The actual episode-range SQL returned two episodes belonging to an archived show in an in-memory database. Month/week rendering does not apply another archive filter.

**Fix:** Apply the archive filter consistently in the backend and the frontend SQL fallback.

**Acceptance:** Archive a show, navigate away and back, and reload the app in a disposable environment. Its episodes remain hidden; unarchive restores them.

**Effort:** Small.

### F10 — Normal removal fails for Sonarr/Radarr-imported titles

**Status (17.09.2026):** Resolved. New migration 019 removes import links and title mappings before show/movie deletion. Duplicate merges validate both IDs and preserve ratings, unioned tags, notes, schedules, watched state, and dependent mappings/history in one transaction. Tests cover normal deletion, merge preservation, and rollback after a final-delete failure. Original baseline evidence follows.

**Evidence:** [010_add_arr_integration.sql](C:/Users/aleks/Desktop/Project/TVC/src-tauri/migrations/010_add_arr_integration.sql:16) defines import references without delete cascading on show/movie IDs. [remove_show](C:/Users/aleks/Desktop/Project/TVC/src-tauri/src/commands/shows.rs:67) and [remove_movie](C:/Users/aleks/Desktop/Project/TVC/src-tauri/src/commands/movies.rs:131) delete the parent directly.

**Reproduction:** With all 18 migrations applied and foreign keys enabled, deleting a show referenced by sonarr_imports fails with a foreign-key constraint error. SQLx enables foreign-key enforcement by default. Radarr uses the same relationship pattern.

**Impact:** Imported items cannot be removed through the normal path. The same obstacle can affect duplicate merging. Error swallowing in the stores makes this difficult to diagnose.

**Fix:** Remove or transfer dependent integration mappings within the same transaction, or introduce the intended cascade behavior in a new migration. Handle title mappings and associated history deliberately.

**Acceptance:** Add disposable Sonarr and Radarr import records, then remove their titles successfully without leaving dangling references. Verify ordinary deletion and duplicate merging.

**Effort:** Small–medium. Do not edit migration 010.

### F11 — Plex can mark the wrong film/show watched

**Status (17.09.2026):** Resolved. Explicit mappings and movie TMDB IDs take precedence; title fallback requires a unique normalized exact match and consistent movie year. Ambiguous/substr-only matches remain unmatched. Tests cover remakes, conflicting years, mapping priority, and Unicode punctuation. Original baseline evidence follows.

**Evidence:** [matcher.rs:142](C:/Users/aleks/Desktop/Project/TVC/src-tauri/src/plex/matcher.rs:142) tries movie title plus year, then falls back to title alone even when Plex supplied a conflicting year. Show matching also accepts the first substring match without an ambiguity check.

**Reproduction:** A library containing a 1984 film returns that film for a same-title 2021 Plex event after the year-aware lookup fails.

**Impact:** Watching an untracked remake can change the older film's watched state. Similarly named shows can be confused.

**Fix:** Prefer stable external IDs from Plex metadata when available. A known year conflict should block automatic matching. Treat ambiguous or partial matches as unresolved and allow an explicit correction.

**Acceptance:** Same-title remakes, ambiguous show names, punctuation variants, and explicit title mappings choose the right item or remain unmatched. No unrelated watched state changes.

**Effort:** Medium.

### F12 — Racing reminders do not consistently follow current settings

**Status (17.09.2026):** Implemented. Scheduler replacement is serialized; settings changes rebuild timers; each timer rechecks current event/time/settings before dispatch and records notified only after successful notification creation. Eligibility regression tests pass. Native OS delivery and resume/sleep behavior remain unverified. Original baseline evidence follows.

**Evidence:** [commands/racing.rs:17](C:/Users/aleks/Desktop/Project/TVC/src-tauri/src/commands/racing.rs:17) and [commands/racing.rs:43](C:/Users/aleks/Desktop/Project/TVC/src-tauri/src/commands/racing.rs:43) update series enablement/notification preferences without rebuilding pending timers. Existing tasks captured their old settings.

Conversely, refresh commands reschedule unconditionally, while [scheduler.rs:24](C:/Users/aleks/Desktop/Project/TVC/src-tauri/src/racing/scheduler.rs:24) does not check racing_config.notifications_enabled. Turning that racing master setting off and then refreshing can recreate timers if the central notification settings still allow racing.

The scheduler also marks an event notified regardless of the notification dispatch result.

**Impact:** Disabled series can still notify, changed lead times can remain stale, refresh can bypass the racing-specific off setting, and failed dispatch can be recorded as delivered.

**Fix:** Centralize scheduling eligibility, rebuild timers after relevant preference changes, recheck eligibility at dispatch, and record delivery only for the intended successful outcome. Serialize cancel/rebuild operations.

**Acceptance:** Use a controllable clock to test disable, lead-time changes, refresh while off, delivery failure, and concurrent rescheduling. Require at most one reminder per intended event.

**Effort:** Medium.

### F13 — A bad racing feed can erase valid cached events

**Status (17.09.2026):** Resolved. Parsing rejects malformed/incomplete/empty feeds, missing identities, duplicate UIDs, and invalid times before replacement. A transaction upserts stable event IDs and removes stale entries together, preserving reminder state for unchanged events. Injected write failures roll back. Original baseline evidence follows.

**Evidence:** [api.rs:28](C:/Users/aleks/Desktop/Project/TVC/src-tauri/src/racing/api.rs:28) returns a vector and discards calendar parse errors. [refresh_series](C:/Users/aleks/Desktop/Project/TVC/src-tauri/src/racing/mod.rs:223) treats any successful HTTP response as sufficient, deletes all stored events for the series, and then inserts the parsed vector.

**Trigger:** A feed URL returns HTTP 200 with HTML, malformed ICS, or unexpectedly empty content.

**Impact:** A previously useful calendar becomes empty and fallback URLs are not attempted for parse failures. Replacement is not transactional; an insertion error can leave a partial calendar. Event identities and notified state are also regenerated on refresh.

**Fix:** Return structured parse errors, validate the feed before replacement, distinguish legitimate empty calendars from invalid responses, and commit a complete replacement transaction. Preserve stable event identity and reminder state using the series/UID relationship.

**Acceptance:** HTML, malformed ICS, and an injected insert failure preserve the previous calendar. A valid empty calendar follows an explicit documented policy.

**Effort:** Medium.

### F14 — Racing date handling omits the last visible day and uses the wrong day near midnight

**Status (17.09.2026):** Resolved. UI local-date ranges convert to UTC with an exclusive next-midnight end; backend queries compare actual instants. Events group by local calendar date. ICS named timezones use chrono-tz; ambiguous/nonexistent local timestamps are rejected. Oslo midnight and DST tests pass. Original baseline evidence follows.

**Evidence:** [RaceCalendar.svelte:36](C:/Users/aleks/Desktop/Project/TVC/src/lib/components/racing/RaceCalendar.svelte:36) sends date-only range bounds. [get_events_for_range](C:/Users/aleks/Desktop/Project/TVC/src-tauri/src/racing/mod.rs:146) compares full ISO timestamp strings with those bounds using an inclusive end. A timestamp later on the end date sorts after the date-only string and is excluded.

The calendar groups events using the UTC string prefix at [RaceCalendar.svelte:63](C:/Users/aleks/Desktop/Project/TVC/src/lib/components/racing/RaceCalendar.svelte:63), but displays their time in the user's local timezone.

**Reproduction:** The actual racing query returned zero rows for a fixture at noon on May 31 when the calendar's end bound was May 31; extending the bound to June 1 returned the event. Separately, 2026-09-15T23:30:00Z is placed under September 15 by the current prefix rule, while Europe/Oslo displays September 16 at 01:30.

**Fix:** Convert visible local-day boundaries into UTC instants and query a half-open interval ending at the following local midnight. Group and label events using the same local-date conversion.

**Acceptance:** Cover the last visible calendar day, a month ending on Sunday, both sides of UTC midnight, and daylight-saving transitions.

**Effort:** Medium.

### F15 — Older requests overwrite newer UI selections

**Status (17.09.2026):** Resolved in the audited stores. Generation guards invalidate stale search, calendar, detail/cast/trailer, awards, and Arr selection requests. Arr imports retain their originating server selection. Mocked reversed-completion tests pass. Original baseline evidence follows.

**Evidence:** [searchShows](C:/Users/aleks/Desktop/Project/TVC/src/lib/stores/shows.svelte.ts:206), [searchMovies](C:/Users/aleks/Desktop/Project/TVC/src/lib/stores/movies.svelte.ts:211), and [loadLibrary](C:/Users/aleks/Desktop/Project/TVC/src/lib/stores/arr.svelte.ts:373) assign results after awaiting without verifying that the request still belongs to the active query/server.

The same pattern exists in calendar/detail loading and [awards selection](C:/Users/aleks/Desktop/Project/TVC/src/lib/stores/awards.svelte.ts:113). Request deduplication only combines identical requests; it does not prevent stale results from different requests.

**Reproduction:** Resolve the newer search first and the older search second: the final result is the old title. Switch Arr from server A to B and resolve A last: selectedServer remains B while displayed library items come from A. The import request is built using the current selected server, so this can also associate items with the wrong server.

These checks executed the current store functions with mocked IPC and simple state shims, without mounting Svelte or connecting to an app/database.

**Fix:** Use a request generation/selection token. Only the latest matching request may update data, errors, and loading state. Clear or invalidate pending requests on close/switch. Snapshot server selection for an import.

**Acceptance:** Test deliberately reversed responses for searches, calendar navigation, Arr selection, and awards/detail selection. Older responses must not overwrite current state.

**Effort:** Medium.

### F16 — Failed show/movie additions look successful

**Status (17.09.2026):** Resolved in the audited mutation paths. Failed show/movie additions reject and leave the modal item available; tier/settings and other affected stores show errors and retain editable state when saving fails. Mocked failures no longer reach the Added success branch. Original baseline evidence follows.

**Evidence:** [addShow](C:/Users/aleks/Desktop/Project/TVC/src/lib/stores/shows.svelte.ts:272) and [addMovie](C:/Users/aleks/Desktop/Project/TVC/src/lib/stores/movies.svelte.ts:262) catch and log write failures without rejecting or returning failure. [SearchModal.svelte:94](C:/Users/aleks/Desktop/Project/TVC/src/lib/components/SearchModal.svelte:94) and [MovieSearchModal.svelte:87](C:/Users/aleks/Desktop/Project/TVC/src/lib/components/MovieSearchModal.svelte:87) then add the item to their successful-added sets.

**Reproduction:** Mock add_show to reject: awaiting the current addShow still resolves normally. The calling component's success branch therefore remains reachable.

**Impact:** A failed addition gets an added indicator and cannot be retried through the same control until local state is reset. Other mutation stores, including several tier/racing operations, also hide errors.

**Fix:** Propagate mutation errors or return an explicit success/failure result. Update success-only UI after confirmed persistence and show a recoverable error. Reuse the better error propagation already present in tier-only search operations.

**Acceptance:** An offline TMDB request or rejected database write leaves the item retryable, displays an error, and never marks it added.

**Effort:** Small–medium.

### F17 — Sync status hides partial failures

**Status (17.09.2026):** Resolved for latest-run reporting. TMDB, racing, and awards retain visible named failures and targeted retries. Attempts are separate from last complete success; failures and subset retries do not advance that success marker. Persistence/failure tests and mocked browser controls pass. Multi-run history and a combined dashboard remain optional. Original baseline evidence follows.

**Original TMDB evidence:** `sync_all_shows` and `sync_all_movies` returned success counts while failures stayed in logs. Data Management could not distinguish a fully successful sync from a partial one using that response.

[Racing refresh](C:/Users/aleks/Desktop/Project/TVC/src-tauri/src/racing/mod.rs:257) similarly catches per-series failures and advances last_refreshed even if every series failed. Awards has a better returned error summary, but its last-sync timestamp also records an attempt regardless of success.

**Impact:** Users cannot tell which metadata is stale or whether retrying is necessary. A recent-looking refresh timestamp can describe a failed attempt.

**Fix:** Return attempted, succeeded, failed, and skipped counts with per-item errors. Track last attempted and last successful refresh separately. Keep the last good data visible with its real freshness.

**Acceptance:** An all-failed refresh is clearly unsuccessful; partial success names the failures; retry can target only failed items.

**Effort:** Medium.

### F18 — Racing and awards requests can remain pending indefinitely

**Status (17.09.2026):** Resolved. Racing and Wikipedia use a reusable HTTP client with a 10-second connect timeout and 30-second total request deadline. A local stalled-response test proves timeout and subsequent recovery; no live provider was needed. Original baseline evidence follows.

**Evidence:** [racing/api.rs:9](C:/Users/aleks/Desktop/Project/TVC/src-tauri/src/racing/api.rs:9) uses the default request client, and [awards/wikipedia.rs:24](C:/Users/aleks/Desktop/Project/TVC/src-tauri/src/awards/wikipedia.rs:24) uses Client::new. The installed reqwest 0.12.25 defaults have no total/read/connect timeout. TMDB and Arr clients already set explicit deadlines.

**Impact:** A connection that stalls can leave the refresh spinner active and block later requests in sequential sync loops for an unbounded time.

**Fix:** Use explicit connection and total/read deadlines, reuse configured clients, and provide bounded retry/cancellation behavior. Preserve cached content when an attempt fails.

**Acceptance:** A local test server that accepts a connection but never completes the response causes a bounded failure; loading state clears and a later retry works.

**Effort:** Small–medium.

## 2. Additional issues and focused follow-ups

**Implemented on 17.09.2026.** The table below preserves the original findings and proposed checks. Current results:

- Actual Settings/ConfirmDialog keyboard checks passed, including Cancel+Enter, wrapping focus, nested Escape, and focus return.
- Prediction writes validate category membership and the existing future/nominated/no-winner editing policy.
- Duplicate merge metadata and atomic rollback are regression-tested.
- Period statistics include the entire final local day; canonical date validation rejects impossible/unpadded dates.
- A real Svelte boundary catches initial render errors and recovers via Try Again; async operation failures receive explicit feedback.
- Unimplemented premiere/update notification controls are removed; awards reminders remain an optional feature.
- Episode queries and calendar grouping use the same custom-or-air date rule, including archived filtering.

| Priority | Issue / evidence | Recommended action and verification |
| --- | --- | --- |
| P2, source-confirmed input behavior; GUI validation pending | [ConfirmDialog.svelte:56](C:/Users/aleks/Desktop/Project/TVC/src/lib/components/common/ConfirmDialog.svelte:56) confirms on any window-level Enter without checking the target or active dialog. Settings/confirmation dialogs do not implement focus trapping/restoration. | Mount the actual dialog and test Enter while Cancel is focused, Enter in background controls, Tab/Shift+Tab, Escape, nested dialogs, and focus return. Use native button behavior and an actual modal focus boundary. |
| P3, source-confirmed contract gap | [set_prediction](C:/Users/aleks/Desktop/Project/TVC/src-tauri/src/awards/db.rs:357) does not check that the nominee belongs to the category or that predictions are still open; foreign keys only validate each ID independently. | Add inexpensive membership validation when touching this code, especially before prediction import. Decide the lock/edit policy based on how the owner and friend want to use it; a competitive anti-cheating system is unnecessary. A normal current UI click was not shown to produce a mismatched pair. |
| P2, source-confirmed data omission | [merge_duplicates](C:/Users/aleks/Desktop/Project/TVC/src-tauri/src/commands/duplicates.rs:91) merges watched/date fields but discards duplicate episode ratings and tags. It also lacks a keep-ID/merge-ID inequality guard. | Define conflict handling, preserve user metadata, move integration references, and reject identical/nonexistent IDs. Use disposable merge fixtures before changing this destructive workflow. |
| P3, SQL reproduced | [statistics.rs](C:/Users/aleks/Desktop/Project/TVC/src-tauri/src/commands/statistics.rs:129) compares watched_at to an inclusive date-only end bound. A watched timestamp at noon on the end day is excluded. loadPeriodStats currently has no component call site. | Fix before exposing period charts. Use half-open datetime ranges with a defined local timezone; test the entire final day. This is a backend/API defect, not a demonstrated current dashboard regression. |
| P3, source-confirmed | [validate_date](C:/Users/aleks/Desktop/Project/TVC/src-tauri/src/commands/validation.rs:35) checks numeric ranges rather than real dates and exact padding. It accepts impossible dates such as February 31 and noncanonical date strings. | Parse with chrono and require canonical YYYY-MM-DD. Test leap years and malformed dates. Normal date-picker input limits current exposure. |
| P3, needs runtime validation | [ErrorBoundary.svelte](C:/Users/aleks/Desktop/Project/TVC/src/lib/components/common/ErrorBoundary.svelte:23) installs window.onerror after mount rather than a Svelte rendering boundary; it does not handle rejected promises. | Test an initial render exception, later render exception, and failed async operation. Use a real rendering boundary plus explicit async error handling where appropriate. |
| P3, source-confirmed gap | Notification settings expose premiere/update categories, but production notification creation for all advertised categories is not established by the current implementation. Migration 017 adds awards_enabled, which is absent from the current notification settings model/UI. | Trace intended notification producers and either implement the promised categories or clarify the UI. Treat awards reminders as a feature, not an existing working toggle. |
| P3, source-confirmed query mismatch | Episode-range SQL selects by aired OR scheduled date, while Agenda displays scheduled date first without a matching range filter. A rescheduled episode can be fetched for its original month but shown under a date outside that month. | Define the visible date once and use it consistently for retrieval/grouping. Test rescheduling across month boundaries in Month, Week, and Agenda views. |

## 3. Engineering and release improvements

### One owner for schema changes — implemented

The duplicate frontend migration shim is removed. The frontend shares an in-flight connection promise; numbered Rust migrations own schema updates. Checksum repair accepts only identical SQL with different LF/CRLF endings; unknown changes remain errors for SQLx to reject. Released migrations 001–018 were untouched; 019 adds title-deletion cleanup. Fresh in-memory schemas apply the numbered migrations in order, and tests cover deletion and checksum behavior.

### Keep release checks lightweight — implemented

`npm run check` runs Svelte/TypeScript diagnostics, all `scripts/test-*.mjs` frontend tests, offline locked Rust unit tests, and the frontend production build. The release script runs this command before version changes, commits, or publishing. It does not launch Tauri or access installed databases. Cargo dependencies must already be cached for the offline test phase.

The final checks passed: 18 frontend tests; 75 Rust tests with one live-network test intentionally ignored; zero Svelte errors and five existing CSS warnings; production frontend build succeeded. A GitHub CI workflow and a comprehensive native desktop automation suite remain optional.

### Reduce hidden failures and improve diagnostics — implemented for audited paths

Audited searches/details/calendars, title additions, tier mutations, notification/racing settings, and awards/Arr requests now discard stale results or surface failures. Latest-run reports distinguish attempts, failures, and complete success. Racing and Wikipedia reuse a bounded HTTP client. Broader diagnostics export, concurrency changes, and virtualization remain optional pending a demonstrated need.

The frontend build still reports mixed static/dynamic store imports. This is a bundling warning, not a build failure; no speculative bundling refactor was added.

### Security and repository hygiene

The release-note HTML and partial-restore issues are now fixed and regression-tested. The July audit's signing-key concern was not reclassified as a confirmed disclosure: this audit did not read private keys or prove historical exposure. Current tracked-file checks found no matching private-key or database files.

The embedded TMDB token was previously accepted by the owner; it is not proposed again as an emergency blocker. Plaintext Arr keys are a local credential-handling tradeoff, not evidence of a remotely exposed service. A credential-vault redesign is low priority at this scale. Full Arr server/request objects are no longer logged by the audited frontend paths; diagnostic messages use names, types, and counts. Any future diagnostics export should omit secrets.

Compatible npm dependency updates were applied without a forced major upgrade; the subsequent npm audit reported zero advisories. This is a registry check of npm dependencies, not a security certification of the app.

Plex currently binds to localhost. Supporting a Plex server on another machine should be a deliberate feature with authentication, not a simple change to listen on every interface.

The plans folder is gitignored. This report exists locally as requested, but it will not be included by an ordinary git add of tracked changes. No ignore rules or staging were changed.

## 4. Features worth adding

These are optional ideas for the owner and friend. Pick based on actual use and enjoyment; they are not all necessary upgrades. A feature may have high personal value even for two people, while broader infrastructure may add maintenance without helping either person.

Start with improvements to saved data, straightforward error feedback, and the workflows already used. Larger ideas such as multiple tier boards, automatic planning, and a full viewing diary can wait until one of the two users wants them.

**Effort scale:** Small = localized work; medium = multiple layers; large = new data model or substantial workflow. These are relative estimates, not delivery commitments.

| Feature | User value | First useful version | Effort / dependency |
| --- | --- | --- | --- |
| **Restorable prediction archive — included in v3.0 backup** | Protects picks when Wikipedia names/categories change and makes PNG evidence unnecessary for recovery. | Full backup now contains ceremony/category/nominee snapshots, source identity, pick timestamps, and an import scope/count preview. A separate predictions-only merge/import remains optional. | Backup coverage complete; standalone archive is a possible extension. |
| **Compare picks with a friend** | Adds a useful two-person activity if both make award predictions. | Manually exchange prediction JSON files and show a side-by-side comparison with correct/scored/pending totals. Keep the friend's imported picks separate. | Medium; optional extension of prediction export, with no accounts or hosted service needed. |
| **Sync results and retry — implemented** | Explains why TV/movie metadata is stale and allows targeted retries. | Data > Sync shows progress, failures, retries, skipped reasons and five recent runs, with original full-run counts retained after retry. Overview summarises TMDB/matching/backup status. Racing and awards keep their own named failures/retries. | History/status extension implemented on Focus; a cross-source dashboard remains optional. |
| **Automatic TV/movie syncing — implemented** | Keeps metadata refreshed without repeated manual Sync All actions. | Independent Off/daily/weekly/monthly controls, next-due display, and overdue catch-up while TVC runs. Defaults to Off; no OS task or syncing while fully closed. | Complete within the in-app scope; legacy custom-looking schedules are intentionally preserved. |
| **Automatic local backups with restore preview — implemented on Focus** | Makes accidental deletion or a failed upgrade recoverable. | Off/daily/weekly/monthly schedules, chosen folder, last successful status, bounded profile-specific retention, scope/count confirmation, optional portable settings and Restart now. | Complete within the in-app/local-file scope; defaults Off. |
| **Viewing diary and rewatches** | Keeps accurate history when an item is watched more than once or marked later. | Separate watch events, editable watched-on date, rewatch action, and corrected period statistics. | Large; new migration and F07 first. |
| **Weekly viewing plan with a time budget** | Helps fit the backlog into available evenings. | Select shows/movies, set minutes per day, preview a proposed schedule, and apply/undo the batch atomically. | Medium–large; F08 and schedule consistency first. |
| **Plex match review queue** | Prevents silent wrong matches and makes corrections easy. | Show unresolved/ambiguous events with candidate titles, year/ID evidence, and a confirm-and-reprocess action. Extend existing mapping support. | Medium; F05 and F11 first. |
| **Named tier boards** | Allows separate all-time favorites, yearly rankings, genre lists, and awards-season lists. | Multiple named boards with independent assignments/order, duplicate-board action, and existing image export per board. | Large; new schema and backup support. |
| **Prediction performance view** | Shows how picks performed across ceremonies without mixing personal history into awards reference history. | Correct/scored/pending totals, year/category breakdown, and a direct link to each original pick. | Medium; use existing tested outcome rules. |
| **Calendar export** | Makes the personal TV/movie/racing plan usable outside TVC. | Export a selected date range as ICS with stable IDs and correct local/UTC handling. Start with a file export before a live subscription service. | Medium; F08 and F14 first. |
| **Saved library filters and a watch-next queue** | Reduces repeated searching in a large library. | Saved filters for unwatched, returning/ended, network, tags, runtime, and user priority; a small pinned watch-next list. | Medium. |
| **Reliable reminders with quiet hours** | Gives useful premiere, racing, and awards prompts without unwanted interruptions. | Clear category settings, quiet hours, tested scheduling after resume, and a delivery history explaining skipped/failed reminders. | Medium–large; F12 and notification producer audit first. |
| **Visible dev profile — implemented** | Makes the owner's side-by-side testing clearer. | A separate dev identity/data target and TVC — Development title. Redacted diagnostics export remains optional. | Implemented; native coexistence smoke test remains separate. |

### Suggested feature order

1. Restorable predictions and backup coverage for irreplaceable data.
2. Visible dev identity, sync error/retry feedback for TMDB/racing/awards, and automatic TV/movie schedules are implemented.
3. Optional friend prediction comparison if both use awards, or a watch-next queue if that is more useful.
4. Choose at most one larger idea, such as a viewing diary or named tier boards, when either user wants it. Defer the rest.

## 5. Implementation completion

### Completed concrete work

- [x] F01: build-specific SQL preload, debug identity/path/title, and safe legacy development-data carryover.
- [x] F02: reject incomplete refreshes; retain provider-removed episodes with personal state; test rollback.
- [x] F03–F04: atomic validated restore, documented v3.0 backup scope, and preview/compatibility checks.
- [x] F05–F06: Unicode-safe Plex payload handling and escaped release notes.
- [x] F07–F11: watched dates, automatic/custom calendar dates, archived filtering, imported deletion, and conservative Plex matching.
- [x] F12–F14: settings-aware reminder lifecycle, validated atomic racing refresh, and local date boundaries.
- [x] F15–F18: request guards, visible failed mutations, complete-sync freshness, named failures/retries, and request deadlines.
- [x] Additional concrete issues: duplicate merging, prediction validation, canonical dates, statistics, modal focus, rendering recovery, and truthful notification controls.
- [x] Engineering: single schema owner, constrained checksum repair, Svelte diagnostics/tests/build command, and release gate.
- [x] Add migration 019; leave released migrations unchanged.
- [x] Add the requested Sync tab and independent TV/movie automatic schedules.
- [x] Update audit, README, local guidance, and the 0.14.6 unreleased changelog.

### Validation and optional follow-ups

- [x] Run focused disposable-data regressions, full local checks, and affected mocked-browser workflows.
- [ ] Native desktop startup/coexistence using isolated disposable profiles; real OS notification and sleep/resume smoke test.
- [ ] Live integrations using development data only, if needed before sharing the release.
- [ ] Product ideas in section 4, broader CSP/credential changes, CI, and full desktop automation are optional future scope.

## 6. Validation performed

### Original audit — 2026-09-15

| Check | Result | Limits |
| --- | --- | --- |
| TypeScript no-emit check | Passed | Does not fully check Svelte templates. |
| node scripts/test-awards.mjs | 4 passed | Outcome-helper coverage, not complete awards UI testing. |
| cargo test --lib --offline | 39 passed, 1 intentionally ignored | The ignored test contacts Wikipedia; it was not run. No desktop startup. |
| npm run build | Passed | Existing mixed static/dynamic import warnings; no desktop execution. |
| Apply migrations 001–018 to new in-memory SQLite | Passed; integrity check OK | Fresh schema path, not every historical upgrade snapshot. |
| Imported-title deletion reproduction | Foreign-key failure reproduced | Disposable schema/query test. |
| Bulk watched-date reproduction | Existing date overwritten | Disposable SQL test. |
| Partial restore reproduction | Reduced dataset committed after skipped insert error | Strategy reproduction; import command was not invoked. |
| Partial episode replacement reproduction | Omitted season's stored state removed | SQL strategy driven by the source-confirmed partial-fetch behavior. |
| Actual archived episode query | Returned archived episodes | Executed current query against disposable fixtures. |
| Plex wrong-year lookup | Selected the conflicting-year film through fallback | Query-level reproduction. |
| Plex UTF-8 preview | Rust panic reproduced | Tiny standalone program with catch_unwind; release abort behavior checked in configuration. |
| Search/Arr reversed responses | Stale results replaced current results | Current store functions with mocked IPC/state; no mounted UI. |
| Failed addShow request | Store resolved instead of rejecting | Mocked IPC. |
| Release-note parser | Harmless input HTML survived unescaped | No active payload or installed app used. |
| Racing UTC/local date comparison | UTC day differed from Oslo display day | Deterministic date example, not a live feed. |
| Actual racing range query | Omitted the event on the final visible date | Disposable fixture; next-day boundary included it. |
| Statistics end-date comparison | End-day noon timestamp excluded | Unwired period API path noted above. |

The first frontend build attempt was blocked by sandbox access to an ancestor directory; rerunning the same build with the required filesystem access succeeded. This was an environment restriction, not a source-code build failure.

### Sync follow-up — 16.09.2026 (historical status)

| Check | Result | Limits |
| --- | --- | --- |
| `cargo check --lib --offline` | Passed | Compile check; no desktop startup. |
| `cargo test --lib --offline` | 49 passed, 1 existing live-network test ignored | Ten new tests; SQLite tests use in-memory databases and fetch tests use injected responses. |
| `npx tsc --noEmit` | Passed | Does not fully check Svelte templates. |
| `npm run build` | Passed | Existing mixed static/dynamic import warnings remain. |
| Actual Data Management dialog in a browser preview | Passed: separate schedules, failed-save retention, failure details, retry completion, Sync All, and progress after closing/reopening the dialog | Simulated IPC; not a Tauri/TMDB end-to-end run. |

The new tests cover schedule intervals/month ends, overdue/interrupted runs, partial/total failures, saved reports, retry scope without delaying a full sync, manual/unmapped skips, failed/truncated season downloads, saved episode state, transaction rollback, and debug SQL preload selection. No installed database was accessed. Live TMDB, native app startup, tray/resume behavior, and side-by-side dev/release execution remain unverified.

**Subsequent dev-only regression (2026-09-16):** The owner reported that opening Data Management froze interaction with `CloudDownload is not defined`. Moving Sync All had removed an icon import still used by Overview's Simulate Update button. The import is restored. A Svelte AST check failed before the fix and passed afterward for all 14 referenced components. The expanded dev browser check covered Overview's development controls, all five tabs with populated fixtures, and closing/reopening; no runtime errors were recorded. TypeScript and the frontend build passed again. IPC and SQL were mocked, so no installed database was accessed. This corrects the missing-import regression; it does not establish that ErrorBoundary can recover from other render exceptions.

The earlier browser check opened Sync directly and missed Overview. Production builds exclude the failing dev-only branch, and plain TypeScript checking does not validate Svelte template names. Start future smoke checks on the default tab and include dev-only controls; Svelte diagnostics are now included in the local check command (see the later follow-up below).

### Backup/restore follow-up — 17.09.2026

| Check | Result | Limits |
| --- | --- | --- |
| `cargo test --lib --offline` | 55 passed, 1 existing live-network test ignored | Six new backup tests; all database fixtures in memory. |
| v3.0 serialized export/restore/export | All included fields and relationships preserved | Includes watched dates, cleared/custom schedules, manual/archived/tier-only entries, tier order, prediction source snapshots/timestamps, Unicode, title corrections, scrobbles, and change history. Excluded preferences remain unchanged. |
| Invalid versions, duplicate IDs, missing references, mismatched picks | Rejected; original fixture unchanged | Tests call the production replacement function. |
| Injected insert failures and deferred foreign-key failure after deletion | Entire replacement rolled back | Checked show, episode, movie, prediction, title-mapping, and final settings-write failures. |
| v1/v2 compatibility and remap ID collisions | Passed | Provider responses are supplied as fixtures; no live remap or post-commit refresh. |
| `node scripts/test-backup.mjs` | 3 passed | Preview validation/counts and explicit old-backup omissions. |
| TypeScript and frontend build | Passed | Existing build warnings; plain TypeScript does not fully check Svelte templates. |
| Data Management/ConfirmDialog in dev browser | Scope preview, cancel, success, older-backup warning, unsupported-file rejection, backend failure, and recovered controls verified | File/IPC/SQL APIs mocked. Accessibility state verified; screenshot capture timed out. No installed database was accessed. |

The GitHub recovery point remains `cf70820`; new changes are local on `codex/audit-data-reliability`. No release, migration-file edit, or installed database operation was performed.

### Existing awards work retained

The current automated suite covers pending results, tied winners, prediction history after ceremony closure, nominee whitespace/punctuation changes, preserved unmatched picks, and the moved Emmy categories. These tests passed. This audit found no failing test that reopens the previously fixed scoring issue.

The user's successful release check remains separate evidence. This audit did not inspect personal picks in either installed database or verify live winners. Structured backup, backend prediction validation, stale-request protection, and clearer sync completeness are now implemented.

### Final implementation validation — 17.09.2026

| Check | Result | Practical limit |
| --- | --- | --- |
| `npm run check` | Passed: 18 frontend tests, Rust suite, Svelte diagnostics, and production frontend build | Source checks only; no installed app launched. |
| Final `cargo test --lib --locked --offline` after added rollback/freshness regressions | 75 passed, 1 live-network test ignored | In-memory DBs plus one disposable dev-profile WAL-copy fixture; no installed databases. |
| Svelte diagnostics | 0 errors, 5 existing CSS warnings | Production build retains existing mixed-import warnings. |
| `npm audit fix` / audit result | Compatible updates applied; 0 reported npm advisories | No forced major upgrades; Cargo security review was not performed. |
| Network/date regressions | Bounded stalled HTTP response; invalid feed rollback; stable IDs/reminders; Oslo DST/local midnight; complete final-day statistics | Local server and fixtures; no live providers. |
| State/error regressions | Reversed requests, close/clear, Arr server switch, failed additions; render error/retry; malformed backup | Store tests use mocked IPC; browser tests mount actual components. |
| Mocked browser workflow | Confirm Cancel+Enter, focus wrapping/return, nested Settings/Confirm Escape, initial-render failure/retry, TMDB failure controls, racing/awards named failures and retry controls | Native/file/SQL APIs mocked. Cancel/focus and render recovery rechecked after npm updates. |
| Formatting/diff | Changed Rust files formatted; `git diff --check` clean | Released migrations 001–018 unchanged; app version files remain 0.14.5 for the release script. |

The recovery point on GitHub remains `cf70820`; this implementation is local and uncommitted on `codex/audit-data-reliability`. No release was built/installed or published. The plans directory is still gitignored.

### Follow-up validation boundary

The dev identity/path changes and carryover algorithm are implemented and unit-tested. Native side-by-side startup and OS notification/sleep behavior have not been exercised; use new isolated disposable profiles for that smoke test and verify database targets first. Live TMDB, Wikipedia, Sonarr/Radarr, and Plex interactions were not run. Existing ambiguous schedules are deliberately preserved, and backups retain the explicit scope/legacy-migration limits above. These are validation boundaries and stated compatibility policies, not claims that every external integration or future input is covered.
