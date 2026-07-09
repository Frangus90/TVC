pub mod shows;
pub mod episodes;
pub mod movies;
pub mod statistics;
pub mod duplicates;
pub mod maintenance;
pub mod metadata;
pub mod backup;
pub mod arr;
pub mod plex;
pub mod racing;
pub mod awards;
pub mod notifications;
pub mod tiers;
pub mod app;
pub mod validation;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use crate::db::connection;
use crate::tmdb;

/// Shape returned to the frontend search modal. Mirrors the legacy SearchResult
/// field names so existing UI code keeps working; the IDs are now TMDB ids.
#[derive(Debug, Serialize, Deserialize)]
pub struct TvSearchResult {
    pub tmdb_id: i64,
    pub name: String,
    pub image_url: Option<String>,
    pub first_air_date: Option<String>,
    pub overview: Option<String>,
    pub year: Option<String>,
}

#[tauri::command]
pub async fn search_shows(query: String) -> Result<Vec<TvSearchResult>, String> {
    let results = tmdb::search_tv(&query).await.map_err(|e| e.to_string())?;
    Ok(results
        .into_iter()
        .map(|r| {
            let year = r
                .first_air_date
                .as_ref()
                .and_then(|d| d.split('-').next().map(|s| s.to_string()));
            TvSearchResult {
                tmdb_id: r.id,
                image_url: r.poster_url(),
                name: r.name,
                first_air_date: r.first_air_date,
                overview: r.overview,
                year,
            }
        })
        .collect())
}

#[tauri::command]
pub async fn get_show_details(
    app: AppHandle,
    id: i64,
) -> Result<tmdb::TvShowDetails, String> {
    ensure_show_is_mapped(&app, id).await?;
    tmdb::get_tv_details(id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sync_episodes_for_show(
    app: AppHandle,
    show_id: i64,
) -> Result<Vec<tmdb::TvEpisode>, String> {
    ensure_show_is_mapped(&app, show_id).await?;
    tmdb::get_tv_episodes(show_id).await.map_err(|e| e.to_string())
}

/// Reject calls that would hit TMDB with a still-quarantined (TVDB) id.
/// Negative ids are manual tier-only entries and have no remote backing — also
/// treated as a hard error here because none of these commands make sense for
/// them.
pub(crate) async fn ensure_show_is_mapped(app: &AppHandle, show_id: i64) -> Result<(), String> {
    if show_id < 0 {
        return Err("This show has no TMDB record (manual entry).".into());
    }

    let pool = connection::get_pool(app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let unmigrated: Option<i64> = sqlx::query_scalar(
        "SELECT unmigrated FROM shows WHERE id = ?",
    )
    .bind(show_id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?;

    if unmigrated.unwrap_or(0) == 1 {
        return Err(
            "Show needs TMDB resolution — open Data Management to map it.".into(),
        );
    }

    Ok(())
}
