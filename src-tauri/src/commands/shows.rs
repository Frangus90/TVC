use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use tauri::AppHandle;
use crate::db::connection;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct TrackedShow {
    pub id: i64,
    pub name: String,
    pub poster_url: Option<String>,
    pub status: Option<String>,
    pub color: Option<String>,
    pub notes: Option<String>,
    pub tags: Option<String>,
    pub rating: Option<f64>,
}

#[tauri::command]
pub async fn add_show(app: AppHandle, id: i64) -> Result<(), String> {
    // Validate input
    crate::commands::validation::validate_id(id)?;
    
    // Get show details from TVDB
    let show_details = crate::tvdb::get_series_extended(id)
        .await
        .map_err(|e| format!("Failed to fetch show details: {}", e))?;

    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let airs_days_json = show_details.airs_days
        .as_ref()
        .and_then(|days| serde_json::to_string(days).ok());

    sqlx::query(
        r#"
        INSERT OR REPLACE INTO shows 
        (id, name, slug, status, poster_url, first_aired, overview, airs_time, airs_days, runtime, added_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))
        "#,
    )
    .bind(show_details.id)
    .bind(&show_details.name)
    .bind(show_details.slug.as_ref())
    .bind(show_details.status.as_ref().and_then(|s| s.name.as_ref()))
    .bind(show_details.image.as_ref())
    .bind(show_details.first_aired.as_ref())
    .bind(show_details.overview.as_ref())
    .bind(show_details.airs_time.as_ref())
    .bind(airs_days_json.as_deref())
    .bind(show_details.average_runtime)
    .execute(&pool)
    .await
    .map_err(|e| format!("Failed to add show: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn remove_show(app: AppHandle, id: i64) -> Result<(), String> {
    crate::commands::validation::validate_id(id)?;
    
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    sqlx::query("DELETE FROM shows WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to remove show: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn get_tracked_shows(app: AppHandle) -> Result<Vec<TrackedShow>, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let shows = sqlx::query_as::<_, TrackedShow>(
        r#"
        SELECT id, name, poster_url, status, color, notes, tags, rating
        FROM shows
        WHERE archived = 0 OR archived IS NULL
        ORDER BY name
        LIMIT 10000
        "#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to get tracked shows: {}", e))?;

    Ok(shows)
}

#[tauri::command]
pub async fn get_archived_shows(app: AppHandle) -> Result<Vec<TrackedShow>, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let shows = sqlx::query_as::<_, TrackedShow>(
        r#"
        SELECT id, name, poster_url, status, color, notes, tags, rating
        FROM shows
        WHERE archived = 1
        ORDER BY name
        LIMIT 10000
        "#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to get archived shows: {}", e))?;

    Ok(shows)
}

#[tauri::command]
pub async fn archive_show(app: AppHandle, id: i64) -> Result<(), String> {
    crate::commands::validation::validate_id(id)?;
    
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    sqlx::query("UPDATE shows SET archived = 1 WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to archive show: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn unarchive_show(app: AppHandle, id: i64) -> Result<(), String> {
    crate::commands::validation::validate_id(id)?;
    
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    sqlx::query("UPDATE shows SET archived = 0 WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to unarchive show: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn update_show_rating(
    app: AppHandle,
    id: i64,
    rating: Option<f64>,
) -> Result<(), String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    sqlx::query(
        r#"
        UPDATE shows
        SET rating = ?
        WHERE id = ?
        "#,
    )
    .bind(rating)
    .bind(id)
    .execute(&pool)
    .await
    .map_err(|e| format!("Failed to update show rating: {}", e))?;

    Ok(())
}

