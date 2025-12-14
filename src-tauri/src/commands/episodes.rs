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

    sqlx::query(
        r#"
        UPDATE episodes
        SET watched = ?
        WHERE id = ?
        "#,
    )
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
    // Get episodes from TVDB
    let episodes = crate::tvdb::get_series_episodes(show_id)
        .await
        .map_err(|e| format!("Failed to fetch episodes: {}", e))?;

    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    // Batch insert/update episodes
    let mut tx = pool.begin().await
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    for episode in episodes {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO episodes
            (id, show_id, season_number, episode_number, name, overview, aired, runtime, image_url)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
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
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to sync episode: {}", e))?;
    }

    // Update last_synced timestamp
    sqlx::query("UPDATE shows SET last_synced = datetime('now') WHERE id = ?")
        .bind(show_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to update last_synced: {}", e))?;

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


