use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::AppHandle;

use crate::db::connection;

/// Export data structure
#[derive(Debug, Serialize, Deserialize)]
pub struct BackupData {
    pub version: String,
    pub exported_at: String,
    pub shows: Vec<ShowBackup>,
    pub episodes: Vec<EpisodeBackup>,
    pub movies: Vec<MovieBackup>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShowBackup {
    pub id: i64,
    pub name: String,
    pub slug: Option<String>,
    pub status: Option<String>,
    pub poster_url: Option<String>,
    pub first_aired: Option<String>,
    pub network: Option<String>,
    pub overview: Option<String>,
    pub airs_time: Option<String>,
    pub airs_days: Option<String>,
    pub runtime: Option<i32>,
    pub added_at: Option<String>,
    pub last_synced: Option<String>,
    pub color: Option<String>,
    pub notes: Option<String>,
    pub tags: Option<String>,
    pub archived: i32,
    pub rating: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EpisodeBackup {
    pub id: i64,
    pub show_id: i64,
    pub season_number: Option<i32>,
    pub episode_number: Option<i32>,
    pub name: Option<String>,
    pub overview: Option<String>,
    pub aired: Option<String>,
    pub runtime: Option<i32>,
    pub image_url: Option<String>,
    pub watched: i32,
    pub watched_at: Option<String>,
    pub scheduled_date: Option<String>,
    pub rating: Option<f64>,
    pub tags: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MovieBackup {
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
    pub watched: i32,
    pub watched_at: Option<String>,
    pub rating: Option<f64>,
    pub notes: Option<String>,
    pub color: Option<String>,
    pub tags: Option<String>,
    pub archived: i32,
    pub added_at: Option<String>,
    pub last_synced: Option<String>,
}

/// Export all user data to JSON
#[tauri::command]
pub async fn export_database(app: AppHandle) -> Result<BackupData, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    // Export shows
    let show_rows = sqlx::query(
        r#"SELECT id, name, slug, status, poster_url, first_aired, network, overview,
                  airs_time, airs_days, runtime, added_at, last_synced, color, notes, tags,
                  COALESCE(archived, 0) as archived, rating
           FROM shows"#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to export shows: {}", e))?;

    let shows: Vec<ShowBackup> = show_rows
        .into_iter()
        .map(|row| ShowBackup {
            id: row.get("id"),
            name: row.get("name"),
            slug: row.get("slug"),
            status: row.get("status"),
            poster_url: row.get("poster_url"),
            first_aired: row.get("first_aired"),
            network: row.get("network"),
            overview: row.get("overview"),
            airs_time: row.get("airs_time"),
            airs_days: row.get("airs_days"),
            runtime: row.get("runtime"),
            added_at: row.get("added_at"),
            last_synced: row.get("last_synced"),
            color: row.get("color"),
            notes: row.get("notes"),
            tags: row.get("tags"),
            archived: row.get("archived"),
            rating: row.get::<Option<f64>, _>("rating"),
        })
        .collect();

    // Export episodes
    let episode_rows = sqlx::query(
        r#"SELECT id, show_id, season_number, episode_number, name, overview, aired,
                  runtime, image_url, watched, watched_at, scheduled_date, rating, tags
           FROM episodes"#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to export episodes: {}", e))?;

    let episodes: Vec<EpisodeBackup> = episode_rows
        .into_iter()
        .map(|row| EpisodeBackup {
            id: row.get("id"),
            show_id: row.get("show_id"),
            season_number: row.get("season_number"),
            episode_number: row.get("episode_number"),
            name: row.get("name"),
            overview: row.get("overview"),
            aired: row.get("aired"),
            runtime: row.get("runtime"),
            image_url: row.get("image_url"),
            watched: row.get("watched"),
            watched_at: row.get("watched_at"),
            scheduled_date: row.get("scheduled_date"),
            rating: row.get::<Option<f64>, _>("rating"),
            tags: row.get("tags"),
        })
        .collect();

    // Export movies
    let movie_rows = sqlx::query(
        r#"SELECT id, title, tagline, overview, poster_url, backdrop_url, release_date,
                  digital_release_date, physical_release_date, runtime, status, genres,
                  vote_average, scheduled_date, watched, watched_at, rating, notes, color,
                  tags, COALESCE(archived, 0) as archived, added_at, last_synced
           FROM movies"#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to export movies: {}", e))?;

    let movies: Vec<MovieBackup> = movie_rows
        .into_iter()
        .map(|row| MovieBackup {
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
            watched: row.get("watched"),
            watched_at: row.get("watched_at"),
            rating: row.get::<Option<f64>, _>("rating"),
            notes: row.get("notes"),
            color: row.get("color"),
            tags: row.get("tags"),
            archived: row.get("archived"),
            added_at: row.get("added_at"),
            last_synced: row.get("last_synced"),
        })
        .collect();

    Ok(BackupData {
        version: "1.0".to_string(),
        exported_at: chrono::Utc::now().to_rfc3339(),
        shows,
        episodes,
        movies,
    })
}

/// Import data from JSON backup (replaces existing data)
#[tauri::command]
pub async fn import_database(app: AppHandle, data: BackupData) -> Result<ImportResult, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    // Clear existing data (in correct order due to foreign keys)
    if let Err(e) = sqlx::query("DELETE FROM episodes")
        .execute(&mut *tx)
        .await
    {
        let _ = tx.rollback().await;
        return Err(format!("Failed to clear episodes: {}", e));
    }

    if let Err(e) = sqlx::query("DELETE FROM shows")
        .execute(&mut *tx)
        .await
    {
        let _ = tx.rollback().await;
        return Err(format!("Failed to clear shows: {}", e));
    }

    if let Err(e) = sqlx::query("DELETE FROM movies")
        .execute(&mut *tx)
        .await
    {
        let _ = tx.rollback().await;
        return Err(format!("Failed to clear movies: {}", e));
    }

    // Import shows
    for show in &data.shows {
        if let Err(e) = sqlx::query(
            r#"INSERT INTO shows (id, name, slug, status, poster_url, first_aired, network,
                                  overview, airs_time, airs_days, runtime, added_at, last_synced,
                                  color, notes, tags, archived, rating)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(show.id)
        .bind(&show.name)
        .bind(&show.slug)
        .bind(&show.status)
        .bind(&show.poster_url)
        .bind(&show.first_aired)
        .bind(&show.network)
        .bind(&show.overview)
        .bind(&show.airs_time)
        .bind(&show.airs_days)
        .bind(show.runtime)
        .bind(&show.added_at)
        .bind(&show.last_synced)
        .bind(&show.color)
        .bind(&show.notes)
        .bind(&show.tags)
        .bind(show.archived)
        .bind(show.rating)
        .execute(&mut *tx)
        .await
        {
            let _ = tx.rollback().await;
            return Err(format!("Failed to import show {}: {}", show.name, e));
        }
    }

    // Import episodes
    for episode in &data.episodes {
        if let Err(e) = sqlx::query(
            r#"INSERT INTO episodes (id, show_id, season_number, episode_number, name, overview,
                                     aired, runtime, image_url, watched, watched_at, scheduled_date,
                                     rating, tags)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(episode.id)
        .bind(episode.show_id)
        .bind(episode.season_number)
        .bind(episode.episode_number)
        .bind(&episode.name)
        .bind(&episode.overview)
        .bind(&episode.aired)
        .bind(episode.runtime)
        .bind(&episode.image_url)
        .bind(episode.watched)
        .bind(&episode.watched_at)
        .bind(&episode.scheduled_date)
        .bind(episode.rating)
        .bind(&episode.tags)
        .execute(&mut *tx)
        .await
        {
            let _ = tx.rollback().await;
            return Err(format!("Failed to import episode: {}", e));
        }
    }

    // Import movies
    for movie in &data.movies {
        if let Err(e) = sqlx::query(
            r#"INSERT INTO movies (id, title, tagline, overview, poster_url, backdrop_url,
                                   release_date, digital_release_date, physical_release_date,
                                   runtime, status, genres, vote_average, scheduled_date, watched,
                                   watched_at, rating, notes, color, tags, archived, added_at, last_synced)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(movie.id)
        .bind(&movie.title)
        .bind(&movie.tagline)
        .bind(&movie.overview)
        .bind(&movie.poster_url)
        .bind(&movie.backdrop_url)
        .bind(&movie.release_date)
        .bind(&movie.digital_release_date)
        .bind(&movie.physical_release_date)
        .bind(movie.runtime)
        .bind(&movie.status)
        .bind(&movie.genres)
        .bind(movie.vote_average)
        .bind(&movie.scheduled_date)
        .bind(movie.watched)
        .bind(&movie.watched_at)
        .bind(movie.rating)
        .bind(&movie.notes)
        .bind(&movie.color)
        .bind(&movie.tags)
        .bind(movie.archived)
        .bind(&movie.added_at)
        .bind(&movie.last_synced)
        .execute(&mut *tx)
        .await
        {
            let _ = tx.rollback().await;
            return Err(format!("Failed to import movie {}: {}", movie.title, e));
        }
    }

    tx.commit()
        .await
        .map_err(|e| format!("Failed to commit transaction: {}", e))?;

    Ok(ImportResult {
        shows_imported: data.shows.len() as u32,
        episodes_imported: data.episodes.len() as u32,
        movies_imported: data.movies.len() as u32,
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportResult {
    pub shows_imported: u32,
    pub episodes_imported: u32,
    pub movies_imported: u32,
}
