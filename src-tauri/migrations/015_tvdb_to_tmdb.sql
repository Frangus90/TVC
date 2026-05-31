-- ============================================
-- TVDB -> TMDB migration: structural prep only
-- ============================================
-- Phase 2 of the TVDB-only -> TMDB-only consolidation. Adds the columns the
-- live remap runner needs; the runner then rewrites IDs via UPDATE statements
-- (with deferred FK enforcement, since FK columns referencing shows(id) live
-- in episodes, cast_members, crew_members, sonarr_imports, etc.).
--
-- ADD COLUMN is chosen deliberately over a full table rebuild: rebuilding
-- shows would either trip foreign-key constraint failures at COMMIT (since
-- multiple child tables reference shows(id)) or require dropping every child
-- table's FK, which is invasive. We're not changing the PK type — only
-- adding metadata columns — so ADD COLUMN is the natural fit.

-- New columns on shows: legacy_tvdb_id preserves the original id (positive
-- only — negative ids are manual tier-only entries), unmigrated flags rows
-- that the runner still needs to resolve via TMDB /find.
ALTER TABLE shows ADD COLUMN legacy_tvdb_id INTEGER;
ALTER TABLE shows ADD COLUMN unmigrated INTEGER NOT NULL DEFAULT 0;

UPDATE shows
SET legacy_tvdb_id = id,
    unmigrated = 1
WHERE id > 0;

CREATE INDEX IF NOT EXISTS idx_shows_unmigrated
    ON shows(unmigrated) WHERE unmigrated = 1;

-- Same idea for episodes: preserve the original TVDB id so the runner can
-- match old change_history / plex_scrobble_log rows to the new episode ids.
ALTER TABLE episodes ADD COLUMN legacy_tvdb_id INTEGER;

UPDATE episodes
SET legacy_tvdb_id = id
WHERE id > 0;

-- Mark migration pending so the Phase 3 runner picks it up on next launch.
INSERT OR REPLACE INTO settings (key, value)
VALUES ('tvdb_to_tmdb_migration_complete', '0');
