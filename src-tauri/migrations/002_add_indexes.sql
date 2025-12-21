-- Add performance indexes for common queries

-- Add scheduled_date column to episodes table (needed for scheduling feature)
ALTER TABLE episodes ADD COLUMN scheduled_date TEXT;

-- Composite index for calendar queries (show_id + aired)
CREATE INDEX IF NOT EXISTS idx_episodes_show_air ON episodes(show_id, aired);

-- Index for scheduled episodes (partial index for non-null values)
CREATE INDEX IF NOT EXISTS idx_episodes_scheduled ON episodes(scheduled_date) WHERE scheduled_date IS NOT NULL;

-- Index for watched status queries
CREATE INDEX IF NOT EXISTS idx_episodes_watched ON episodes(watched);

-- Composite index for episode lookups by show, season, and episode number
CREATE INDEX IF NOT EXISTS idx_episodes_show_season_ep ON episodes(show_id, season_number, episode_number);

-- Index for show name searches
CREATE INDEX IF NOT EXISTS idx_shows_name ON shows(name);







