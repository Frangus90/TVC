-- Add movies table for TMDB movie tracking
CREATE TABLE IF NOT EXISTS movies (
  id INTEGER PRIMARY KEY,
  title TEXT NOT NULL,
  tagline TEXT,
  overview TEXT,
  poster_url TEXT,
  backdrop_url TEXT,
  release_date TEXT,
  digital_release_date TEXT,
  physical_release_date TEXT,
  runtime INTEGER,
  status TEXT,
  genres TEXT,
  vote_average REAL,
  scheduled_date TEXT,
  watched INTEGER DEFAULT 0,
  watched_at TEXT,
  rating INTEGER,
  notes TEXT,
  color TEXT,
  tags TEXT,
  archived INTEGER DEFAULT 0,
  added_at TEXT DEFAULT (datetime('now')),
  last_synced TEXT
);

CREATE INDEX idx_movies_digital_release ON movies(digital_release_date);
CREATE INDEX idx_movies_scheduled ON movies(scheduled_date);
CREATE INDEX idx_movies_archived ON movies(archived);

-- Add archived column to shows table
ALTER TABLE shows ADD COLUMN archived INTEGER DEFAULT 0;
CREATE INDEX idx_shows_archived ON shows(archived);
