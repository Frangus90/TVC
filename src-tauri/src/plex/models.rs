use serde::{Deserialize, Serialize};

/// Plex webhook payload
#[derive(Debug, Clone, Deserialize)]
pub struct PlexPayload {
    pub event: String,
    #[serde(rename = "Metadata")]
    pub metadata: Option<PlexMetadata>,
    // Allow any other fields Plex sends
    #[serde(flatten)]
    pub _extra: serde_json::Value,
}

/// Plex metadata from webhook
#[derive(Debug, Clone, Deserialize)]
pub struct PlexMetadata {
    #[serde(rename = "type")]
    pub media_type: String,
    pub title: String,
    /// Show name (for episodes)
    #[serde(rename = "grandparentTitle")]
    pub grandparent_title: Option<String>,
    /// Season number
    #[serde(rename = "parentIndex")]
    pub parent_index: Option<i32>,
    /// Episode number
    pub index: Option<i32>,
    /// Release year
    pub year: Option<i32>,
    /// External IDs (not reliable for show matching)
    #[serde(rename = "Guid", default)]
    #[allow(dead_code)]
    pub guids: Vec<PlexGuid>,
    // Allow any other fields Plex sends
    #[serde(flatten)]
    pub _extra: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlexGuid {
    #[allow(dead_code)]
    pub id: String,
}

/// Plex scrobbler configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlexConfig {
    pub enabled: bool,
    pub port: u16,
}

impl Default for PlexConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 9876,
        }
    }
}

/// Scrobble log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrobbleLogEntry {
    pub id: i64,
    pub event_type: String,
    pub media_type: String,
    pub raw_title: String,
    pub show_name: Option<String>,
    pub season_number: Option<i32>,
    pub episode_number: Option<i32>,
    pub year: Option<i32>,
    pub matched_entity_type: Option<String>,
    pub matched_entity_id: Option<i64>,
    pub match_method: Option<String>,
    pub scrobbled_at: String,
}

/// Match result for a scrobble
#[derive(Debug, Clone)]
pub struct MatchResult {
    pub entity_type: String,
    pub entity_id: i64,
    pub method: String,
}
