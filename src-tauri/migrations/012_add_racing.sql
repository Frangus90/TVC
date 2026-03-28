-- Racing series the user can track
CREATE TABLE IF NOT EXISTS racing_series (
    id INTEGER PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    category TEXT NOT NULL,
    ics_url TEXT NOT NULL,
    custom_ics_url TEXT,
    enabled INTEGER DEFAULT 0,
    notify_enabled INTEGER DEFAULT 1,
    notify_minutes INTEGER DEFAULT 30,
    color TEXT NOT NULL,
    custom_color TEXT,
    added_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- Individual race sessions parsed from ICS feeds
CREATE TABLE IF NOT EXISTS racing_events (
    id INTEGER PRIMARY KEY,
    series_slug TEXT NOT NULL,
    uid TEXT NOT NULL,
    event_title TEXT NOT NULL,
    session_name TEXT,
    circuit TEXT,
    start_time TEXT NOT NULL,
    end_time TEXT,
    description TEXT,
    notified INTEGER DEFAULT 0,
    fetched_at TEXT DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(series_slug, uid),
    FOREIGN KEY (series_slug) REFERENCES racing_series(slug) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_racing_events_series ON racing_events(series_slug);
CREATE INDEX IF NOT EXISTS idx_racing_events_start ON racing_events(start_time);
CREATE INDEX IF NOT EXISTS idx_racing_events_notified ON racing_events(notified);

-- Global racing config (singleton row)
CREATE TABLE IF NOT EXISTS racing_config (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    notifications_enabled INTEGER DEFAULT 1,
    default_notify_minutes INTEGER DEFAULT 30,
    last_refreshed TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);

INSERT OR IGNORE INTO racing_config (id) VALUES (1);

-- Seed known racing series (all disabled by default)
INSERT OR IGNORE INTO racing_series (slug, name, category, ics_url, color) VALUES
    ('f1', 'Formula 1', 'open-wheel', 'https://f1.vidmar.net/calendar.ics', '#E10600'),
    ('f2', 'Formula 2', 'open-wheel', 'https://calendar.google.com/calendar/ical/rttoqh7u6m247f2ub6c05m4pe4%40group.calendar.google.com/public/basic.ics', '#0090D0'),
    ('f3', 'Formula 3', 'open-wheel', 'https://calendar.google.com/calendar/ical/sorhedtr7q5qmea6f0hvf20864%40group.calendar.google.com/public/basic.ics', '#DA291C'),
    ('f1-academy', 'F1 Academy', 'open-wheel', 'https://calendar.google.com/calendar/ical/sorhedtr7q5qmea6f0hvf20864%40group.calendar.google.com/public/basic.ics', '#6B21A8'),
    ('formula-e', 'Formula E', 'open-wheel', 'https://calendar.google.com/calendar/ical/vno0ntshopq0nmob26db2pcen8%40group.calendar.google.com/public/basic.ics', '#00A3E0'),
    ('indycar', 'IndyCar', 'open-wheel', 'https://calendar.google.com/calendar/ical/hlskhf7l8ce7btind39bb9kf1o%40group.calendar.google.com/public/basic.ics', '#1E3A5F'),
    ('super-formula', 'Super Formula', 'open-wheel', 'https://calendar.google.com/calendar/ical/db8c47ne2bt9qbld2mhdabm0u8%40group.calendar.google.com/public/basic.ics', '#003DA5'),
    ('motogp', 'MotoGP', 'motorcycle', 'https://nixxo.github.io/calendars/motogp/2026/MotoGP_2026_calendar.ics', '#BE0000'),
    ('moto2', 'Moto2', 'motorcycle', 'https://nixxo.github.io/calendars/motogp/2026/Moto2_2026_calendar.ics', '#005CA9'),
    ('moto3', 'Moto3', 'motorcycle', 'https://nixxo.github.io/calendars/motogp/2026/Moto3_2026_calendar.ics', '#009639'),
    ('wsbk', 'WorldSBK', 'motorcycle', 'https://calendar.google.com/calendar/ical/fei68gpe16c85ed3jjdtvrn8ns%40group.calendar.google.com/public/basic.ics', '#D4380D'),
    ('wec', 'WEC', 'endurance', 'https://calendar.google.com/calendar/ical/61jccgg4rshh1temqk0dj4lens%40group.calendar.google.com/public/basic.ics', '#004C97'),
    ('imsa', 'IMSA', 'endurance', 'https://calendar.google.com/calendar/ical/hlskhf7l8ce7btind39bb9kf1o%40group.calendar.google.com/public/basic.ics', '#C41E3A'),
    ('nascar-cup', 'NASCAR Cup', 'stock-car', 'https://calendar.google.com/calendar/ical/db8c47ne2bt9qbld2mhdabm0u8%40group.calendar.google.com/public/basic.ics', '#FFD659'),
    ('nascar-xs', 'NASCAR Xfinity', 'stock-car', 'https://calendar.google.com/calendar/ical/db8c47ne2bt9qbld2mhdabm0u8%40group.calendar.google.com/public/basic.ics', '#1572B6'),
    ('nascar-truck', 'NASCAR Truck', 'stock-car', 'https://calendar.google.com/calendar/ical/db8c47ne2bt9qbld2mhdabm0u8%40group.calendar.google.com/public/basic.ics', '#4CAF50'),
    ('wrc', 'WRC', 'rally', 'https://calendar.google.com/calendar/ical/fei68gpe16c85ed3jjdtvrn8ns%40group.calendar.google.com/public/basic.ics', '#0078D4'),
    ('dtm', 'DTM', 'touring', 'https://calendar.google.com/calendar/ical/db8c47ne2bt9qbld2mhdabm0u8%40group.calendar.google.com/public/basic.ics', '#ED1C24'),
    ('v8supercars', 'Supercars', 'touring', 'https://calendar.google.com/calendar/ical/db8c47ne2bt9qbld2mhdabm0u8%40group.calendar.google.com/public/basic.ics', '#00843D'),
    ('supergt', 'Super GT', 'endurance', 'https://calendar.google.com/calendar/ical/db8c47ne2bt9qbld2mhdabm0u8%40group.calendar.google.com/public/basic.ics', '#FF6B00'),
    ('extreme-e', 'Extreme E', 'other', 'https://calendar.google.com/calendar/ical/db8c47ne2bt9qbld2mhdabm0u8%40group.calendar.google.com/public/basic.ics', '#7CB342');
