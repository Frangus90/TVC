-- ============================================
-- Part A: Tier-only decoupling
-- ============================================

-- Flag to mark shows/movies as tier-list only (not tracked in calendar)
ALTER TABLE shows ADD COLUMN tier_only INTEGER NOT NULL DEFAULT 0;
ALTER TABLE movies ADD COLUMN tier_only INTEGER NOT NULL DEFAULT 0;

CREATE INDEX idx_shows_tier_only ON shows(tier_only);
CREATE INDEX idx_movies_tier_only ON movies(tier_only);

-- ============================================
-- Part B: Custom tiers with tier_id FK
-- ============================================

-- Tier definitions (user-customizable)
CREATE TABLE tiers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    position INTEGER NOT NULL,
    name TEXT NOT NULL,
    color TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Seed default tiers (10-star preset, position 10=best, 1=worst)
INSERT INTO tiers (position, name, color) VALUES
    (10, 'Masterpiece', ''),
    (9, 'Excellent', ''),
    (8, 'Great', ''),
    (7, 'Good', ''),
    (6, 'Solid', ''),
    (5, 'Average', ''),
    (4, 'Below Average', ''),
    (3, 'Poor', ''),
    (2, 'Bad', ''),
    (1, 'Terrible', '');

-- Add tier_id FK to shows and movies
ALTER TABLE shows ADD COLUMN tier_id INTEGER REFERENCES tiers(id) ON DELETE SET NULL;
ALTER TABLE movies ADD COLUMN tier_id INTEGER REFERENCES tiers(id) ON DELETE SET NULL;

CREATE INDEX idx_shows_tier_id ON shows(tier_id);
CREATE INDEX idx_movies_tier_id ON movies(tier_id);

-- Migrate existing ratings to tier_id
-- Maps: 5.0 -> position 10, 4.5 -> 9, 4.0 -> 8, ..., 0.5 -> 1
UPDATE shows SET tier_id = (
    SELECT t.id FROM tiers t WHERE t.position = CAST(shows.rating * 2 AS INTEGER)
) WHERE rating IS NOT NULL;

UPDATE movies SET tier_id = (
    SELECT t.id FROM tiers t WHERE t.position = CAST(movies.rating * 2 AS INTEGER)
) WHERE rating IS NOT NULL;

-- Store tier preset preference
INSERT OR IGNORE INTO settings (key, value) VALUES ('tier_preset', '10-star');
