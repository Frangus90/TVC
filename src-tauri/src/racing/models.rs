use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RacingSeries {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub category: String,
    pub ics_url: String,
    pub custom_ics_url: Option<String>,
    pub enabled: bool,
    pub notify_enabled: bool,
    pub notify_minutes: i32,
    pub color: String,
    pub custom_color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RacingEvent {
    pub id: i64,
    pub series_slug: String,
    pub uid: String,
    pub event_title: String,
    pub session_name: Option<String>,
    pub circuit: Option<String>,
    pub start_time: String,
    pub end_time: Option<String>,
    pub description: Option<String>,
    pub notified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RacingConfig {
    pub notifications_enabled: bool,
    pub default_notify_minutes: i32,
    pub last_refreshed: Option<String>,
}

impl Default for RacingConfig {
    fn default() -> Self {
        Self {
            notifications_enabled: true,
            default_notify_minutes: 30,
            last_refreshed: None,
        }
    }
}
