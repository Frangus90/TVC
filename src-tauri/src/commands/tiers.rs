use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row};
use tauri::AppHandle;
use crate::db::connection;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Tier {
    pub id: i64,
    pub position: i64,
    pub name: String,
    pub color: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteTierResult {
    pub affected_shows: i64,
    pub affected_movies: i64,
}

// ============================================
// Tier CRUD commands
// ============================================

#[tauri::command]
pub async fn get_tiers(app: AppHandle) -> Result<Vec<Tier>, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let tiers = sqlx::query_as::<_, Tier>(
        "SELECT id, position, name, color, created_at FROM tiers ORDER BY position DESC"
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to get tiers: {}", e))?;

    Ok(tiers)
}

#[tauri::command]
pub async fn create_tier(
    app: AppHandle,
    name: String,
    color: String,
) -> Result<Tier, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    // New tier gets position 0 (bottom). User reorders after.
    let result = sqlx::query(
        "INSERT INTO tiers (position, name, color) VALUES (0, ?, ?) RETURNING id, position, name, color, created_at"
    )
    .bind(&name)
    .bind(&color)
    .fetch_one(&pool)
    .await
    .map_err(|e| format!("Failed to create tier: {}", e))?;

    Ok(Tier {
        id: result.get("id"),
        position: result.get("position"),
        name: result.get("name"),
        color: result.get("color"),
        created_at: result.get("created_at"),
    })
}

#[tauri::command]
pub async fn update_tier(
    app: AppHandle,
    id: i64,
    name: Option<String>,
    color: Option<String>,
) -> Result<(), String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    if let Some(name) = name {
        sqlx::query("UPDATE tiers SET name = ? WHERE id = ?")
            .bind(&name)
            .bind(id)
            .execute(&pool)
            .await
            .map_err(|e| format!("Failed to update tier name: {}", e))?;
    }

    if let Some(color) = color {
        sqlx::query("UPDATE tiers SET color = ? WHERE id = ?")
            .bind(&color)
            .bind(id)
            .execute(&pool)
            .await
            .map_err(|e| format!("Failed to update tier color: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub async fn delete_tier(
    app: AppHandle,
    id: i64,
) -> Result<DeleteTierResult, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let mut tx = pool.begin().await
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    // Count affected items and delete within the same transaction
    let affected_shows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM shows WHERE tier_id = ?"
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| format!("Failed to count affected shows: {}", e))?;

    let affected_movies: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM movies WHERE tier_id = ?"
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| format!("Failed to count affected movies: {}", e))?;

    // Delete the tier (ON DELETE SET NULL will unrate items)
    sqlx::query("DELETE FROM tiers WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to delete tier: {}", e))?;

    tx.commit().await
        .map_err(|e| format!("Failed to commit tier deletion: {}", e))?;

    Ok(DeleteTierResult {
        affected_shows,
        affected_movies,
    })
}

#[tauri::command]
pub async fn reorder_tiers(
    app: AppHandle,
    tier_ids: Vec<i64>,
) -> Result<(), String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let mut tx = pool.begin().await
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    // Assign positions: first in vec = highest position (best tier)
    let total = tier_ids.len() as i64;
    for (i, tier_id) in tier_ids.iter().enumerate() {
        let position = total - i as i64;
        sqlx::query("UPDATE tiers SET position = ? WHERE id = ?")
            .bind(position)
            .bind(tier_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to reorder tier: {}", e))?;
    }

    tx.commit().await
        .map_err(|e| format!("Failed to commit tier reorder: {}", e))?;

    Ok(())
}

// ============================================
// Tier preset commands
// ============================================

#[tauri::command]
pub async fn get_tier_preset(app: AppHandle) -> Result<String, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let preset: Option<String> = sqlx::query_scalar(
        "SELECT value FROM settings WHERE key = 'tier_preset'"
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| format!("Failed to get tier preset: {}", e))?;

    Ok(preset.unwrap_or_else(|| "10-star".to_string()))
}

#[tauri::command]
pub async fn apply_tier_preset(
    app: AppHandle,
    preset: String,
) -> Result<(), String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let tiers_data: Vec<(&str, i64)> = match preset.as_str() {
        "10-star" => vec![
            ("Masterpiece", 10), ("Excellent", 9), ("Great", 8),
            ("Good", 7), ("Solid", 6), ("Average", 5),
            ("Below Average", 4), ("Poor", 3), ("Bad", 2), ("Terrible", 1),
        ],
        "5-star" => vec![
            ("Excellent", 5), ("Great", 4), ("Good", 3),
            ("Average", 2), ("Poor", 1),
        ],
        _ => return Err(format!("Unknown preset: {}", preset)),
    };

    let mut tx = pool.begin().await
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    // Unrate all items first
    sqlx::query("UPDATE shows SET tier_id = NULL WHERE tier_id IS NOT NULL")
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to unrate shows: {}", e))?;

    sqlx::query("UPDATE movies SET tier_id = NULL WHERE tier_id IS NOT NULL")
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to unrate movies: {}", e))?;

    // Clear existing tiers
    sqlx::query("DELETE FROM tiers")
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to clear tiers: {}", e))?;

    // Insert new tiers
    for (name, position) in tiers_data {
        sqlx::query("INSERT INTO tiers (position, name, color) VALUES (?, ?, '')")
            .bind(position)
            .bind(name)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to insert tier: {}", e))?;
    }

    // Save preset preference
    sqlx::query("INSERT OR REPLACE INTO settings (key, value) VALUES ('tier_preset', ?)")
        .bind(&preset)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to save preset: {}", e))?;

    tx.commit().await
        .map_err(|e| format!("Failed to commit preset change: {}", e))?;

    Ok(())
}

// ============================================
// Tier list item commands
// ============================================

#[derive(Debug, Serialize, Deserialize)]
pub struct TierListShow {
    pub id: i64,
    pub name: String,
    pub poster_url: Option<String>,
    pub tier_id: Option<i64>,
    pub rank_order: Option<i32>,
    pub tier_only: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TierListMovie {
    pub id: i64,
    pub title: String,
    pub poster_url: Option<String>,
    pub tier_id: Option<i64>,
    pub rank_order: Option<i32>,
    pub tier_only: bool,
}

#[tauri::command]
pub async fn get_tier_list_shows(app: AppHandle) -> Result<Vec<TierListShow>, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let rows = sqlx::query(
        r#"
        SELECT id, name, poster_url, tier_id, rank_order, tier_only
        FROM shows
        WHERE tier_only = 1 OR tier_id IS NOT NULL
        ORDER BY name
        LIMIT 10000
        "#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to get tier list shows: {}", e))?;

    let shows: Vec<TierListShow> = rows
        .into_iter()
        .map(|row| TierListShow {
            id: row.get("id"),
            name: row.get("name"),
            poster_url: row.get("poster_url"),
            tier_id: row.get("tier_id"),
            rank_order: row.get("rank_order"),
            tier_only: row.get::<i32, _>("tier_only") == 1,
        })
        .collect();

    eprintln!("[tier-add] get_tier_list_shows returned {} rows", shows.len());

    Ok(shows)
}

#[tauri::command]
pub async fn get_tier_list_movies(app: AppHandle) -> Result<Vec<TierListMovie>, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let rows = sqlx::query(
        r#"
        SELECT id, title, poster_url, tier_id, rank_order, tier_only
        FROM movies
        WHERE tier_only = 1 OR tier_id IS NOT NULL
        ORDER BY title
        LIMIT 10000
        "#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to get tier list movies: {}", e))?;

    let movies: Vec<TierListMovie> = rows
        .into_iter()
        .map(|row| TierListMovie {
            id: row.get("id"),
            title: row.get("title"),
            poster_url: row.get("poster_url"),
            tier_id: row.get("tier_id"),
            rank_order: row.get("rank_order"),
            tier_only: row.get::<i32, _>("tier_only") == 1,
        })
        .collect();

    eprintln!("[tier-add] get_tier_list_movies returned {} rows", movies.len());

    Ok(movies)
}

// ============================================
// Tier-only add/promote/demote commands
// ============================================

#[tauri::command]
pub async fn add_show_tier_only(
    app: AppHandle,
    id: i64,
    tier_id: Option<i64>,
) -> Result<(), String> {
    eprintln!("[tier-add] add_show_tier_only id={} tier_id={:?}", id, tier_id);
    crate::commands::validation::validate_id(id)?;

    let show_details = crate::tmdb::get_tv_details(id)
        .await
        .map_err(|e| {
            eprintln!("[tier-add] TMDB fetch failed for show id={}: {}", id, e);
            format!("Failed to fetch show details: {}", e)
        })?;
    eprintln!("[tier-add] TMDB ok: id={} name={:?}", show_details.id, show_details.name);

    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let result = sqlx::query(
        r#"
        INSERT INTO shows
        (id, name, status, poster_url, first_aired, overview, network, runtime, added_at, tier_only, tier_id)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, datetime('now'), 1, ?)
        ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            status = excluded.status,
            poster_url = excluded.poster_url,
            first_aired = excluded.first_aired,
            overview = excluded.overview,
            network = excluded.network,
            runtime = excluded.runtime,
            tier_id = excluded.tier_id
        "#,
    )
    .bind(show_details.id)
    .bind(&show_details.name)
    .bind(show_details.status.as_ref())
    .bind(show_details.poster_url())
    .bind(show_details.first_air_date.as_ref())
    .bind(show_details.overview.as_ref())
    .bind(show_details.network_name())
    .bind(show_details.runtime())
    .bind(tier_id)
    .execute(&pool)
    .await
    .map_err(|e| {
        eprintln!("[tier-add] INSERT failed for show id={}: {}", id, e);
        format!("Failed to add show to tier list: {}", e)
    })?;
    eprintln!(
        "[tier-add] INSERT ok: id={} rows_affected={}",
        id,
        result.rows_affected()
    );

    // Echo the row back so we can confirm what landed in the DB.
    if let Ok(row) = sqlx::query(
        "SELECT tier_only, tier_id, archived FROM shows WHERE id = ?",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    {
        let tier_only: i32 = row.get("tier_only");
        let resolved_tier_id: Option<i64> = row.get("tier_id");
        let archived: Option<i32> = row.try_get("archived").ok();
        eprintln!(
            "[tier-add] row state: id={} tier_only={} tier_id={:?} archived={:?}",
            id, tier_only, resolved_tier_id, archived
        );
    }

    Ok(())
}

#[tauri::command]
pub async fn add_movie_tier_only(
    app: AppHandle,
    id: i64,
    tier_id: Option<i64>,
) -> Result<(), String> {
    eprintln!("[tier-add] add_movie_tier_only id={} tier_id={:?}", id, tier_id);
    crate::commands::validation::validate_id(id)?;

    let (movie_details, release_dates) = crate::tmdb::get_movie_with_release_dates(id, "US")
        .await
        .map_err(|e| {
            eprintln!("[tier-add] TMDB fetch failed for movie id={}: {}", id, e);
            format!("Failed to fetch movie details: {}", e)
        })?;
    eprintln!("[tier-add] TMDB ok: id={} title={:?}", movie_details.id, movie_details.title);

    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let genres_json = movie_details.genres_string();

    sqlx::query(
        r#"
        INSERT INTO movies
        (id, title, tagline, overview, poster_url, backdrop_url, release_date,
         digital_release_date, physical_release_date, runtime, status, genres,
         vote_average, added_at, last_synced, tier_only, tier_id)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'), 1, ?)
        ON CONFLICT(id) DO UPDATE SET
            title = excluded.title,
            tagline = excluded.tagline,
            overview = excluded.overview,
            poster_url = excluded.poster_url,
            backdrop_url = excluded.backdrop_url,
            release_date = excluded.release_date,
            digital_release_date = excluded.digital_release_date,
            physical_release_date = excluded.physical_release_date,
            runtime = excluded.runtime,
            status = excluded.status,
            genres = excluded.genres,
            vote_average = excluded.vote_average,
            last_synced = datetime('now'),
            tier_id = excluded.tier_id
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
    .bind(tier_id)
    .execute(&pool)
    .await
    .map_err(|e| {
        eprintln!("[tier-add] INSERT failed for movie id={}: {}", id, e);
        format!("Failed to add movie to tier list: {}", e)
    })?;
    eprintln!("[tier-add] INSERT ok for movie id={}", id);

    Ok(())
}

// Manual entries use negative IDs (positive IDs collide with TMDB).
// Microsecond timestamp gives ~10^6 IDs/sec headroom; on collision we walk
// down by 1 until INSERT succeeds. Caps retries to avoid pathological loops.
async fn insert_manual_show(
    pool: &sqlx::SqlitePool,
    title: &str,
    poster_url: Option<&str>,
    tier_id: Option<i64>,
) -> Result<i64, String> {
    let mut candidate = -(chrono::Utc::now().timestamp_micros());
    for _ in 0..1000 {
        let result = sqlx::query(
            r#"
            INSERT INTO shows (id, name, poster_url, added_at, tier_only, tier_id)
            VALUES (?, ?, ?, datetime('now'), 1, ?)
            "#,
        )
        .bind(candidate)
        .bind(title)
        .bind(poster_url)
        .bind(tier_id)
        .execute(pool)
        .await;

        match result {
            Ok(_) => return Ok(candidate),
            Err(sqlx::Error::Database(db_err)) if db_err.message().contains("UNIQUE") => {
                candidate -= 1;
            }
            Err(e) => return Err(format!("Failed to add manual show: {}", e)),
        }
    }
    Err("Failed to generate unique manual show ID after 1000 attempts".to_string())
}

async fn insert_manual_movie(
    pool: &sqlx::SqlitePool,
    title: &str,
    poster_url: Option<&str>,
    tier_id: Option<i64>,
) -> Result<i64, String> {
    let mut candidate = -(chrono::Utc::now().timestamp_micros());
    for _ in 0..1000 {
        let result = sqlx::query(
            r#"
            INSERT INTO movies (id, title, poster_url, added_at, tier_only, tier_id)
            VALUES (?, ?, ?, datetime('now'), 1, ?)
            "#,
        )
        .bind(candidate)
        .bind(title)
        .bind(poster_url)
        .bind(tier_id)
        .execute(pool)
        .await;

        match result {
            Ok(_) => return Ok(candidate),
            Err(sqlx::Error::Database(db_err)) if db_err.message().contains("UNIQUE") => {
                candidate -= 1;
            }
            Err(e) => return Err(format!("Failed to add manual movie: {}", e)),
        }
    }
    Err("Failed to generate unique manual movie ID after 1000 attempts".to_string())
}

#[tauri::command]
pub async fn add_manual_show(
    app: AppHandle,
    title: String,
    poster_url: Option<String>,
    tier_id: Option<i64>,
) -> Result<i64, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    insert_manual_show(&pool, &title, poster_url.as_deref(), tier_id).await
}

#[tauri::command]
pub async fn add_manual_movie(
    app: AppHandle,
    title: String,
    poster_url: Option<String>,
    tier_id: Option<i64>,
) -> Result<i64, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    insert_manual_movie(&pool, &title, poster_url.as_deref(), tier_id).await
}

#[tauri::command]
pub async fn promote_show_to_tracked(app: AppHandle, id: i64) -> Result<(), String> {
    if id < 0 {
        return Err("Cannot promote manual entries to tracked (no external data source)".to_string());
    }

    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    sqlx::query("UPDATE shows SET tier_only = 0 WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to promote show: {}", e))?;

    // Sync episodes for the newly tracked show
    crate::commands::episodes::sync_show_episodes(app, id).await?;

    Ok(())
}

#[tauri::command]
pub async fn promote_movie_to_tracked(app: AppHandle, id: i64) -> Result<(), String> {
    if id < 0 {
        return Err("Cannot promote manual entries to tracked (no external data source)".to_string());
    }

    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    sqlx::query("UPDATE movies SET tier_only = 0 WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to promote movie: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn demote_show_to_tier_only(app: AppHandle, id: i64) -> Result<(), String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    // Set tier_only flag
    sqlx::query("UPDATE shows SET tier_only = 1 WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to demote show: {}", e))?;

    // Clear scheduled episodes for this show
    sqlx::query("UPDATE episodes SET scheduled_date = NULL WHERE show_id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to clear episode schedules: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn demote_movie_to_tier_only(app: AppHandle, id: i64) -> Result<(), String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    sqlx::query("UPDATE movies SET tier_only = 1, scheduled_date = NULL WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to demote movie: {}", e))?;

    Ok(())
}

// ============================================
// Update tier assignment (replaces old rating commands)
// ============================================

#[tauri::command]
pub async fn update_show_tier(
    app: AppHandle,
    id: i64,
    tier_id: Option<i64>,
) -> Result<(), String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    sqlx::query("UPDATE shows SET tier_id = ?, rank_order = NULL WHERE id = ?")
        .bind(tier_id)
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to update show tier: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn update_movie_tier(
    app: AppHandle,
    id: i64,
    tier_id: Option<i64>,
) -> Result<(), String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    sqlx::query("UPDATE movies SET tier_id = ?, rank_order = NULL WHERE id = ?")
        .bind(tier_id)
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to update movie tier: {}", e))?;

    Ok(())
}

