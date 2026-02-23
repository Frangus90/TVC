-- Add rank_order column for ordering items within same rating tier
ALTER TABLE shows ADD COLUMN rank_order INTEGER;
ALTER TABLE movies ADD COLUMN rank_order INTEGER;

-- Indexes for efficient ordering queries
CREATE INDEX idx_shows_rating_order ON shows(rating, rank_order);
CREATE INDEX idx_movies_rating_order ON movies(rating, rank_order);
