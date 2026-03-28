use tauri::AppHandle;

use crate::db::connection;
use crate::notifications::{self, models::{CreateNotification, Notification, NotificationSettings}};

#[tauri::command]
pub async fn get_notification_settings(app: AppHandle) -> Result<NotificationSettings, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    Ok(notifications::get_settings(&pool).await)
}

#[tauri::command]
pub async fn update_notification_settings(
    app: AppHandle,
    settings: NotificationSettings,
) -> Result<(), String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    notifications::update_settings(&pool, &settings).await
}

#[tauri::command]
pub async fn get_notifications(
    app: AppHandle,
    limit: Option<i32>,
    offset: Option<i32>,
    unread_only: Option<bool>,
) -> Result<Vec<Notification>, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    notifications::get_notifications(
        &pool,
        limit.unwrap_or(50),
        offset.unwrap_or(0),
        unread_only.unwrap_or(false),
    )
    .await
}

#[tauri::command]
pub async fn get_unread_notification_count(app: AppHandle) -> Result<i64, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    notifications::get_unread_count(&pool).await
}

#[tauri::command]
pub async fn mark_notification_read(app: AppHandle, id: i64) -> Result<(), String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    notifications::mark_read(&pool, id).await
}

#[tauri::command]
pub async fn mark_all_notifications_read(app: AppHandle) -> Result<(), String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    notifications::mark_all_read(&pool).await
}

#[tauri::command]
pub async fn dismiss_notification(app: AppHandle, id: i64) -> Result<(), String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    notifications::dismiss(&pool, id).await
}

#[tauri::command]
pub async fn dismiss_all_notifications(app: AppHandle) -> Result<(), String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    notifications::dismiss_all(&pool).await
}

#[tauri::command]
pub async fn test_in_app_notification(
    app: AppHandle,
    notification_type: Option<String>,
) -> Result<(), String> {
    let notif_type = notification_type.as_deref().unwrap_or("system");

    let (title, body, ref_type) = match notif_type {
        "racing" => (
            "F1: Race Starting Soon",
            "Monaco Grand Prix at Circuit de Monaco starts in 30 minutes",
            Some("racing_event"),
        ),
        "plex" => (
            "Plex: Marked episode as watched",
            "The Bear S03E01",
            Some("episode"),
        ),
        "premiere" => (
            "Premiere Tonight",
            "Severance S02E01 airs at 9:00 PM",
            Some("episode"),
        ),
        "update" => (
            "Update Available",
            "TVC v0.9.3 is ready to install",
            Some("app"),
        ),
        _ => (
            "Test Notification",
            "If you see this, in-app notifications are working!",
            None,
        ),
    };

    notifications::send_notification(
        &app,
        &CreateNotification {
            r#type: notif_type.to_string(),
            title: title.to_string(),
            body: body.to_string(),
            icon: None,
            reference_id: None,
            reference_type: ref_type.map(|s| s.to_string()),
            expires_at: None,
        },
    )
    .await?;

    Ok(())
}
