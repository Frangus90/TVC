use serde::{Deserialize, Serialize};

use crate::tvdb::{self, EpisodeBase, SearchResult, SeriesExtended};

#[derive(Debug, Serialize, Deserialize)]
pub struct TrackedShow {
    pub id: i64,
    pub name: String,
    pub poster_url: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Episode {
    pub id: i64,
    pub show_id: i64,
    pub show_name: String,
    pub season_number: i32,
    pub episode_number: i32,
    pub name: Option<String>,
    pub aired: Option<String>,
    pub watched: bool,
    pub poster_url: Option<String>,
}

#[tauri::command]
pub async fn search_shows(query: String) -> Result<Vec<SearchResult>, String> {
    tvdb::search_series(&query).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_show_details(id: i64) -> Result<SeriesExtended, String> {
    tvdb::get_series_extended(id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_show(_id: i64) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn remove_show(_id: i64) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn get_tracked_shows() -> Result<Vec<TrackedShow>, String> {
    Ok(vec![])
}

#[tauri::command]
pub async fn sync_episodes_for_show(show_id: i64) -> Result<Vec<EpisodeBase>, String> {
    tvdb::get_series_episodes(show_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sync_show_episodes(_show_id: i64) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn mark_episode_watched(_episode_id: i64, _watched: bool) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn get_episodes_for_range(
    _start_date: String,
    _end_date: String,
) -> Result<Vec<Episode>, String> {
    Ok(vec![])
}
