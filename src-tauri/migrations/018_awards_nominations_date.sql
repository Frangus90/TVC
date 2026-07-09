-- Add the "nominations announced" date (parsed from a ceremony page's Key dates
-- table) so the Predict tab can show when nominations are expected for an upcoming
-- ceremony whose nominations aren't out yet.
ALTER TABLE award_ceremonies ADD COLUMN nominations_date TEXT;
