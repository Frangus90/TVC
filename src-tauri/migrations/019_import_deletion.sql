-- Imported titles may be deleted through the library, tiers, or quarantine.
-- Keep this cleanup at the database boundary so all delete paths agree.
CREATE TRIGGER cleanup_show_imports BEFORE DELETE ON shows BEGIN
    DELETE FROM sonarr_imports WHERE show_id = OLD.id;
    DELETE FROM title_mappings WHERE media_type = 'show' AND tvc_id = OLD.id;
END;
CREATE TRIGGER cleanup_movie_imports BEFORE DELETE ON movies BEGIN
    DELETE FROM radarr_imports WHERE movie_id = OLD.id;
    DELETE FROM title_mappings WHERE media_type = 'movie' AND tvc_id = OLD.id;
END;
