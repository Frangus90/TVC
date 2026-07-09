//! Tauri commands for the awards feature (thin wrappers over `crate::awards`).

use tauri::AppHandle;

use crate::awards::{db, sync};
use crate::db::connection;

/// Pull fresh data from Wikipedia into the local DB. `full=true` re-pulls 20 years
/// of history; otherwise only the newest ceremonies are refreshed. This is the
/// "Refresh" button endpoint.
#[tauri::command]
pub async fn sync_awards(app: AppHandle, full: Option<bool>) -> Result<sync::SyncSummary, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {e}"))?;
    Ok(sync::sync(&pool, full.unwrap_or(false)).await)
}

/// List stored ceremonies for an award type ("oscars" | "emmys"), newest first.
#[tauri::command]
pub async fn get_award_ceremonies(
    app: AppHandle,
    award_type: String,
) -> Result<Vec<db::CeremonySummary>, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {e}"))?;
    db::get_ceremonies(&pool, &award_type).await
}

/// Full detail for one ceremony: categories with their nominees and winner flags.
#[tauri::command]
pub async fn get_ceremony_detail(
    app: AppHandle,
    ceremony_id: i64,
) -> Result<db::CeremonyDetail, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {e}"))?;
    db::get_ceremony_detail(&pool, ceremony_id).await
}
