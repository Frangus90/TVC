-- Sonarr/Radarr server credentials
CREATE TABLE IF NOT EXISTS arr_servers (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    type TEXT NOT NULL CHECK (type IN ('sonarr', 'radarr')),
    base_url TEXT NOT NULL,
    api_key TEXT NOT NULL,
    is_active INTEGER DEFAULT 1,
    auto_sync_enabled INTEGER DEFAULT 0,
    sync_interval_hours INTEGER DEFAULT 24,
    last_synced TEXT,
    added_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- Track Sonarr imports
CREATE TABLE IF NOT EXISTS sonarr_imports (
    id INTEGER PRIMARY KEY,
    show_id INTEGER NOT NULL,
    sonarr_series_id INTEGER,
    arr_server_id INTEGER NOT NULL,
    monitored INTEGER,
    imported_at TEXT DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (show_id) REFERENCES shows(id),
    FOREIGN KEY (arr_server_id) REFERENCES arr_servers(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_sonarr_imports_show ON sonarr_imports(show_id);
CREATE INDEX IF NOT EXISTS idx_sonarr_imports_server ON sonarr_imports(arr_server_id);

-- Track Radarr imports
CREATE TABLE IF NOT EXISTS radarr_imports (
    id INTEGER PRIMARY KEY,
    movie_id INTEGER NOT NULL,
    radarr_movie_id INTEGER,
    arr_server_id INTEGER NOT NULL,
    monitored INTEGER,
    imported_at TEXT DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (movie_id) REFERENCES movies(id),
    FOREIGN KEY (arr_server_id) REFERENCES arr_servers(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_radarr_imports_movie ON radarr_imports(movie_id);
CREATE INDEX IF NOT EXISTS idx_radarr_imports_server ON radarr_imports(arr_server_id);

-- Plex Scrobbler configuration (single row)
CREATE TABLE IF NOT EXISTS plex_scrobbler_config (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    enabled INTEGER DEFAULT 0,
    port INTEGER DEFAULT 9876,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);

INSERT OR IGNORE INTO plex_scrobbler_config (id) VALUES (1);

-- Plex scrobble history log
CREATE TABLE IF NOT EXISTS plex_scrobble_log (
    id INTEGER PRIMARY KEY,
    event_type TEXT NOT NULL,
    media_type TEXT NOT NULL,
    raw_title TEXT NOT NULL,
    show_name TEXT,
    season_number INTEGER,
    episode_number INTEGER,
    year INTEGER,
    matched_entity_type TEXT,
    matched_entity_id INTEGER,
    match_method TEXT,
    scrobbled_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_plex_scrobble_log_date ON plex_scrobble_log(scrobbled_at);

-- Title mappings for fuzzy match corrections (Plex title -> TVC title)
CREATE TABLE IF NOT EXISTS title_mappings (
    id INTEGER PRIMARY KEY,
    plex_title TEXT NOT NULL UNIQUE,
    media_type TEXT NOT NULL CHECK (media_type IN ('show', 'movie')),
    tvc_id INTEGER NOT NULL,
    tvc_title TEXT NOT NULL,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_title_mappings_plex ON title_mappings(plex_title);
CREATE INDEX IF NOT EXISTS idx_title_mappings_type ON title_mappings(media_type);
