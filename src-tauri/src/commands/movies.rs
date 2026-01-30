use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row};
use tauri::AppHandle;
use crate::db::connection;
use crate::tmdb;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct TrackedMovie {
    pub id: i64,
    pub title: String,
    pub tagline: Option<String>,
    pub poster_url: Option<String>,
    pub release_date: Option<String>,
    pub digital_release_date: Option<String>,
    pub runtime: Option<i32>,
    pub status: Option<String>,
    pub scheduled_date: Option<String>,
    pub watched: bool,
    pub rating: Option<f64>,
    pub color: Option<String>,
    pub archived: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MovieSearchResult {
    pub id: i64,
    pub title: String,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
    pub release_date: Option<String>,
    pub vote_average: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CalendarMovie {
    pub id: i64,
    pub title: String,
    pub poster_url: Option<String>,
    pub runtime: Option<i32>,
    pub scheduled_date: Option<String>,
    pub digital_release_date: Option<String>,
    pub watched: bool,
    pub color: Option<String>,
}

#[tauri::command]
pub async fn search_movies(query: String) -> Result<Vec<MovieSearchResult>, String> {
    let results = tmdb::search_movies(&query)
        .await
        .map_err(|e| e.to_string())?;

    Ok(results
        .into_iter()
        .map(|r| {
            let poster_url = r.poster_url();
            MovieSearchResult {
                id: r.id,
                title: r.title,
                overview: r.overview,
                poster_url,
                release_date: r.release_date,
                vote_average: r.vote_average,
            }
        })
        .collect())
}

#[tauri::command]
pub async fn add_movie(app: AppHandle, id: i64) -> Result<(), String> {
    crate::commands::validation::validate_id(id)?;
    
    // Get movie details and release dates from TMDB
    let (movie_details, release_dates) = tmdb::get_movie_with_release_dates(id, "US")
        .await
        .map_err(|e| format!("Failed to fetch movie details: {}", e))?;

    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let genres_json = movie_details.genres_string();

    sqlx::query(
        r#"
        INSERT OR REPLACE INTO movies
        (id, title, tagline, overview, poster_url, backdrop_url, release_date,
         digital_release_date, physical_release_date, runtime, status, genres,
         vote_average, added_at, last_synced)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))
        "#,
    )
    .bind(movie_details.id)
    .bind(&movie_details.title)
    .bind(movie_details.tagline.as_ref())
    .bind(movie_details.overview.as_ref())
    .bind(movie_details.poster_url())
    .bind(movie_details.backdrop_url())
    .bind(release_dates.theatrical.as_ref())
    .bind(release_dates.digital.as_ref())
    .bind(release_dates.physical.as_ref())
    .bind(movie_details.runtime)
    .bind(movie_details.status.as_ref())
    .bind(genres_json.as_ref())
    .bind(movie_details.vote_average)
    .execute(&pool)
    .await
    .map_err(|e| format!("Failed to add movie: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn remove_movie(app: AppHandle, id: i64) -> Result<(), String> {
    crate::commands::validation::validate_id(id)?;
    
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    sqlx::query("DELETE FROM movies WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to remove movie: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn get_tracked_movies(app: AppHandle) -> Result<Vec<TrackedMovie>, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let rows = sqlx::query(
        r#"
        SELECT id, title, tagline, poster_url, release_date, digital_release_date,
               runtime, status, scheduled_date, watched, rating, color, archived
        FROM movies
        WHERE archived = 0
        ORDER BY title
        LIMIT 10000
        "#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to get tracked movies: {}", e))?;

    let movies: Vec<TrackedMovie> = rows
        .into_iter()
        .map(|row| TrackedMovie {
            id: row.get("id"),
            title: row.get("title"),
            tagline: row.get("tagline"),
            poster_url: row.get("poster_url"),
            release_date: row.get("release_date"),
            digital_release_date: row.get("digital_release_date"),
            runtime: row.get("runtime"),
            status: row.get("status"),
            scheduled_date: row.get("scheduled_date"),
            watched: row.get::<i32, _>("watched") == 1,
            rating: row.get("rating"),
            color: row.get("color"),
            archived: row.get::<i32, _>("archived") == 1,
        })
        .collect();

    Ok(movies)
}

#[tauri::command]
pub async fn get_archived_movies(app: AppHandle) -> Result<Vec<TrackedMovie>, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let rows = sqlx::query(
        r#"
        SELECT id, title, tagline, poster_url, release_date, digital_release_date,
               runtime, status, scheduled_date, watched, rating, color, archived
        FROM movies
        WHERE archived = 1
        ORDER BY title
        LIMIT 10000
        "#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to get archived movies: {}", e))?;

    let movies: Vec<TrackedMovie> = rows
        .into_iter()
        .map(|row| TrackedMovie {
            id: row.get("id"),
            title: row.get("title"),
            tagline: row.get("tagline"),
            poster_url: row.get("poster_url"),
            release_date: row.get("release_date"),
            digital_release_date: row.get("digital_release_date"),
            runtime: row.get("runtime"),
            status: row.get("status"),
            scheduled_date: row.get("scheduled_date"),
            watched: row.get::<i32, _>("watched") == 1,
            rating: row.get("rating"),
            color: row.get("color"),
            archived: row.get::<i32, _>("archived") == 1,
        })
        .collect();

    Ok(movies)
}

#[tauri::command]
pub async fn update_movie_rating(
    app: AppHandle,
    id: i64,
    rating: Option<f64>,
) -> Result<(), String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    sqlx::query("UPDATE movies SET rating = ? WHERE id = ?")
        .bind(rating)
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to update movie rating: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn mark_movie_watched(
    app: AppHandle,
    id: i64,
    watched: bool,
) -> Result<(), String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    // Use parameterized queries instead of format!() for safety
    if watched {
        sqlx::query(r#"UPDATE movies SET watched = ?, watched_at = datetime('now') WHERE id = ?"#)
            .bind(1)
            .bind(id)
            .execute(&pool)
            .await
            .map_err(|e| format!("Failed to mark movie watched: {}", e))?;
    } else {
        sqlx::query(r#"UPDATE movies SET watched = ?, watched_at = NULL WHERE id = ?"#)
            .bind(0)
            .bind(id)
            .execute(&pool)
            .await
            .map_err(|e| format!("Failed to mark movie watched: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub async fn schedule_movie(
    app: AppHandle,
    id: i64,
    date: String,
) -> Result<(), String> {
    crate::commands::validation::validate_id(id)?;
    crate::commands::validation::validate_date(&date)?;
    
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    sqlx::query("UPDATE movies SET scheduled_date = ? WHERE id = ?")
        .bind(&date)
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to schedule movie: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn unschedule_movie(app: AppHandle, id: i64) -> Result<(), String> {
    crate::commands::validation::validate_id(id)?;
    
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    sqlx::query("UPDATE movies SET scheduled_date = NULL WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to unschedule movie: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn archive_movie(app: AppHandle, id: i64) -> Result<(), String> {
    crate::commands::validation::validate_id(id)?;
    
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    sqlx::query("UPDATE movies SET archived = 1 WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to archive movie: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn unarchive_movie(app: AppHandle, id: i64) -> Result<(), String> {
    crate::commands::validation::validate_id(id)?;
    
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    sqlx::query("UPDATE movies SET archived = 0 WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to unarchive movie: {}", e))?;

    Ok(())
}

/// Helper function to sync a single movie (takes reference to avoid cloning)
async fn sync_movie_ref(app: &AppHandle, id: i64) -> Result<(), String> {
    sync_movie(app.clone(), id).await
}

#[tauri::command]
pub async fn sync_movie(app: AppHandle, id: i64) -> Result<(), String> {
    // Get updated movie details from TMDB
    let (movie_details, release_dates) = tmdb::get_movie_with_release_dates(id, "US")
        .await
        .map_err(|e| format!("Failed to fetch movie details: {}", e))?;

    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let genres_json = movie_details.genres_string();

    // Update movie data (preserving user data like watched, rating, scheduled_date, etc.)
    sqlx::query(
        r#"
        UPDATE movies SET
            title = ?,
            tagline = ?,
            overview = ?,
            poster_url = ?,
            backdrop_url = ?,
            release_date = ?,
            digital_release_date = ?,
            physical_release_date = ?,
            runtime = ?,
            status = ?,
            genres = ?,
            vote_average = ?,
            last_synced = datetime('now')
        WHERE id = ?
        "#,
    )
    .bind(&movie_details.title)
    .bind(movie_details.tagline.as_ref())
    .bind(movie_details.overview.as_ref())
    .bind(movie_details.poster_url())
    .bind(movie_details.backdrop_url())
    .bind(release_dates.theatrical.as_ref())
    .bind(release_dates.digital.as_ref())
    .bind(release_dates.physical.as_ref())
    .bind(movie_details.runtime)
    .bind(movie_details.status.as_ref())
    .bind(genres_json.as_ref())
    .bind(movie_details.vote_average)
    .bind(id)
    .execute(&pool)
    .await
    .map_err(|e| format!("Failed to sync movie: {}", e))?;

    Ok(())
}

/// Sync all tracked movies - fetches fresh data from TMDB for all movies
#[tauri::command]
pub async fn sync_all_movies(app: AppHandle) -> Result<u32, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    // Get all tracked movie IDs
    let movie_ids: Vec<i64> = sqlx::query_scalar("SELECT id FROM movies")
        .fetch_all(&pool)
        .await
        .map_err(|e| format!("Failed to get movies: {}", e))?;

    let mut synced = 0u32;
    let mut errors: Vec<String> = Vec::new();

    // Use reference to app to avoid cloning in loop
    for movie_id in movie_ids {
        match sync_movie_ref(&app, movie_id).await {
            Ok(_) => synced += 1,
            Err(e) => {
                let error_msg = format!("Movie {}: {}", movie_id, e);
                eprintln!("Failed to sync movie {}: {}", movie_id, e);
                errors.push(error_msg);
            }
        }
    }

    // Log errors if any occurred (for debugging - return value remains compatible)
    if !errors.is_empty() {
        eprintln!("[sync_all_movies] {} movies failed to sync: {:?}", errors.len(), errors);
    }

    Ok(synced)
}

#[tauri::command]
pub async fn get_movies_for_range(
    app: AppHandle,
    start_date: String,
    end_date: String,
) -> Result<Vec<CalendarMovie>, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    // Get movies where scheduled_date falls in range (only manually scheduled movies appear on calendar)
    let rows = sqlx::query(
        r#"
        SELECT id, title, poster_url, runtime, scheduled_date, digital_release_date, watched, color
        FROM movies
        WHERE archived = 0
          AND scheduled_date >= ? AND scheduled_date <= ?
        ORDER BY scheduled_date, title
        LIMIT 10000
        "#,
    )
    .bind(&start_date)
    .bind(&end_date)
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to get movies for range: {}", e))?;

    let movies: Vec<CalendarMovie> = rows
        .into_iter()
        .map(|row| CalendarMovie {
            id: row.get("id"),
            title: row.get("title"),
            poster_url: row.get("poster_url"),
            runtime: row.get("runtime"),
            scheduled_date: row.get("scheduled_date"),
            digital_release_date: row.get("digital_release_date"),
            watched: row.get::<i32, _>("watched") == 1,
            color: row.get("color"),
        })
        .collect();

    Ok(movies)
}

#[tauri::command]
pub async fn get_movie_details(app: AppHandle, id: i64) -> Result<MovieDetail, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let row = sqlx::query(
        r#"
        SELECT id, title, tagline, overview, poster_url, backdrop_url,
               release_date, digital_release_date, physical_release_date,
               runtime, status, genres, vote_average, scheduled_date,
               watched, watched_at, rating, notes, color, tags, archived,
               added_at, last_synced
        FROM movies
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| format!("Failed to get movie details: {}", e))?;

    match row {
        Some(row) => Ok(MovieDetail {
            id: row.get("id"),
            title: row.get("title"),
            tagline: row.get("tagline"),
            overview: row.get("overview"),
            poster_url: row.get("poster_url"),
            backdrop_url: row.get("backdrop_url"),
            release_date: row.get("release_date"),
            digital_release_date: row.get("digital_release_date"),
            physical_release_date: row.get("physical_release_date"),
            runtime: row.get("runtime"),
            status: row.get("status"),
            genres: row.get("genres"),
            vote_average: row.get("vote_average"),
            scheduled_date: row.get("scheduled_date"),
            watched: row.get::<i32, _>("watched") == 1,
            watched_at: row.get("watched_at"),
            rating: row.get("rating"),
            notes: row.get("notes"),
            color: row.get("color"),
            tags: row.get("tags"),
            archived: row.get::<i32, _>("archived") == 1,
            added_at: row.get("added_at"),
            last_synced: row.get("last_synced"),
        }),
        None => Err("Movie not found".to_string()),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MovieDetail {
    pub id: i64,
    pub title: String,
    pub tagline: Option<String>,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub release_date: Option<String>,
    pub digital_release_date: Option<String>,
    pub physical_release_date: Option<String>,
    pub runtime: Option<i32>,
    pub status: Option<String>,
    pub genres: Option<String>,
    pub vote_average: Option<f64>,
    pub scheduled_date: Option<String>,
    pub watched: bool,
    pub watched_at: Option<String>,
    pub rating: Option<f64>,
    pub notes: Option<String>,
    pub color: Option<String>,
    pub tags: Option<String>,
    pub archived: bool,
    pub added_at: Option<String>,
    pub last_synced: Option<String>,
}
