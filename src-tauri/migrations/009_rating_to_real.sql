-- Convert ratings from INTEGER to REAL for half-star support
-- Shows table
ALTER TABLE shows ADD COLUMN rating_new REAL;
UPDATE shows SET rating_new = CAST(rating AS REAL) WHERE rating IS NOT NULL;
ALTER TABLE shows DROP COLUMN rating;
ALTER TABLE shows RENAME COLUMN rating_new TO rating;

-- Movies table
ALTER TABLE movies ADD COLUMN rating_new REAL;
UPDATE movies SET rating_new = CAST(rating AS REAL) WHERE rating IS NOT NULL;
ALTER TABLE movies DROP COLUMN rating;
ALTER TABLE movies RENAME COLUMN rating_new TO rating;
