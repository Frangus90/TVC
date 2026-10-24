# Episode sync ID collisions

## Investigation

The reported error is `Failed to sync episode: UNIQUE constraint failed: episodes.id`, affecting Top Gear. Both manual show refresh and Data sync call the same native episode-refresh function.

- Existing refresh keeps episodes with personal state, then replaces returned episodes one season/episode at a time. A new episode ID can still belong to a saved row awaiting replacement or a retained episode whose numbering moved.
- Memory-only reproductions cover swapped IDs/positions, a moved watched special, an ID owned by another show and duplicate IDs in an API response. Failed refreshes roll back their transaction.
- On 24.10.2026 the public TMDB response for Top Gear (2002), TV ID 45, contained 334 episodes across 34 seasons, with no duplicate IDs or season/episode pairs. The verified development database does not contain Top Gear. The exact conflicting row in the reported installation has not been inspected; the release database and sidecars must never be accessed.

## Plan

1. Reconcile the complete response before inserts. Match saved episodes by unchanged provider ID first, then by season/episode when an old ID is absent from the response. Preserve personal state and legacy IDs; retain unmatched protected rows.
2. Remove all matched old rows before inserting any replacements. Validate duplicate IDs/positions up front. Keep writes, metadata and history/scrobble ID updates in one transaction; reject IDs belonging to another show with useful details.
3. Add memory-only regressions for moves/swaps, changed IDs, preserved state/references, unrelated-show conflicts and rollback. Run project checks and update the next-release changelog without changing app version files or released migrations.

## Safeguards

Work on `codex/fix-episode-sync-collisions`. Never open, copy, repair or otherwise touch the release database or its sidecars. No data deletion/repair command is run against installed libraries. Do not release, tag, merge main or modify the pre-Focus recovery backup.

## Validation

- Reconciliation now plans identity matches against the complete response, deletes every matched old row before inserts, and preserves watched status/date, schedules, ratings, tags and legacy TVDB IDs. Unmatched rows with personal state or current/legacy history/Plex references remain saved.
- Changed-ID number fallbacks update typed history/Plex references in the same transaction. Their old IDs are absent from the incoming-ID set, so source/destination sets do not overlap and sequential updates cannot chain into another episode.
- Incoming IDs and season/episode pairs are checked before writes. A conflict with another show's episode reports both the incoming episode ID/number and its saved owner; no row is stolen or replaced across shows.
- Eight new memory-only regressions initially passed with the five existing episode tests (**13/13**). Independent review found and corrected an extra cleanup case: unmatched records referenced only by a legacy episode ID must remain protected, matching existing Maintenance behavior. A ninth regression covers that case.
- Final validation after that correction: **0 Svelte errors/warnings, 101 frontend tests passed, 139 Rust tests passed (including all 14 episode tests), 1 live Wikipedia test ignored, production frontend build passed**. Nine new episode regressions cover moves/swaps, full personal state and legacy IDs, typed history/Plex references, unrelated owners, invalid/duplicate responses, reference-only retention and a real insert failure rolling back metadata and history.
- Both independent reviews passed after the legacy-reference correction. Rust formatting and `git diff --check` passed. App manifests/locks remain **0.15.1**; the only new version entry is **0.15.2 Unreleased** in the human-readable changelog. Released migrations are unchanged.
- Logs and current public-feed diagnostics are ignored local artifacts: `episode-sync-id-collisions-check.log` and `top-gear-tmdb-diagnostics.json`. The JSON contains public episode metadata only, not an API token or installed-library data.
- Only the verified `com.tvc.app.dev/tvc_dev.db` was queried, read-only, to check whether Top Gear existed. No installed data was changed, and the release database/sidecars were never accessed.

## Remaining limits

The public current feed and synthetic saved-state reproductions establish sync bugs, but they do not identify the exact stored row behind the reported Top Gear failure. The fix is intended to resolve same-show moves/reordering. Genuine cross-show ID ownership conflicts or malformed provider responses still stop with preserved data and a clearer diagnostic; they are not silently discarded. The owner must retry the affected title after testing/installing the fix. No release is published as part of this work.
