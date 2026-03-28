pub mod models;

use sqlx::{Pool, Row, Sqlite};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::db::connection;
use models::{CreateNotification, Notification, NotificationSettings};

/// Get notification settings from database
pub async fn get_settings(pool: &Pool<Sqlite>) -> NotificationSettings {
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
    .ok()
    .flatten();

    match row {
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
        },
        None => NotificationSettings::default(),
    }
}

/// Update notification settings
pub async fn update_settings(
    pool: &Pool<Sqlite>,
    settings: &NotificationSettings,
) -> Result<(), String> {
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
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to update notification settings: {}", e))?;

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
    let row = sqlx::query("SELECT COUNT(*) as count FROM notifications WHERE read = 0 AND dismissed = 0")
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

    let settings = get_settings(&pool).await;

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

    let send_os = settings.os_fallback
        || (settings.tray_notifications && !window_visible);

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
