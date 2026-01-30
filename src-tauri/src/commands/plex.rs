use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::AppHandle;

use crate::db::connection;
use crate::plex::{self, models::{PlexConfig, ScrobbleLogEntry}};

/// Get Plex scrobbler configuration
#[tauri::command]
pub async fn get_plex_config(app: AppHandle) -> Result<PlexConfig, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    Ok(plex::get_config(&pool).await)
}

/// Update Plex scrobbler configuration
#[tauri::command]
pub async fn update_plex_config(app: AppHandle, config: PlexConfig) -> Result<(), String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    plex::update_config(&pool, &config).await
}

/// Start the Plex webhook server
#[tauri::command]
pub async fn start_plex_server(app: AppHandle, port: u16) -> Result<(), String> {
    crate::commands::validation::validate_port(port)?;
    plex::start_server(app, port).await
}

/// Stop the Plex webhook server
#[tauri::command]
pub async fn stop_plex_server() -> Result<(), String> {
    plex::stop_server().await
}

/// Server status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlexServerStatus {
    pub running: bool,
    pub port: Option<u16>,
}

/// Get Plex server status
#[tauri::command]
pub async fn get_plex_server_status() -> PlexServerStatus {
    PlexServerStatus {
        running: plex::is_server_running().await,
        port: plex::get_server_port().await,
    }
}

/// Get recent scrobble log entries
#[tauri::command]
pub async fn get_scrobble_log(app: AppHandle, limit: i32) -> Result<Vec<ScrobbleLogEntry>, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let rows = sqlx::query(
        r#"
        SELECT id, event_type, media_type, raw_title, show_name, season_number,
               episode_number, year, matched_entity_type, matched_entity_id,
               match_method, scrobbled_at
        FROM plex_scrobble_log
        ORDER BY scrobbled_at DESC
        LIMIT ?
        "#,
    )
    .bind(limit)
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to get scrobble log: {}", e))?;

    let entries: Vec<ScrobbleLogEntry> = rows
        .into_iter()
        .map(|row| ScrobbleLogEntry {
            id: row.get("id"),
            event_type: row.get("event_type"),
            media_type: row.get("media_type"),
            raw_title: row.get("raw_title"),
            show_name: row.get("show_name"),
            season_number: row.get("season_number"),
            episode_number: row.get("episode_number"),
            year: row.get("year"),
            matched_entity_type: row.get("matched_entity_type"),
            matched_entity_id: row.get("matched_entity_id"),
            match_method: row.get("match_method"),
            scrobbled_at: row.get("scrobbled_at"),
        })
        .collect();

    Ok(entries)
}
