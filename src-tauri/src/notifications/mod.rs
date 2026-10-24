pub mod models;

use chrono::{DateTime, Duration, Local, NaiveDateTime, NaiveTime, Utc};
use sqlx::{Pool, Row, Sqlite};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::db::connection;
use models::{CreateNotification, Notification, NotificationSettings};

/// Used by the scheduler's eligibility check; delivery itself requires a successful read.
pub async fn get_settings(pool: &Pool<Sqlite>) -> NotificationSettings {
    get_settings_checked(pool).await.unwrap_or_else(|error| {
        eprintln!("[Notifications] Could not load preferences: {}", error);
        NotificationSettings::default()
    })
}

/// Surface read failures to the Settings page instead of presenting unsaved defaults.
pub async fn get_settings_checked(pool: &Pool<Sqlite>) -> Result<NotificationSettings, String> {
    let row = sqlx::query(
        r#"
        SELECT enabled, sound_enabled, sound_volume, sound_choice, popup_position,
               popup_duration, max_visible, os_fallback, tray_notifications,
               racing_enabled, plex_enabled, premiere_enabled, update_enabled, system_enabled
        FROM notification_settings WHERE id = 1
        "#,
    )
    .fetch_optional(pool)
    .await
    .map_err(|error| format!("Failed to read notification settings: {}", error))?;

    let mut settings = match row {
        Some(r) => NotificationSettings {
            enabled: r.get::<i32, _>("enabled") == 1,
            sound_enabled: r.get::<i32, _>("sound_enabled") == 1,
            sound_volume: r.get::<i32, _>("sound_volume"),
            sound_choice: r.get::<String, _>("sound_choice"),
            popup_position: r.get::<String, _>("popup_position"),
            popup_duration: r.get::<i32, _>("popup_duration"),
            max_visible: r.get::<i32, _>("max_visible"),
            os_fallback: r.get::<i32, _>("os_fallback") == 1,
            tray_notifications: r.get::<i32, _>("tray_notifications") == 1,
            racing_enabled: r.get::<i32, _>("racing_enabled") == 1,
            plex_enabled: r.get::<i32, _>("plex_enabled") == 1,
            premiere_enabled: r.get::<i32, _>("premiere_enabled") == 1,
            update_enabled: r.get::<i32, _>("update_enabled") == 1,
            system_enabled: r.get::<i32, _>("system_enabled") == 1,
            ..NotificationSettings::default()
        },
        None => NotificationSettings::default(),
    };
    // New preferences use the existing generic table; released migrations stay unchanged.
    let rows = sqlx::query("SELECT key, value FROM settings WHERE key IN ('notification_quiet_hours_enabled', 'notification_quiet_hours_start', 'notification_quiet_hours_end', 'notification_auto_clear_after_days')")
        .fetch_all(pool).await.map_err(|error| format!("Failed to read notification preferences: {}", error))?;
    for row in rows {
        let Some(value) = row.get::<Option<String>, _>("value") else {
            continue;
        };
        match row.get::<String, _>("key").as_str() {
            "notification_quiet_hours_enabled" => settings.quiet_hours_enabled = value == "true",
            "notification_quiet_hours_start" => settings.quiet_hours_start = value,
            "notification_quiet_hours_end" => settings.quiet_hours_end = value,
            "notification_auto_clear_after_days" => {
                settings.auto_clear_after_days = value
                    .parse()
                    .map_err(|_| "Invalid saved notification auto-clear setting.".to_string())?;
            }
            _ => {}
        }
    }
    validate_quiet_hours(&settings)?;
    validate_auto_clear_after_days(settings.auto_clear_after_days)?;
    Ok(settings)
}

fn validate_auto_clear_after_days(days: i32) -> Result<(), String> {
    if ![0, 1, 7, 30].contains(&days) {
        return Err("Auto-clear must be Off, 1 day, 1 week or 1 month.".to_string());
    }
    Ok(())
}

fn parse_clock(value: &str) -> Option<NaiveTime> {
    let bytes = value.as_bytes();
    if bytes.len() != 5
        || bytes[2] != b':'
        || ![bytes[0], bytes[1], bytes[3], bytes[4]]
            .iter()
            .all(u8::is_ascii_digit)
    {
        return None;
    }
    NaiveTime::parse_from_str(value, "%H:%M").ok()
}

fn validate_quiet_hours(settings: &NotificationSettings) -> Result<(), String> {
    if parse_clock(&settings.quiet_hours_start).is_none()
        || parse_clock(&settings.quiet_hours_end).is_none()
    {
        return Err("Quiet hours must use 24-hour HH:mm times.".to_string());
    }
    if settings.quiet_hours_enabled && settings.quiet_hours_start == settings.quiet_hours_end {
        return Err("Quiet hours must have different start and end times.".to_string());
    }
    Ok(())
}

pub fn is_quiet_hours_at(settings: &NotificationSettings, local_time: NaiveTime) -> bool {
    if !settings.quiet_hours_enabled {
        return false;
    }
    let (Some(start), Some(end)) = (
        parse_clock(&settings.quiet_hours_start),
        parse_clock(&settings.quiet_hours_end),
    ) else {
        return false;
    };
    if start < end {
        local_time >= start && local_time < end
    } else if start > end {
        local_time >= start || local_time < end
    } else {
        false
    }
}

/// Update notification settings
pub async fn update_settings(
    pool: &Pool<Sqlite>,
    settings: &NotificationSettings,
) -> Result<(), String> {
    validate_quiet_hours(settings)?;
    validate_auto_clear_after_days(settings.auto_clear_after_days)?;
    let mut transaction = pool
        .begin()
        .await
        .map_err(|error| format!("Failed to save notification settings: {}", error))?;
    sqlx::query(
        r#"
        UPDATE notification_settings
        SET enabled = ?, sound_enabled = ?, sound_volume = ?, sound_choice = ?,
            popup_position = ?, popup_duration = ?, max_visible = ?, os_fallback = ?,
            tray_notifications = ?, racing_enabled = ?, plex_enabled = ?, premiere_enabled = ?,
            update_enabled = ?, system_enabled = ?, updated_at = datetime('now')
        WHERE id = 1
        "#,
    )
    .bind(if settings.enabled { 1 } else { 0 })
    .bind(if settings.sound_enabled { 1 } else { 0 })
    .bind(settings.sound_volume)
    .bind(&settings.sound_choice)
    .bind(&settings.popup_position)
    .bind(settings.popup_duration)
    .bind(settings.max_visible)
    .bind(if settings.os_fallback { 1 } else { 0 })
    .bind(if settings.tray_notifications { 1 } else { 0 })
    .bind(if settings.racing_enabled { 1 } else { 0 })
    .bind(if settings.plex_enabled { 1 } else { 0 })
    .bind(if settings.premiere_enabled { 1 } else { 0 })
    .bind(if settings.update_enabled { 1 } else { 0 })
    .bind(if settings.system_enabled { 1 } else { 0 })
    .execute(&mut *transaction)
    .await
    .map_err(|e| format!("Failed to update notification settings: {}", e))?;

    for (key, value) in [
        (
            "notification_quiet_hours_enabled",
            settings.quiet_hours_enabled.to_string(),
        ),
        (
            "notification_quiet_hours_start",
            settings.quiet_hours_start.clone(),
        ),
        (
            "notification_quiet_hours_end",
            settings.quiet_hours_end.clone(),
        ),
        (
            "notification_auto_clear_after_days",
            settings.auto_clear_after_days.to_string(),
        ),
    ] {
        sqlx::query("INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)")
            .bind(key)
            .bind(value)
            .execute(&mut *transaction)
            .await
            .map_err(|error| format!("Failed to save notification preferences: {}", error))?;
    }
    transaction
        .commit()
        .await
        .map_err(|error| format!("Failed to finish saving notification settings: {}", error))?;
    Ok(())
}

/// Create a new notification and return it with its ID
pub async fn create_notification(
    pool: &Pool<Sqlite>,
    notif: &CreateNotification,
) -> Result<Notification, String> {
    let result = sqlx::query(
        r#"
        INSERT INTO notifications (type, title, body, icon, reference_id, reference_type, expires_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&notif.r#type)
    .bind(&notif.title)
    .bind(&notif.body)
    .bind(&notif.icon)
    .bind(&notif.reference_id)
    .bind(&notif.reference_type)
    .bind(&notif.expires_at)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to create notification: {}", e))?;

    let id = result.last_insert_rowid();

    Ok(Notification {
        id,
        r#type: notif.r#type.clone(),
        title: notif.title.clone(),
        body: notif.body.clone(),
        icon: notif.icon.clone(),
        reference_id: notif.reference_id.clone(),
        reference_type: notif.reference_type.clone(),
        read: false,
        dismissed: false,
        created_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        expires_at: notif.expires_at.clone(),
    })
}

/// Get notifications with pagination
pub async fn get_notifications(
    pool: &Pool<Sqlite>,
    limit: i32,
    offset: i32,
    unread_only: bool,
) -> Result<Vec<Notification>, String> {
    let query = if unread_only {
        r#"
        SELECT id, type, title, body, icon, reference_id, reference_type,
               read, dismissed, created_at, expires_at
        FROM notifications
        WHERE dismissed = 0 AND read = 0
        ORDER BY created_at DESC
        LIMIT ? OFFSET ?
        "#
    } else {
        r#"
        SELECT id, type, title, body, icon, reference_id, reference_type,
               read, dismissed, created_at, expires_at
        FROM notifications
        WHERE dismissed = 0
        ORDER BY created_at DESC
        LIMIT ? OFFSET ?
        "#
    };

    let rows = sqlx::query(query)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to get notifications: {}", e))?;

    Ok(rows.into_iter().map(|r| row_to_notification(r)).collect())
}

/// Get count of unread notifications
pub async fn get_unread_count(pool: &Pool<Sqlite>) -> Result<i64, String> {
    let row =
        sqlx::query("SELECT COUNT(*) as count FROM notifications WHERE read = 0 AND dismissed = 0")
            .fetch_one(pool)
            .await
            .map_err(|e| format!("Failed to get unread count: {}", e))?;

    Ok(row.get::<i64, _>("count"))
}

/// Mark a single notification as read
pub async fn mark_read(pool: &Pool<Sqlite>, id: i64) -> Result<(), String> {
    sqlx::query("UPDATE notifications SET read = 1 WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to mark notification read: {}", e))?;

    Ok(())
}

/// Mark all notifications as read
pub async fn mark_all_read(pool: &Pool<Sqlite>) -> Result<(), String> {
    sqlx::query("UPDATE notifications SET read = 1 WHERE read = 0")
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to mark all notifications read: {}", e))?;

    Ok(())
}

/// Dismiss a single notification
pub async fn dismiss(pool: &Pool<Sqlite>, id: i64) -> Result<(), String> {
    sqlx::query("UPDATE notifications SET dismissed = 1 WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to dismiss notification: {}", e))?;

    Ok(())
}

/// Dismiss all notifications
pub async fn dismiss_all(pool: &Pool<Sqlite>) -> Result<(), String> {
    sqlx::query("UPDATE notifications SET dismissed = 1 WHERE dismissed = 0")
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to dismiss all notifications: {}", e))?;

    Ok(())
}

/// Clear old history using the saved age limit, including read and unread items.
pub async fn clear_old_notifications(pool: &Pool<Sqlite>) -> Result<Vec<i64>, String> {
    clear_old_notifications_at(pool, Utc::now()).await
}

async fn clear_old_notifications_at(
    pool: &Pool<Sqlite>,
    now: DateTime<Utc>,
) -> Result<Vec<i64>, String> {
    let mut transaction = pool
        .begin()
        .await
        .map_err(|error| format!("Failed to clear old notifications: {}", error))?;
    // Read the policy in the same snapshot as the history being dismissed.
    let saved_days: Option<Option<String>> = sqlx::query_scalar(
        "SELECT value FROM settings WHERE key = 'notification_auto_clear_after_days'",
    )
    .fetch_optional(&mut *transaction)
    .await
    .map_err(|error| format!("Failed to read notification auto-clear setting: {}", error))?;
    let days = saved_days
        .flatten()
        .map(|value| {
            value
                .parse()
                .map_err(|_| "Invalid saved notification auto-clear setting.".to_string())
        })
        .transpose()?
        .unwrap_or(0);
    validate_auto_clear_after_days(days)?;
    if days == 0 {
        transaction
            .commit()
            .await
            .map_err(|error| format!("Failed to finish clearing old notifications: {}", error))?;
        return Ok(Vec::new());
    }
    let cutoff = now - Duration::days(i64::from(days));
    let rows =
        sqlx::query("SELECT id, created_at FROM notifications WHERE dismissed = 0 ORDER BY id")
            .fetch_all(&mut *transaction)
            .await
            .map_err(|error| format!("Failed to read old notifications: {}", error))?;
    let mut cleared = Vec::new();
    for row in rows {
        // SQLite's default timestamps are UTC without an offset. Exported/imported
        // timestamps may instead use RFC3339. Invalid dates stay in history.
        let Some(created_at) = row.get::<Option<String>, _>("created_at") else {
            continue;
        };
        let created_at = DateTime::parse_from_rfc3339(&created_at)
            .map(|date| date.with_timezone(&Utc))
            .ok()
            .or_else(|| {
                NaiveDateTime::parse_from_str(&created_at, "%Y-%m-%d %H:%M:%S%.f")
                    .ok()
                    .map(|date| date.and_utc())
            });
        if created_at.is_some_and(|date| date < cutoff) {
            let id = row.get::<i64, _>("id");
            sqlx::query("UPDATE notifications SET dismissed = 1 WHERE id = ?")
                .bind(id)
                .execute(&mut *transaction)
                .await
                .map_err(|error| format!("Failed to clear old notifications: {}", error))?;
            cleared.push(id);
        }
    }
    transaction
        .commit()
        .await
        .map_err(|error| format!("Failed to finish clearing old notifications: {}", error))?;
    Ok(cleared)
}

/// Check if a specific notification type is enabled
pub fn is_type_enabled(settings: &NotificationSettings, notification_type: &str) -> bool {
    if !settings.enabled {
        return false;
    }
    match notification_type {
        "racing" => settings.racing_enabled,
        "plex" => settings.plex_enabled,
        "premiere" => settings.premiere_enabled,
        "update" => settings.update_enabled,
        "system" => settings.system_enabled,
        _ => true,
    }
}

/// Central notification dispatch: creates DB entry, emits frontend event,
/// and sends OS notification when appropriate (os_fallback or tray mode).
pub async fn send_notification(
    app: &AppHandle,
    create: &CreateNotification,
) -> Result<Option<Notification>, String> {
    let pool = connection::get_pool(app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let settings = get_settings_checked(&pool).await?;

    // Check if this notification type is enabled
    if !is_type_enabled(&settings, &create.r#type) {
        return Ok(None);
    }

    // Create the DB entry (always, so it appears in notification center later)
    let notif = create_notification(&pool, create).await?;

    // Emit to frontend (in-app popup if window is visible)
    let _ = app.emit("notification", &notif);

    // Determine if we should send an OS notification
    let window_visible = app
        .get_webview_window("main")
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false);

    let send_os = !is_quiet_hours_at(&settings, Local::now().time())
        && (settings.os_fallback || (settings.tray_notifications && !window_visible));

    if send_os {
        let _ = app
            .notification()
            .builder()
            .title(&notif.title)
            .body(&notif.body)
            .show();
    }

    Ok(Some(notif))
}

fn row_to_notification(r: sqlx::sqlite::SqliteRow) -> Notification {
    Notification {
        id: r.get("id"),
        r#type: r.get("type"),
        title: r.get("title"),
        body: r.get("body"),
        icon: r.get("icon"),
        reference_id: r.get("reference_id"),
        reference_type: r.get("reference_type"),
        read: r.get::<i32, _>("read") == 1,
        dismissed: r.get::<i32, _>("dismissed") == 1,
        created_at: r.get("created_at"),
        expires_at: r.get("expires_at"),
    }
}

#[cfg(test)]
mod tests;
