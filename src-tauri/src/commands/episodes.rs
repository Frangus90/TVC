use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row};
use tauri::AppHandle;
use crate::db::connection;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Episode {
    pub id: i64,
    pub show_id: i64,
    #[sqlx(rename = "show_name")]
    pub show_name: String,
    pub season_number: i32,
    pub episode_number: i32,
    pub name: Option<String>,
    pub aired: Option<String>,
    pub scheduled_date: Option<String>,
    pub watched: bool,
    pub poster_url: Option<String>,
}

#[tauri::command]
pub async fn mark_episode_watched(
    app: AppHandle,
    episode_id: i64,
    watched: bool,
) -> Result<(), String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let watched_at = if watched { "datetime('now')" } else { "NULL" };

    sqlx::query(&format!(
        r#"
        UPDATE episodes
        SET watched = ?, watched_at = {}
        WHERE id = ?
        "#,
        watched_at
    ))
    .bind(if watched { 1 } else { 0 })
    .bind(episode_id)
    .execute(&pool)
    .await
    .map_err(|e| format!("Failed to mark episode watched: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn get_episodes_for_range(
    app: AppHandle,
    start_date: String,
    end_date: String,
) -> Result<Vec<Episode>, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let rows = sqlx::query(
        r#"
        SELECT 
            e.id,
            e.show_id,
            s.name as show_name,
            COALESCE(e.season_number, 0) as season_number,
            COALESCE(e.episode_number, 0) as episode_number,
            e.name,
            e.aired,
            e.scheduled_date,
            e.watched,
            s.poster_url
        FROM episodes e
        JOIN shows s ON e.show_id = s.id
        WHERE (e.aired >= ? AND e.aired <= ?) 
           OR (e.scheduled_date >= ? AND e.scheduled_date <= ?)
        ORDER BY COALESCE(e.scheduled_date, e.aired), s.name
        "#,
    )
    .bind(&start_date)
    .bind(&end_date)
    .bind(&start_date)
    .bind(&end_date)
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to get episodes for range: {}", e))?;

    let episodes: Vec<Episode> = rows
        .into_iter()
        .map(|row| Episode {
            id: row.get("id"),
            show_id: row.get("show_id"),
            show_name: row.get("show_name"),
            season_number: row.get("season_number"),
            episode_number: row.get("episode_number"),
            name: row.get("name"),
            aired: row.get("aired"),
            scheduled_date: row.get("scheduled_date"),
            watched: row.get::<i32, _>("watched") == 1,
            poster_url: row.get("poster_url"),
        })
        .collect();

    Ok(episodes)
}

#[tauri::command]
pub async fn sync_show_episodes(app: AppHandle, show_id: i64) -> Result<(), String> {
    // Fetch show details from TVDB
    let show_details = crate::tvdb::get_series_extended(show_id)
        .await
        .map_err(|e| format!("Failed to fetch show details: {}", e))?;

    // Get episodes from TVDB
    let episodes = crate::tvdb::get_series_episodes(show_id)
        .await
        .map_err(|e| format!("Failed to fetch episodes: {}", e))?;

    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let mut tx = pool.begin().await
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    // Update show metadata (preserves user data: color, notes, tags, rating)
    let airs_days_json = show_details.airs_days
        .as_ref()
        .and_then(|days| serde_json::to_string(days).ok());

    sqlx::query(
        r#"
        UPDATE shows SET
            name = ?,
            slug = ?,
            status = ?,
            poster_url = ?,
            first_aired = ?,
            overview = ?,
            airs_time = ?,
            airs_days = ?,
            runtime = ?,
            last_synced = datetime('now')
        WHERE id = ?
        "#,
    )
    .bind(&show_details.name)
    .bind(show_details.slug.as_ref())
    .bind(show_details.status.as_ref().and_then(|s| s.name.as_ref()))
    .bind(show_details.image.as_ref())
    .bind(show_details.first_aired.as_ref())
    .bind(show_details.overview.as_ref())
    .bind(show_details.airs_time.as_ref())
    .bind(airs_days_json.as_deref())
    .bind(show_details.average_runtime)
    .bind(show_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Failed to update show: {}", e))?;

    // Update episodes
    for episode in episodes {
        // Use INSERT ... ON CONFLICT to properly update existing episodes
        // - New episodes: insert with scheduled_date = aired
        // - Existing episodes: update metadata AND scheduled_date to new aired date
        // - Preserves: watched, watched_at, rating, tags (user data)
        sqlx::query(
            r#"
            INSERT INTO episodes
            (id, show_id, season_number, episode_number, name, overview, aired, runtime, image_url, scheduled_date)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                season_number = excluded.season_number,
                episode_number = excluded.episode_number,
                name = excluded.name,
                overview = excluded.overview,
                aired = excluded.aired,
                runtime = excluded.runtime,
                image_url = excluded.image_url,
                scheduled_date = excluded.aired
            "#,
        )
        .bind(episode.id)
        .bind(show_id)
        .bind(episode.season_number)
        .bind(episode.episode_number)
        .bind(episode.name.as_ref())
        .bind(episode.overview.as_ref())
        .bind(episode.aired.as_ref())
        .bind(episode.runtime)
        .bind(episode.image.as_ref())
        .bind(episode.aired.as_ref()) // scheduled_date = aired for new episodes
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to sync episode: {}", e))?;
    }

    tx.commit().await
        .map_err(|e| format!("Failed to commit transaction: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn schedule_episode(
    app: AppHandle,
    episode_id: i64,
    date: String,
) -> Result<(), String> {
    println!("[Backend] schedule_episode called: episode_id={}, date={}", episode_id, date);
    
    let pool = connection::get_pool(&app).await
        .map_err(|e| {
            let err_msg = format!("Database error: {}", e);
            println!("[Backend] schedule_episode database connection error: {}", err_msg);
            err_msg
        })?;

    println!("[Backend] Executing UPDATE query");
    let result = sqlx::query("UPDATE episodes SET scheduled_date = ? WHERE id = ?")
        .bind(&date)
        .bind(episode_id)
        .execute(&pool)
        .await
        .map_err(|e| {
            let err_msg = format!("Failed to schedule episode: {}", e);
            println!("[Backend] schedule_episode query error: {}", err_msg);
            err_msg
        })?;

    println!("[Backend] schedule_episode completed: rows_affected={}", result.rows_affected());
    Ok(())
}

#[tauri::command]
pub async fn unschedule_episode(app: AppHandle, episode_id: i64) -> Result<(), String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    sqlx::query("UPDATE episodes SET scheduled_date = NULL WHERE id = ?")
        .bind(episode_id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to unschedule episode: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn mark_season_watched(
    app: AppHandle,
    show_id: i64,
    season_number: i32,
    watched: bool,
) -> Result<(), String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let watched_at = if watched { "datetime('now')" } else { "NULL" };

    sqlx::query(&format!(
        r#"
        UPDATE episodes
        SET watched = ?, watched_at = {}
        WHERE show_id = ? AND season_number = ?
        "#,
        watched_at
    ))
    .bind(if watched { 1 } else { 0 })
    .bind(show_id)
    .bind(season_number)
    .execute(&pool)
    .await
    .map_err(|e| format!("Failed to mark season watched: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn mark_show_watched(
    app: AppHandle,
    show_id: i64,
    watched: bool,
) -> Result<(), String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let watched_at = if watched { "datetime('now')" } else { "NULL" };

    sqlx::query(&format!(
        r#"
        UPDATE episodes
        SET watched = ?, watched_at = {}
        WHERE show_id = ?
        "#,
        watched_at
    ))
    .bind(if watched { 1 } else { 0 })
    .bind(show_id)
    .execute(&pool)
    .await
    .map_err(|e| format!("Failed to mark show watched: {}", e))?;

    Ok(())
}


