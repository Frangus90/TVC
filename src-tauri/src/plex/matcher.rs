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
        "SELECT tvc_id FROM title_mappings WHERE plex_title = ? AND media_type = 'show'",
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
                method: if mapped_show_id.is_some() {
                    "mapping"
                } else {
                    "title"
                }
                .to_string(),
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
    tmdb_id: Option<i64>,
) -> Option<MatchResult> {
    // First check title_mappings
    let mapped_movie_id: Option<i64> = sqlx::query_scalar(
        "SELECT t.tvc_id FROM title_mappings t JOIN movies m ON m.id = t.tvc_id WHERE t.plex_title = ? AND t.media_type = 'movie'"
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

    if let Some(id) = tmdb_id {
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM movies WHERE id = ?)")
            .bind(id)
            .fetch_one(pool)
            .await
            .ok()?;
        // A known provider identity must never fall back to a different movie.
        return exists.then(|| MatchResult {
            entity_type: "movie".into(),
            entity_id: id,
            method: "tmdb".into(),
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

/// Only a unique normalized title is safe. Substring matches can select remakes/spinoffs.
async fn find_show_by_name(pool: &Pool<Sqlite>, name: &str) -> Option<i64> {
    let normalized = normalize_title(name);
    let rows = sqlx::query("SELECT id, name FROM shows")
        .fetch_all(pool)
        .await
        .ok()?;
    let ids: Vec<i64> = rows
        .iter()
        .filter(|r| normalize_title(r.get("name")) == normalized)
        .map(|r| r.get("id"))
        .collect();
    if ids.len() == 1 {
        Some(ids[0])
    } else {
        None
    }
}

async fn find_movie_by_title(pool: &Pool<Sqlite>, title: &str, year: Option<i32>) -> Option<i64> {
    let normalized = normalize_title(title);
    let rows = sqlx::query(
        "SELECT id, title, CAST(strftime('%Y', release_date) AS INTEGER) AS year FROM movies",
    )
    .fetch_all(pool)
    .await
    .ok()?;
    let ids: Vec<i64> = rows
        .iter()
        .filter(|r| {
            normalize_title(r.get("title")) == normalized
                && year.map_or(true, |y| r.get::<Option<i32>, _>("year") == Some(y))
        })
        .map(|r| r.get("id"))
        .collect();
    if ids.len() == 1 {
        Some(ids[0])
    } else {
        None
    }
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

pub async fn mark_episode_watched(pool: &Pool<Sqlite>, episode_id: i64) -> Result<(), String> {
    crate::watch_history::episodes(pool, crate::watch_history::Episodes::One(episode_id), true)
        .await
}

pub async fn mark_movie_watched(pool: &Pool<Sqlite>, movie_id: i64) -> Result<(), String> {
    crate::watch_history::movie(pool, movie_id, true).await
}
