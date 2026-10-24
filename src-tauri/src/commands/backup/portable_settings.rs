//! Explicit non-secret preferences. Never serialize the generic settings table.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{Row, SqliteConnection};
use std::collections::HashSet;

use crate::commands::preferences::GeneralPreferences;
use crate::notifications::models::NotificationSettings;

const GENERAL_KEYS: &[&str] = &[
    "startup_page",
    "remember_calendar",
    "close_action",
    "start_minimized",
    "release_country",
    "last_page",
    "calendar_view",
    "calendar_filter",
    "default_tier_list",
];
const NOTIFICATION_KEYS: &[&str] = &[
    "enabled",
    "sound_enabled",
    "sound_volume",
    "sound_choice",
    "popup_position",
    "popup_duration",
    "max_visible",
    "os_fallback",
    "tray_notifications",
    "racing_enabled",
    "plex_enabled",
    "premiere_enabled",
    "update_enabled",
    "system_enabled",
    "quiet_hours_enabled",
    "quiet_hours_start",
    "quiet_hours_end",
];
const APPEARANCE_KEYS: &[&str] = &[
    "accentColor",
    "fontSize",
    "compactSpacing",
    "colorblindFriendly",
    "hidePosters",
    "posterSize",
];

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortableSettings {
    pub appearance: Value,
    pub general: Value,
    pub notifications: Value,
    pub racing: RacingPreferences,
    pub tier_preset: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RacingPreferences {
    pub notifications_enabled: bool,
    pub default_notify_minutes: i32,
    pub series: Vec<SeriesPreferences>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeriesPreferences {
    pub slug: String,
    pub enabled: bool,
    pub notify_enabled: bool,
    pub notify_minutes: i32,
    pub custom_color: Option<String>,
}

fn exact_keys(value: &Value, keys: &[&str], group: &str) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("Invalid {group} settings"))?;
    if object.len() != keys.len() || keys.iter().any(|key| !object.contains_key(*key)) {
        return Err(format!(
            "Backup {group} settings contain missing or unsupported preferences"
        ));
    }
    Ok(())
}

fn color(value: &str) -> bool {
    matches!(value.len(), 4 | 7)
        && value.starts_with('#')
        && value[1..].bytes().all(|c| c.is_ascii_hexdigit())
}

fn clock(value: &str) -> bool {
    let b = value.as_bytes();
    b.len() == 5
        && b[2] == b':'
        && [b[0], b[1], b[3], b[4]].iter().all(u8::is_ascii_digit)
        && chrono::NaiveTime::parse_from_str(value, "%H:%M").is_ok()
}

impl PortableSettings {
    pub fn validate(&self) -> Result<(), String> {
        exact_keys(&self.appearance, APPEARANCE_KEYS, "appearance")?;
        if !self.appearance["accentColor"].as_str().is_some_and(color)
            || !self.appearance["fontSize"]
                .as_f64()
                .is_some_and(|v| (0.75..=1.5).contains(&v))
            || !["small", "medium", "large"]
                .contains(&self.appearance["posterSize"].as_str().unwrap_or(""))
            || ["compactSpacing", "colorblindFriendly", "hidePosters"]
                .iter()
                .any(|key| !self.appearance[*key].is_boolean())
        {
            return Err("Invalid appearance preferences in backup".into());
        }
        // Backups from before the default-list setting used the same v4 format.
        let mut general_value = self.general.clone();
        if let Some(object) = general_value.as_object_mut() {
            object.entry("default_tier_list").or_insert(json!("shows"));
        }
        exact_keys(&general_value, GENERAL_KEYS, "general")?;
        let general: GeneralPreferences = serde_json::from_value(general_value)
            .map_err(|e| format!("Invalid general settings: {e}"))?;
        if !["calendar", "movies", "shows", "last_used"].contains(&general.startup_page.as_str())
            || !["tray", "quit"].contains(&general.close_action.as_str())
            || ![
                "US", "NO", "GB", "SE", "DK", "FI", "DE", "FR", "NL", "ES", "IT", "CA", "AU",
            ]
            .contains(&general.release_country.as_str())
            || ![
                "calendar",
                "movies",
                "shows",
                "tiers",
                "racing",
                "awards",
                "statistics",
                "data",
                "settings",
            ]
            .contains(&general.last_page.as_str())
            || !["month", "week", "agenda"].contains(&general.calendar_view.as_str())
            || !["all", "shows", "movies"].contains(&general.calendar_filter.as_str())
            || !["shows", "movies"].contains(&general.default_tier_list.as_str())
        {
            return Err("Invalid general preferences in backup".into());
        }
        exact_keys(&self.notifications, NOTIFICATION_KEYS, "notification")?;
        let n: NotificationSettings = serde_json::from_value(self.notifications.clone())
            .map_err(|e| format!("Invalid notification settings: {e}"))?;
        if !(0..=100).contains(&n.sound_volume)
            || !["chime", "bell", "soft", "alert"].contains(&n.sound_choice.as_str())
            || !["top-right", "top-left", "bottom-right", "bottom-left"]
                .contains(&n.popup_position.as_str())
            || ![0, 3000, 5000, 8000, 12000].contains(&n.popup_duration)
            || !(1..=5).contains(&n.max_visible)
            || !clock(&n.quiet_hours_start)
            || !clock(&n.quiet_hours_end)
            || (n.quiet_hours_enabled && n.quiet_hours_start == n.quiet_hours_end)
        {
            return Err("Invalid notification preferences in backup".into());
        }
        if !["10-star", "5-star", "custom"].contains(&self.tier_preset.as_str()) {
            return Err("Invalid tier preset in backup".into());
        }
        let mut slugs = HashSet::new();
        if !(0..=10080).contains(&self.racing.default_notify_minutes)
            || self.racing.series.len() > 100
        {
            return Err("Invalid racing preferences in backup".into());
        }
        for series in &self.racing.series {
            if series.slug.is_empty()
                || series.slug.len() > 80
                || !series
                    .slug
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
                || !slugs.insert(&series.slug)
                || !(0..=10080).contains(&series.notify_minutes)
                || series.custom_color.as_ref().is_some_and(|c| !color(c))
            {
                return Err("Invalid racing series preferences in backup".into());
            }
        }
        Ok(())
    }

    pub async fn export(connection: &mut SqliteConnection) -> Result<Self, String> {
        let result: Result<Self, sqlx::Error> = async {
            let mut appearance = json!({"accentColor":"#bba4ff", "fontSize":1.0, "compactSpacing":false, "colorblindFriendly":false, "hidePosters":false, "posterSize":"medium"});
            let mut general = serde_json::to_value(GeneralPreferences::default()).expect("General preferences are serializable");
            let mut notifications = serde_json::to_value(NotificationSettings::default()).expect("Notification preferences are serializable");
            let mut tier_preset = "10-star".to_string();
            let rows = sqlx::query("SELECT key, value FROM settings WHERE key LIKE 'theme_%' OR key = 'general_preferences' OR key = 'tier_preset' OR key LIKE 'notification_quiet_hours_%'").fetch_all(&mut *connection).await?;
            for row in rows {
                let key: String = row.get("key");
                let Some(value) = row.get::<Option<String>, _>("value") else { continue; };
                if key == "general_preferences" {
                    general = serde_json::from_str(&value).map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
                } else if key == "tier_preset" { tier_preset = value; }
                else if let Some(key) = key.strip_prefix("theme_") {
                    if APPEARANCE_KEYS.contains(&key) {
                        appearance[key] = if appearance[key].is_string() { Value::String(value) } else { serde_json::from_str(&value).map_err(|e| sqlx::Error::Decode(Box::new(e)))? };
                    }
                } else if let Some(key) = key.strip_prefix("notification_") {
                    if NOTIFICATION_KEYS.contains(&key) {
                        notifications[key] = if notifications[key].is_boolean() { serde_json::from_str(&value).map_err(|e| sqlx::Error::Decode(Box::new(e)))? } else { Value::String(value) };
                    }
                }
            }
            if let Some(object) = general.as_object_mut() {
                object.entry("default_tier_list").or_insert(json!("shows"));
            }
            let row = sqlx::query("SELECT * FROM notification_settings WHERE id = 1").fetch_one(&mut *connection).await?;
            for key in NOTIFICATION_KEYS.iter().filter(|key| !key.starts_with("quiet_hours_")) {
                notifications[*key] = if notifications[*key].is_boolean() { Value::Bool(row.get::<i32, _>(*key) == 1) }
                    else if notifications[*key].is_number() { json!(row.get::<i32, _>(*key)) }
                    else { Value::String(row.get::<String, _>(*key)) };
            }
            let config = sqlx::query("SELECT notifications_enabled, default_notify_minutes FROM racing_config WHERE id = 1").fetch_one(&mut *connection).await?;
            let series = sqlx::query("SELECT slug, enabled, notify_enabled, notify_minutes, custom_color FROM racing_series ORDER BY slug").fetch_all(&mut *connection).await?
                .into_iter().map(|r| SeriesPreferences { slug:r.get("slug"), enabled:r.get::<i32,_>("enabled") == 1, notify_enabled:r.get::<i32,_>("notify_enabled") == 1, notify_minutes:r.get("notify_minutes"), custom_color:r.get("custom_color") }).collect();
            Ok(Self { appearance, general, notifications, tier_preset, racing: RacingPreferences { notifications_enabled:config.get::<i32,_>("notifications_enabled") == 1, default_notify_minutes:config.get("default_notify_minutes"), series } })
        }.await;
        let settings = result.map_err(|e| format!("Could not export portable settings: {e}"))?;
        settings.validate()?;
        Ok(settings)
    }

    pub async fn restore(&self, connection: &mut SqliteConnection) -> Result<(), String> {
        self.validate()?;
        let n: NotificationSettings =
            serde_json::from_value(self.notifications.clone()).map_err(|e| e.to_string())?;
        let result: Result<(), sqlx::Error> = async {
            for key in APPEARANCE_KEYS {
                let value = &self.appearance[*key];
                put(connection, &format!("theme_{key}"), value.as_str().map(String::from).unwrap_or_else(|| value.to_string())).await?;
            }
            put(connection, "general_preferences", self.general.to_string()).await?;
            put(connection, "tier_preset", self.tier_preset.clone()).await?;
            let saved = sqlx::query("UPDATE notification_settings SET enabled=?,sound_enabled=?,sound_volume=?,sound_choice=?,popup_position=?,popup_duration=?,max_visible=?,os_fallback=?,tray_notifications=?,racing_enabled=?,plex_enabled=?,premiere_enabled=?,update_enabled=?,system_enabled=?,updated_at=datetime('now') WHERE id=1")
                .bind(n.enabled).bind(n.sound_enabled).bind(n.sound_volume).bind(&n.sound_choice).bind(&n.popup_position).bind(n.popup_duration).bind(n.max_visible)
                .bind(n.os_fallback).bind(n.tray_notifications).bind(n.racing_enabled).bind(n.plex_enabled).bind(n.premiere_enabled).bind(n.update_enabled).bind(n.system_enabled)
                .execute(&mut *connection).await?;
            if saved.rows_affected() != 1 { return Err(sqlx::Error::Protocol("Notification settings are missing from the current database".into())); }
            put(connection, "notification_quiet_hours_enabled", n.quiet_hours_enabled.to_string()).await?;
            put(connection, "notification_quiet_hours_start", n.quiet_hours_start).await?;
            put(connection, "notification_quiet_hours_end", n.quiet_hours_end).await?;
            let saved = sqlx::query("UPDATE racing_config SET notifications_enabled=?,default_notify_minutes=?,updated_at=datetime('now') WHERE id=1")
                .bind(self.racing.notifications_enabled).bind(self.racing.default_notify_minutes).execute(&mut *connection).await?;
            if saved.rows_affected() != 1 { return Err(sqlx::Error::Protocol("Racing settings are missing from the current database".into())); }
            // Empty scopes mean no followed series. Target-only series must not
            // stay enabled after a portable restore from a different app version.
            sqlx::query("UPDATE racing_series SET enabled=0").execute(&mut *connection).await?;
            for series in &self.racing.series {
                let result = sqlx::query("UPDATE racing_series SET enabled=?,notify_enabled=?,notify_minutes=?,custom_color=? WHERE slug=?")
                    .bind(series.enabled).bind(series.notify_enabled).bind(series.notify_minutes).bind(&series.custom_color).bind(&series.slug).execute(&mut *connection).await?;
                if result.rows_affected() != 1 { return Err(sqlx::Error::Protocol(format!("Racing series '{}' is unavailable in this TVC version", series.slug))); }
            }
            Ok(())
        }.await;
        result.map_err(|e| format!("Could not restore portable settings; no changes saved: {e}"))
    }
}

async fn put(
    connection: &mut SqliteConnection,
    key: &str,
    value: String,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO settings(key,value) VALUES (?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value")
        .bind(key).bind(value).execute(connection).await?;
    Ok(())
}
