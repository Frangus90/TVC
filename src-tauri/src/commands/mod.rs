pub mod shows;
pub mod episodes;
pub mod movies;
pub mod statistics;
pub mod history;
pub mod duplicates;
pub mod maintenance;
pub mod metadata;
pub mod backup;
pub mod arr;
pub mod plex;
pub mod app;

use crate::tvdb::{self, EpisodeBase, SearchResult, SeriesExtended};

// Keep old command signatures for backward compatibility but delegate to new implementations
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
pub async fn sync_episodes_for_show(show_id: i64) -> Result<Vec<EpisodeBase>, String> {
    tvdb::get_series_episodes(show_id)
        .await
        .map_err(|e| e.to_string())
}
