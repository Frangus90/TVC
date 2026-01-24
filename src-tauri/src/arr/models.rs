use serde::{Deserialize, Serialize};

/// Arr server configuration stored in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArrServer {
    pub id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub server_type: String,
    pub base_url: String,
    pub api_key: String,
    pub is_active: bool,
    pub auto_sync_enabled: bool,
    pub sync_interval_hours: i32,
    pub last_synced: Option<String>,
    pub added_at: Option<String>,
}

/// Request to add/update a server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArrServerRequest {
    pub name: String,
    #[serde(rename = "type")]
    pub server_type: String,
    pub base_url: String,
    pub api_key: String,
}

/// Sonarr series from API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SonarrSeries {
    pub id: i64,
    pub title: String,
    pub sort_title: Option<String>,
    pub status: Option<String>,
    pub overview: Option<String>,
    pub network: Option<String>,
    pub year: Option<i32>,
    pub path: Option<String>,
    pub tvdb_id: Option<i64>,
    pub imdb_id: Option<String>,
    pub monitored: bool,
    pub runtime: Option<i32>,
    pub genres: Option<Vec<String>>,
    pub images: Option<Vec<SonarrImage>>,
    pub statistics: Option<SonarrStatistics>,
}

impl SonarrSeries {
    pub fn poster_url(&self) -> Option<String> {
        self.images.as_ref().and_then(|images| {
            images
                .iter()
                .find(|img| img.cover_type == "poster")
                .map(|img| img.remote_url.clone().unwrap_or_default())
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SonarrImage {
    pub cover_type: String,
    pub url: Option<String>,
    pub remote_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SonarrStatistics {
    pub season_count: Option<i32>,
    pub episode_count: Option<i32>,
    pub episode_file_count: Option<i32>,
    pub total_episode_count: Option<i32>,
    pub percent_of_episodes: Option<f64>,
}

/// Radarr movie from API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RadarrMovie {
    pub id: i64,
    pub title: String,
    pub sort_title: Option<String>,
    pub status: Option<String>,
    pub overview: Option<String>,
    pub year: Option<i32>,
    pub path: Option<String>,
    pub tmdb_id: Option<i64>,
    pub imdb_id: Option<String>,
    pub monitored: bool,
    pub runtime: Option<i32>,
    pub genres: Option<Vec<String>>,
    pub images: Option<Vec<RadarrImage>>,
    pub has_file: Option<bool>,
    pub studio: Option<String>,
}

impl RadarrMovie {
    pub fn poster_url(&self) -> Option<String> {
        self.images.as_ref().and_then(|images| {
            images
                .iter()
                .find(|img| img.cover_type == "poster")
                .map(|img| img.remote_url.clone().unwrap_or_default())
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RadarrImage {
    pub cover_type: String,
    pub url: Option<String>,
    pub remote_url: Option<String>,
}

/// System status from Sonarr/Radarr
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArrSystemStatus {
    pub version: String,
    pub app_name: Option<String>,
}

/// Import result for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub total: usize,
    pub imported: usize,
    pub skipped: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

/// Library item for preview (unified for both Sonarr and Radarr)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryItem {
    pub arr_id: i64,
    pub title: String,
    pub year: Option<i32>,
    pub poster_url: Option<String>,
    pub monitored: bool,
    pub tvdb_id: Option<i64>,
    pub tmdb_id: Option<i64>,
    pub already_tracked: bool,
}
