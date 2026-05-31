use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row};
use std::collections::HashMap;
use tauri::AppHandle;
use crate::db::connection;
use crate::tmdb;

/// User-controlled per-episode state. Preserved across re-syncs by keying on
/// (season, episode) — TMDB episode IDs can change when the show is updated.
#[derive(Debug, Clone, Default)]
struct PreservedEpisodeState {
    watched: bool,
    watched_at: Option<String>,
    scheduled_date: Option<String>,
    rating: Option<f64>,
    tags: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Episode {
    pub id: i64,
    pub show_id: i64,
    #[sqlx(rename = "show_name")]
    pub show_name: String,
    pub network: Option<String>,
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
    crate::commands::validation::validate_id(episode_id)?;
    
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    // Use parameterized query instead of format! for safety
    if watched {
        sqlx::query(
            r#"
            UPDATE episodes
            SET watched = ?, watched_at = datetime('now')
            WHERE id = ?
            "#,
        )
        .bind(1)
        .bind(episode_id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to mark episode watched: {}", e))?;
    } else {
        sqlx::query(
            r#"
            UPDATE episodes
            SET watched = ?, watched_at = NULL
            WHERE id = ?
            "#,
        )
        .bind(0)
        .bind(episode_id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to mark episode watched: {}", e))?;
    }

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
            s.network,
            COALESCE(e.season_number, 0) as season_number,
            COALESCE(e.episode_number, 0) as episode_number,
            e.name,
            e.aired,
            e.scheduled_date,
            e.watched,
            s.poster_url
        FROM episodes e
        JOIN shows s ON e.show_id = s.id
        WHERE s.tier_only = 0
          AND ((e.aired >= ? AND e.aired <= ?)
           OR (e.scheduled_date >= ? AND e.scheduled_date <= ?))
        ORDER BY COALESCE(e.scheduled_date, e.aired), s.name
        LIMIT 10000
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
            network: row.get("network"),
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
    crate::commands::ensure_show_is_mapped(&app, show_id).await?;

    tmdb::invalidate_tv_show_cache(show_id).await;

    let show_details = tmdb::get_tv_details(show_id)
        .await
        .map_err(|e| format!("Failed to fetch show details: {}", e))?;

    let episodes = tmdb::get_tv_episodes(show_id)
        .await
        .map_err(|e| format!("Failed to fetch episodes: {}", e))?;

    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let preserved = snapshot_episode_state(&pool, show_id).await?;

    let mut tx = pool.begin().await
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    if let Err(e) = sqlx::query(
        r#"
        UPDATE shows SET
            name = ?,
            status = ?,
            poster_url = ?,
            first_aired = ?,
            network = ?,
            overview = ?,
            runtime = ?,
            last_synced = datetime('now')
        WHERE id = ?
        "#,
    )
    .bind(&show_details.name)
    .bind(show_details.status.as_ref())
    .bind(show_details.poster_url())
    .bind(show_details.first_air_date.as_ref())
    .bind(show_details.network_name())
    .bind(show_details.overview.as_ref())
    .bind(show_details.runtime())
    .bind(show_id)
    .execute(&mut *tx)
    .await
    {
        let _ = tx.rollback().await;
        return Err(format!("Failed to update show: {}", e));
    }

    // Drop existing episodes — TMDB episode IDs are not stable across re-syncs.
    // change_history rows for the gone IDs are cascaded by FK; user state is
    // restored below by (season, episode) lookup.
    if let Err(e) = sqlx::query("DELETE FROM episodes WHERE show_id = ?")
        .bind(show_id)
        .execute(&mut *tx)
        .await
    {
        let _ = tx.rollback().await;
        return Err(format!("Failed to clear old episodes: {}", e));
    }

    for ep in episodes {
        let state = preserved
            .get(&(ep.season_number, ep.episode_number))
            .cloned()
            .unwrap_or_default();

        if let Err(e) = sqlx::query(
            r#"
            INSERT INTO episodes
            (id, show_id, season_number, episode_number, name, overview, aired,
             runtime, image_url, scheduled_date, watched, watched_at, rating, tags)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(ep.id)
        .bind(show_id)
        .bind(ep.season_number)
        .bind(ep.episode_number)
        .bind(ep.name.as_ref())
        .bind(ep.overview.as_ref())
        .bind(ep.air_date.as_ref())
        .bind(ep.runtime)
        .bind(ep.image_url())
        .bind(state.scheduled_date.or_else(|| ep.air_date.clone()))
        .bind(if state.watched { 1 } else { 0 })
        .bind(state.watched_at)
        .bind(state.rating)
        .bind(state.tags)
        .execute(&mut *tx)
        .await
        {
            let _ = tx.rollback().await;
            return Err(format!("Failed to sync episode: {}", e));
        }
    }

    tx.commit().await
        .map_err(|e| format!("Failed to commit transaction: {}", e))?;

    Ok(())
}

async fn snapshot_episode_state(
    pool: &sqlx::SqlitePool,
    show_id: i64,
) -> Result<HashMap<(i32, i32), PreservedEpisodeState>, String> {
    let rows = sqlx::query(
        r#"
        SELECT season_number, episode_number, watched, watched_at,
               scheduled_date, rating, tags
        FROM episodes
        WHERE show_id = ?
        "#,
    )
    .bind(show_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to snapshot episode state: {}", e))?;

    let mut map = HashMap::new();
    for row in rows {
        let key = (
            row.get::<i32, _>("season_number"),
            row.get::<i32, _>("episode_number"),
        );
        map.insert(
            key,
            PreservedEpisodeState {
                watched: row.get::<i32, _>("watched") == 1,
                watched_at: row.get("watched_at"),
                scheduled_date: row.get("scheduled_date"),
                rating: row.get("rating"),
                tags: row.get("tags"),
            },
        );
    }
    Ok(map)
}

async fn sync_show_episodes_ref(app: &AppHandle, show_id: i64) -> Result<(), String> {
    sync_show_episodes(app.clone(), show_id).await
}

/// Sync all tracked shows — refetches metadata + episodes from TMDB.
/// Skips quarantined rows (they'd just error in the per-show call anyway).
#[tauri::command]
pub async fn sync_all_shows(app: AppHandle) -> Result<u32, String> {
    tmdb::clear_all_tv_caches().await;

    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let show_ids: Vec<i64> = sqlx::query_scalar(
        "SELECT id FROM shows WHERE id > 0 AND unmigrated = 0",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to get shows: {}", e))?;

    let mut synced = 0u32;
    let mut errors: Vec<String> = Vec::new();

    for show_id in show_ids {
        match sync_show_episodes_ref(&app, show_id).await {
            Ok(_) => synced += 1,
            Err(e) => {
                let error_msg = format!("Show {}: {}", show_id, e);
                eprintln!("Failed to sync show {}: {}", show_id, e);
                errors.push(error_msg);
            }
        }
        // Gentle pace to stay well under TMDB's ~50 req/s ceiling.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    if !errors.is_empty() {
        eprintln!("[sync_all_shows] {} shows failed to sync: {:?}", errors.len(), errors);
    }

    Ok(synced)
}

#[tauri::command]
pub async fn schedule_episode(
    app: AppHandle,
    episode_id: i64,
    date: String,
) -> Result<(), String> {
    crate::commands::validation::validate_id(episode_id)?;
    crate::commands::validation::validate_date(&date)?;
    
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    sqlx::query("UPDATE episodes SET scheduled_date = ? WHERE id = ?")
        .bind(&date)
        .bind(episode_id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to schedule episode: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn unschedule_episode(app: AppHandle, episode_id: i64) -> Result<(), String> {
    crate::commands::validation::validate_id(episode_id)?;
    
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
    crate::commands::validation::validate_id(show_id)?;
    
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    // Use parameterized query instead of format! for safety
    if watched {
        sqlx::query(
            r#"
            UPDATE episodes
            SET watched = ?, watched_at = datetime('now')
            WHERE show_id = ? AND season_number = ?
            "#,
        )
        .bind(1)
        .bind(show_id)
        .bind(season_number)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to mark season watched: {}", e))?;
    } else {
        sqlx::query(
            r#"
            UPDATE episodes
            SET watched = ?, watched_at = NULL
            WHERE show_id = ? AND season_number = ?
            "#,
        )
        .bind(0)
        .bind(show_id)
        .bind(season_number)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to mark season watched: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub async fn mark_show_watched(
    app: AppHandle,
    show_id: i64,
    watched: bool,
) -> Result<(), String> {
    crate::commands::validation::validate_id(show_id)?;
    
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    // Use parameterized query instead of format! for safety
    if watched {
        sqlx::query(
            r#"
            UPDATE episodes
            SET watched = ?, watched_at = datetime('now')
            WHERE show_id = ?
            "#,
        )
        .bind(1)
        .bind(show_id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to mark show watched: {}", e))?;
    } else {
        sqlx::query(
            r#"
            UPDATE episodes
            SET watched = ?, watched_at = NULL
            WHERE show_id = ?
            "#,
        )
        .bind(0)
        .bind(show_id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to mark show watched: {}", e))?;
    }

    Ok(())
}


