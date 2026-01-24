use sqlx::{Pool, Row, Sqlite};

use super::models::MatchResult;

/// Match an episode by show name, season, and episode number
pub async fn match_episode(
    pool: &Pool<Sqlite>,
    show_name: &str,
    season: i32,
    episode: i32,
) -> Option<MatchResult> {
    // First check title_mappings for a corrected match
    let mapped_show_id: Option<i64> = sqlx::query_scalar(
        "SELECT tvc_id FROM title_mappings WHERE plex_title = ? AND media_type = 'show'"
    )
    .bind(show_name)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let show_id = if let Some(id) = mapped_show_id {
        Some(id)
    } else {
        // Try fuzzy match by show name (case-insensitive)
        find_show_by_name(pool, show_name).await
    };

    if let Some(show_id) = show_id {
        // Find the episode
        let episode_id: Option<i64> = sqlx::query_scalar(
            "SELECT id FROM episodes WHERE show_id = ? AND season_number = ? AND episode_number = ?"
        )
        .bind(show_id)
        .bind(season)
        .bind(episode)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

        if let Some(episode_id) = episode_id {
            return Some(MatchResult {
                entity_type: "episode".to_string(),
                entity_id: episode_id,
                method: if mapped_show_id.is_some() { "mapping" } else { "title" }.to_string(),
            });
        }
    }

    None
}

/// Match a movie by title and year
pub async fn match_movie(
    pool: &Pool<Sqlite>,
    title: &str,
    year: Option<i32>,
) -> Option<MatchResult> {
    // First check title_mappings
    let mapped_movie_id: Option<i64> = sqlx::query_scalar(
        "SELECT tvc_id FROM title_mappings WHERE plex_title = ? AND media_type = 'movie'"
    )
    .bind(title)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    if let Some(movie_id) = mapped_movie_id {
        return Some(MatchResult {
            entity_type: "movie".to_string(),
            entity_id: movie_id,
            method: "mapping".to_string(),
        });
    }

    // Try fuzzy match by title (and optionally year)
    let movie_id = find_movie_by_title(pool, title, year).await;

    if let Some(movie_id) = movie_id {
        return Some(MatchResult {
            entity_type: "movie".to_string(),
            entity_id: movie_id,
            method: "title".to_string(),
        });
    }

    None
}

/// Find a show by name (case-insensitive, fuzzy)
async fn find_show_by_name(pool: &Pool<Sqlite>, name: &str) -> Option<i64> {
    let normalized = normalize_title(name);

    // Try exact match first (case-insensitive)
    let exact: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM shows WHERE LOWER(name) = LOWER(?)"
    )
    .bind(name)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    if exact.is_some() {
        return exact;
    }

    // Try normalized match
    let rows = sqlx::query("SELECT id, name FROM shows")
        .fetch_all(pool)
        .await
        .ok()?;

    for row in rows {
        let show_name: String = row.get("name");
        if normalize_title(&show_name) == normalized {
            return Some(row.get("id"));
        }
    }

    // Try partial match (show name contains Plex title or vice versa)
    let lower_name = name.to_lowercase();
    for row in sqlx::query("SELECT id, name FROM shows")
        .fetch_all(pool)
        .await
        .ok()?
    {
        let show_name: String = row.get("name");
        let lower_show = show_name.to_lowercase();

        if lower_show.contains(&lower_name) || lower_name.contains(&lower_show) {
            return Some(row.get("id"));
        }
    }

    None
}

/// Find a movie by title and optionally year
async fn find_movie_by_title(pool: &Pool<Sqlite>, title: &str, year: Option<i32>) -> Option<i64> {
    let normalized = normalize_title(title);

    // Try exact match with year first
    if let Some(year) = year {
        let exact: Option<i64> = sqlx::query_scalar(
            "SELECT id FROM movies WHERE LOWER(title) = LOWER(?) AND strftime('%Y', release_date) = ?"
        )
        .bind(title)
        .bind(year.to_string())
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

        if exact.is_some() {
            return exact;
        }
    }

    // Try exact title match without year
    let exact: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM movies WHERE LOWER(title) = LOWER(?)"
    )
    .bind(title)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    if exact.is_some() {
        return exact;
    }

    // Try normalized match
    let rows = sqlx::query("SELECT id, title FROM movies")
        .fetch_all(pool)
        .await
        .ok()?;

    for row in rows {
        let movie_title: String = row.get("title");
        if normalize_title(&movie_title) == normalized {
            return Some(row.get("id"));
        }
    }

    None
}

/// Normalize a title for fuzzy matching
fn normalize_title(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Mark an episode as watched
pub async fn mark_episode_watched(pool: &Pool<Sqlite>, episode_id: i64) -> Result<(), String> {
    sqlx::query(
        "UPDATE episodes SET watched = 1, watched_at = datetime('now') WHERE id = ?"
    )
    .bind(episode_id)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to mark episode watched: {}", e))?;

    Ok(())
}

/// Mark a movie as watched
pub async fn mark_movie_watched(pool: &Pool<Sqlite>, movie_id: i64) -> Result<(), String> {
    sqlx::query(
        "UPDATE movies SET watched = 1, watched_at = datetime('now') WHERE id = ?"
    )
    .bind(movie_id)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to mark movie watched: {}", e))?;

    Ok(())
}
