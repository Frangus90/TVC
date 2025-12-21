use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::AppHandle;
use crate::db::connection;

#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub total_shows: i64,
    pub total_episodes: i64,
    pub total_movies: i64,
    pub orphaned_episodes: i64,
    pub unaired_unscheduled_episodes: i64,
    pub database_size_bytes: i64,
    pub change_history_count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CleanupResult {
    pub orphaned_episodes_removed: i64,
    pub unaired_episodes_removed: i64,
    pub history_entries_removed: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CleanupEpisodePreview {
    pub id: i64,
    pub show_name: String,
    pub season_number: i32,
    pub episode_number: i32,
    pub name: Option<String>,
}

/// Get database statistics
#[tauri::command]
pub async fn get_database_stats(app: AppHandle) -> Result<DatabaseStats, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let total_shows: i64 = sqlx::query(r#"SELECT COUNT(*) as count FROM shows"#)
        .fetch_one(&pool)
        .await
        .map(|row| row.get("count"))
        .unwrap_or(0);

    let total_episodes: i64 = sqlx::query(r#"SELECT COUNT(*) as count FROM episodes"#)
        .fetch_one(&pool)
        .await
        .map(|row| row.get("count"))
        .unwrap_or(0);

    let total_movies: i64 = sqlx::query(r#"SELECT COUNT(*) as count FROM movies"#)
        .fetch_one(&pool)
        .await
        .map(|row| row.get("count"))
        .unwrap_or(0);

    // Orphaned episodes (show deleted but episodes remain)
    let orphaned_episodes: i64 = sqlx::query(
        r#"SELECT COUNT(*) as count FROM episodes e
           WHERE NOT EXISTS (SELECT 1 FROM shows s WHERE s.id = e.show_id)"#
    )
    .fetch_one(&pool)
    .await
    .map(|row| row.get("count"))
    .unwrap_or(0);

    // Unaired and unscheduled episodes
    let unaired_unscheduled: i64 = sqlx::query(
        r#"SELECT COUNT(*) as count FROM episodes
           WHERE (aired IS NULL OR aired = '')
           AND (scheduled_date IS NULL OR scheduled_date = '')
           AND watched = 0"#
    )
    .fetch_one(&pool)
    .await
    .map(|row| row.get("count"))
    .unwrap_or(0);

    // Get database file size
    let db_size: i64 = sqlx::query(r#"SELECT page_count * page_size as size FROM pragma_page_count(), pragma_page_size()"#)
        .fetch_one(&pool)
        .await
        .map(|row| row.get("size"))
        .unwrap_or(0);

    let history_count: i64 = sqlx::query(r#"SELECT COUNT(*) as count FROM change_history"#)
        .fetch_one(&pool)
        .await
        .map(|row| row.get("count"))
        .unwrap_or(0);

    Ok(DatabaseStats {
        total_shows,
        total_episodes,
        total_movies,
        orphaned_episodes,
        unaired_unscheduled_episodes: unaired_unscheduled,
        database_size_bytes: db_size,
        change_history_count: history_count,
    })
}

/// Remove orphaned episodes (episodes whose show no longer exists)
#[tauri::command]
pub async fn cleanup_orphaned_episodes(app: AppHandle) -> Result<i64, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let result = sqlx::query(
        r#"DELETE FROM episodes
           WHERE NOT EXISTS (SELECT 1 FROM shows s WHERE s.id = episodes.show_id)"#
    )
    .execute(&pool)
    .await
    .map_err(|e| format!("Failed to cleanup orphaned episodes: {}", e))?;

    Ok(result.rows_affected() as i64)
}

/// Remove unaired episodes that are not scheduled and not watched
#[tauri::command]
pub async fn cleanup_unaired_episodes(app: AppHandle) -> Result<i64, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let result = sqlx::query(
        r#"DELETE FROM episodes
           WHERE (aired IS NULL OR aired = '')
           AND (scheduled_date IS NULL OR scheduled_date = '')
           AND watched = 0"#
    )
    .execute(&pool)
    .await
    .map_err(|e| format!("Failed to cleanup unaired episodes: {}", e))?;

    Ok(result.rows_affected() as i64)
}

/// Get preview of orphaned episodes (episodes whose show no longer exists)
#[tauri::command]
pub async fn get_orphaned_episodes_preview(app: AppHandle) -> Result<Vec<CleanupEpisodePreview>, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let rows = sqlx::query(
        r#"SELECT e.id, e.show_id, e.season_number, e.episode_number, e.name
           FROM episodes e
           WHERE NOT EXISTS (SELECT 1 FROM shows s WHERE s.id = e.show_id)
           ORDER BY e.show_id, e.season_number, e.episode_number
           LIMIT 50"#
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to get orphaned episodes: {}", e))?;

    let episodes: Vec<CleanupEpisodePreview> = rows.iter().map(|row| {
        let show_id: i64 = row.get("show_id");
        CleanupEpisodePreview {
            id: row.get("id"),
            show_name: format!("Unknown Show (ID: {})", show_id),
            season_number: row.get("season_number"),
            episode_number: row.get("episode_number"),
            name: row.get("name"),
        }
    }).collect();

    Ok(episodes)
}

/// Get preview of unaired/unscheduled episodes
#[tauri::command]
pub async fn get_unaired_episodes_preview(app: AppHandle) -> Result<Vec<CleanupEpisodePreview>, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let rows = sqlx::query(
        r#"SELECT e.id, COALESCE(s.name, 'Unknown Show') as show_name,
                  e.season_number, e.episode_number, e.name
           FROM episodes e
           LEFT JOIN shows s ON s.id = e.show_id
           WHERE (e.aired IS NULL OR e.aired = '')
           AND (e.scheduled_date IS NULL OR e.scheduled_date = '')
           AND e.watched = 0
           ORDER BY show_name, e.season_number, e.episode_number
           LIMIT 50"#
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to get unaired episodes: {}", e))?;

    let episodes: Vec<CleanupEpisodePreview> = rows.iter().map(|row| {
        CleanupEpisodePreview {
            id: row.get("id"),
            show_name: row.get("show_name"),
            season_number: row.get("season_number"),
            episode_number: row.get("episode_number"),
            name: row.get("name"),
        }
    }).collect();

    Ok(episodes)
}

/// Optimize database (VACUUM and rebuild indexes)
#[tauri::command]
pub async fn optimize_database(app: AppHandle) -> Result<(), String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    // Run VACUUM to reclaim space
    sqlx::query(r#"VACUUM"#)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to vacuum database: {}", e))?;

    // Analyze for query optimization
    sqlx::query(r#"ANALYZE"#)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to analyze database: {}", e))?;

    Ok(())
}

/// Run full cleanup operation
#[tauri::command]
pub async fn run_full_cleanup(app: AppHandle) -> Result<CleanupResult, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    // Cleanup orphaned episodes
    let orphaned = sqlx::query(
        r#"DELETE FROM episodes
           WHERE NOT EXISTS (SELECT 1 FROM shows s WHERE s.id = episodes.show_id)"#
    )
    .execute(&pool)
    .await
    .map_err(|e| format!("Failed to cleanup orphaned episodes: {}", e))?
    .rows_affected() as i64;

    // Cleanup unaired unscheduled episodes
    let unaired = sqlx::query(
        r#"DELETE FROM episodes
           WHERE (aired IS NULL OR aired = '')
           AND (scheduled_date IS NULL OR scheduled_date = '')
           AND watched = 0"#
    )
    .execute(&pool)
    .await
    .map_err(|e| format!("Failed to cleanup unaired episodes: {}", e))?
    .rows_affected() as i64;

    // Clear old history entries (older than 30 days)
    let history = sqlx::query(
        r#"DELETE FROM change_history
           WHERE changed_at < datetime('now', '-30 days')"#
    )
    .execute(&pool)
    .await
    .map_err(|e| format!("Failed to cleanup history: {}", e))?
    .rows_affected() as i64;

    // Vacuum after cleanup
    sqlx::query(r#"VACUUM"#)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to vacuum database: {}", e))?;

    Ok(CleanupResult {
        orphaned_episodes_removed: orphaned,
        unaired_episodes_removed: unaired,
        history_entries_removed: history,
    })
}
