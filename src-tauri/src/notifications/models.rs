use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: i64,
    pub r#type: String,
    pub title: String,
    pub body: String,
    pub icon: Option<String>,
    pub reference_id: Option<String>,
    pub reference_type: Option<String>,
    pub read: bool,
    pub dismissed: bool,
    pub created_at: String,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    pub enabled: bool,
    pub sound_enabled: bool,
    pub sound_volume: i32,
    pub sound_choice: String,
    pub popup_position: String,
    pub popup_duration: i32,
    pub max_visible: i32,
    pub os_fallback: bool,
    pub tray_notifications: bool,
    pub racing_enabled: bool,
    pub plex_enabled: bool,
    pub premiere_enabled: bool,
    pub update_enabled: bool,
    pub system_enabled: bool,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            sound_enabled: true,
            sound_volume: 50,
            sound_choice: "chime".to_string(),
            popup_position: "top-right".to_string(),
            popup_duration: 8000,
            max_visible: 3,
            os_fallback: false,
            tray_notifications: true,
            racing_enabled: true,
            plex_enabled: true,
            premiere_enabled: true,
            update_enabled: true,
            system_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateNotification {
    pub r#type: String,
    pub title: String,
    pub body: String,
    pub icon: Option<String>,
    pub reference_id: Option<String>,
    pub reference_type: Option<String>,
    pub expires_at: Option<String>,
}
