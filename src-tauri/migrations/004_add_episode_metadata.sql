-- Add metadata columns to episodes table

-- User rating (1-5 stars)
ALTER TABLE episodes ADD COLUMN rating INTEGER;

-- Tags for organization (stored as JSON array)
ALTER TABLE episodes ADD COLUMN tags TEXT;



