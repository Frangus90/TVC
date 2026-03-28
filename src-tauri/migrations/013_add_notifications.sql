-- Notification history
CREATE TABLE IF NOT EXISTS notifications (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    type TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    icon TEXT,
    reference_id TEXT,
    reference_type TEXT,
    read INTEGER DEFAULT 0,
    dismissed INTEGER DEFAULT 0,
    created_at TEXT DEFAULT (datetime('now')),
    expires_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_notifications_type ON notifications(type);
CREATE INDEX IF NOT EXISTS idx_notifications_read ON notifications(read);
CREATE INDEX IF NOT EXISTS idx_notifications_created ON notifications(created_at);

-- Notification settings (singleton row)
CREATE TABLE IF NOT EXISTS notification_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    enabled INTEGER DEFAULT 1,
    sound_enabled INTEGER DEFAULT 1,
    sound_volume INTEGER DEFAULT 50,
    sound_choice TEXT DEFAULT 'chime',
    popup_position TEXT DEFAULT 'top-right',
    popup_duration INTEGER DEFAULT 8000,
    max_visible INTEGER DEFAULT 3,
    os_fallback INTEGER DEFAULT 0,
    tray_notifications INTEGER DEFAULT 1,
    racing_enabled INTEGER DEFAULT 1,
    plex_enabled INTEGER DEFAULT 1,
    premiere_enabled INTEGER DEFAULT 1,
    update_enabled INTEGER DEFAULT 1,
    system_enabled INTEGER DEFAULT 1,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);

INSERT OR IGNORE INTO notification_settings (id) VALUES (1);
