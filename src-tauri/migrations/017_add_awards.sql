-- Awards feature: Oscars & Emmys nominations, history, and a prediction game.
-- Data is pulled at runtime from Wikipedia (MediaWiki API) into these tables; no
-- rows are seeded here. A ceremony holds categories, each category holds nominees,
-- and a nominee may carry the user's single prediction. See plans/awards-feature.md.

-- One ceremony per award edition (e.g. the 97th Academy Awards).
CREATE TABLE IF NOT EXISTS award_ceremonies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    award_type TEXT NOT NULL,              -- 'oscars' | 'emmys'
    edition INTEGER NOT NULL,              -- ceremony ordinal, e.g. 97
    name TEXT NOT NULL,                    -- '97th Academy Awards'
    year INTEGER NOT NULL,                 -- ceremony year
    ceremony_date TEXT,                    -- ISO date, nullable
    status TEXT NOT NULL,                  -- 'past' | 'nominated' | 'upcoming'
    wiki_title TEXT NOT NULL,              -- MediaWiki page title used to sync
    last_synced TEXT,
    UNIQUE (award_type, edition)
);

-- A competitive category within a ceremony (e.g. Best Picture).
CREATE TABLE IF NOT EXISTS award_categories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ceremony_id INTEGER NOT NULL REFERENCES award_ceremonies(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    display_order INTEGER,
    UNIQUE (ceremony_id, name)
);

-- A nominee within a category. is_winner: 1 winner, 0 loser, NULL = not yet announced.
CREATE TABLE IF NOT EXISTS award_nominees (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    category_id INTEGER NOT NULL REFERENCES award_categories(id) ON DELETE CASCADE,
    title TEXT NOT NULL,                   -- the film / show / work
    detail TEXT,                           -- people / network / song (nullable)
    is_winner INTEGER,                     -- 1 | 0 | NULL (unknown)
    source_key TEXT NOT NULL,              -- normalized natural key for idempotent upsert
    UNIQUE (category_id, source_key)
);

-- The user's single prediction per category (single-user, local).
CREATE TABLE IF NOT EXISTS award_predictions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    category_id INTEGER NOT NULL REFERENCES award_categories(id) ON DELETE CASCADE,
    nominee_id INTEGER NOT NULL REFERENCES award_nominees(id) ON DELETE CASCADE,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (category_id)
);

CREATE INDEX IF NOT EXISTS idx_award_ceremonies_type ON award_ceremonies(award_type);
CREATE INDEX IF NOT EXISTS idx_award_categories_ceremony ON award_categories(ceremony_id);
CREATE INDEX IF NOT EXISTS idx_award_nominees_category ON award_nominees(category_id);
CREATE INDEX IF NOT EXISTS idx_award_predictions_category ON award_predictions(category_id);

-- Per-category notification toggle, matching the existing *_enabled columns.
ALTER TABLE notification_settings ADD COLUMN awards_enabled INTEGER DEFAULT 1;
