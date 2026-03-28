use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

use crate::db::connection;
use crate::racing::{self, models::{RacingConfig, RacingEvent, RacingSeries}};

#[tauri::command]
pub async fn get_racing_series(app: AppHandle) -> Result<Vec<RacingSeries>, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    racing::get_all_series(&pool).await
}

#[tauri::command]
pub async fn toggle_racing_series(
    app: AppHandle,
    slug: String,
    enabled: bool,
) -> Result<(), String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    racing::toggle_series(&pool, &slug, enabled).await
}

#[tauri::command]
pub async fn update_racing_series_color(
    app: AppHandle,
    slug: String,
    color: Option<String>,
) -> Result<(), String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    racing::update_series_color(&pool, &slug, color.as_deref()).await
}

#[tauri::command]
pub async fn update_racing_series_notification(
    app: AppHandle,
    slug: String,
    notify_enabled: bool,
    notify_minutes: i32,
) -> Result<(), String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    racing::update_series_notification(&pool, &slug, notify_enabled, notify_minutes).await
}

#[tauri::command]
pub async fn update_racing_series_ics_url(
    app: AppHandle,
    slug: String,
    custom_url: Option<String>,
) -> Result<(), String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    racing::update_series_ics_url(&pool, &slug, custom_url.as_deref()).await
}

#[tauri::command]
pub async fn get_racing_events_for_range(
    app: AppHandle,
    start: String,
    end: String,
) -> Result<Vec<RacingEvent>, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    racing::get_events_for_range(&pool, &start, &end).await
}

#[tauri::command]
pub async fn refresh_racing_data(app: AppHandle) -> Result<usize, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let count = racing::refresh_all_enabled(&pool).await?;

    // Reschedule notifications after data refresh
    racing::scheduler::reschedule(app).await;

    Ok(count)
}

#[tauri::command]
pub async fn refresh_single_racing_series(app: AppHandle, slug: String) -> Result<usize, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let series_list = racing::get_all_series(&pool).await?;
    let series = series_list
        .iter()
        .find(|s| s.slug == slug)
        .ok_or_else(|| format!("Series '{}' not found", slug))?;

    let count = racing::refresh_series(&pool, series).await?;

    // Reschedule notifications
    racing::scheduler::reschedule(app).await;

    Ok(count)
}

#[tauri::command]
pub async fn get_racing_config(app: AppHandle) -> Result<RacingConfig, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    Ok(racing::get_config(&pool).await)
}

#[tauri::command]
pub async fn update_racing_config(
    app: AppHandle,
    notifications_enabled: bool,
    default_notify_minutes: i32,
) -> Result<(), String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let config = RacingConfig {
        notifications_enabled,
        default_notify_minutes,
        last_refreshed: None, // Don't overwrite last_refreshed
    };

    racing::update_config(&pool, &config).await?;

    // Reschedule if notifications state changed
    if notifications_enabled {
        racing::scheduler::reschedule(app).await;
    } else {
        racing::scheduler::cancel_all().await;
    }

    Ok(())
}

/// Dev-only: fire a test notification immediately to verify the notification system works
#[tauri::command]
pub async fn test_racing_notification(app: AppHandle) -> Result<(), String> {
    app.notification()
        .builder()
        .title("TVC Racing: Test Notification")
        .body("If you see this, racing notifications are working!")
        .show()
        .map_err(|e| format!("Failed to send notification: {}", e))?;

    Ok(())
}
