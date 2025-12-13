-- Tracked TV shows
CREATE TABLE IF NOT EXISTS shows (
    id INTEGER PRIMARY KEY,           -- TVDB series ID
    name TEXT NOT NULL,
    slug TEXT,
    status TEXT,                      -- Continuing, Ended, etc.
    poster_url TEXT,
    first_aired TEXT,
    network TEXT,
    overview TEXT,
    airs_time TEXT,                   -- e.g., "21:00"
    airs_days TEXT,                   -- JSON: {"monday": true, ...}
    runtime INTEGER,
    added_at TEXT DEFAULT (datetime('now')),
    last_synced TEXT
);

-- Episodes for tracked shows
CREATE TABLE IF NOT EXISTS episodes (
    id INTEGER PRIMARY KEY,           -- TVDB episode ID
    show_id INTEGER NOT NULL,
    season_number INTEGER,
    episode_number INTEGER,
    name TEXT,
    overview TEXT,
    aired TEXT,                       -- Air date (YYYY-MM-DD)
    runtime INTEGER,
    image_url TEXT,
    watched INTEGER DEFAULT 0,        -- 0=unwatched, 1=watched
    watched_at TEXT,
    FOREIGN KEY (show_id) REFERENCES shows(id) ON DELETE CASCADE
);

-- Indexes for faster queries
CREATE INDEX IF NOT EXISTS idx_episodes_show_id ON episodes(show_id);
CREATE INDEX IF NOT EXISTS idx_episodes_aired ON episodes(aired);
CREATE INDEX IF NOT EXISTS idx_episodes_watched ON episodes(watched);

-- App settings
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT
);
