-- Cast members for shows and movies
CREATE TABLE IF NOT EXISTS cast_members (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    show_id INTEGER,
    movie_id INTEGER,
    person_id INTEGER,
    name TEXT NOT NULL,
    character_name TEXT,
    order_index INTEGER DEFAULT 0,
    image_url TEXT,
    FOREIGN KEY (show_id) REFERENCES shows(id) ON DELETE CASCADE,
    FOREIGN KEY (movie_id) REFERENCES movies(id) ON DELETE CASCADE
);

-- Crew members for shows and movies
CREATE TABLE IF NOT EXISTS crew_members (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    show_id INTEGER,
    movie_id INTEGER,
    person_id INTEGER,
    name TEXT NOT NULL,
    job TEXT,
    department TEXT,
    image_url TEXT,
    FOREIGN KEY (show_id) REFERENCES shows(id) ON DELETE CASCADE,
    FOREIGN KEY (movie_id) REFERENCES movies(id) ON DELETE CASCADE
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_cast_show ON cast_members(show_id);
CREATE INDEX IF NOT EXISTS idx_cast_movie ON cast_members(movie_id);
CREATE INDEX IF NOT EXISTS idx_crew_show ON crew_members(show_id);
CREATE INDEX IF NOT EXISTS idx_crew_movie ON crew_members(movie_id);
