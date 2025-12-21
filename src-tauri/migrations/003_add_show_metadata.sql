-- Add metadata columns to shows table

-- Color for show-specific theming
ALTER TABLE shows ADD COLUMN color TEXT;

-- Custom notes for shows
ALTER TABLE shows ADD COLUMN notes TEXT;

-- Tags for organization (stored as JSON array)
ALTER TABLE shows ADD COLUMN tags TEXT;






