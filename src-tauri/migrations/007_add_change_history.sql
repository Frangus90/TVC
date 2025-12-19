-- Change history table for tracking modifications
CREATE TABLE IF NOT EXISTS change_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_type TEXT NOT NULL,      -- 'episode', 'movie', 'show'
    entity_id INTEGER NOT NULL,
    change_type TEXT NOT NULL,      -- 'watched', 'scheduled', 'unscheduled', 'rating'
    old_value TEXT,
    new_value TEXT,
    changed_at TEXT DEFAULT (datetime('now')),
    user_action TEXT                -- 'manual', 'auto', 'sync', 'bulk'
);

-- Index for efficient querying
CREATE INDEX IF NOT EXISTS idx_change_history_entity ON change_history(entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_change_history_date ON change_history(changed_at);
CREATE INDEX IF NOT EXISTS idx_change_history_type ON change_history(change_type);
