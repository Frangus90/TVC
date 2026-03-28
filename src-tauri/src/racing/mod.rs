pub mod api;
pub mod models;
pub mod scheduler;

use sqlx::{Pool, Row, Sqlite};
use tauri::AppHandle;

use crate::db::connection;
use models::{RacingConfig, RacingEvent, RacingSeries};

/// Get racing config from database
pub async fn get_config(pool: &Pool<Sqlite>) -> RacingConfig {
    let row = sqlx::query(
        "SELECT notifications_enabled, default_notify_minutes, last_refreshed FROM racing_config WHERE id = 1",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    match row {
        Some(r) => RacingConfig {
            notifications_enabled: r.get::<i32, _>("notifications_enabled") == 1,
            default_notify_minutes: r.get::<i32, _>("default_notify_minutes"),
            last_refreshed: r.get::<Option<String>, _>("last_refreshed"),
        },
        None => RacingConfig::default(),
    }
}

/// Update racing config
pub async fn update_config(pool: &Pool<Sqlite>, config: &RacingConfig) -> Result<(), String> {
    sqlx::query(
        r#"
        UPDATE racing_config
        SET notifications_enabled = ?, default_notify_minutes = ?, updated_at = datetime('now')
        WHERE id = 1
        "#,
    )
    .bind(if config.notifications_enabled { 1 } else { 0 })
    .bind(config.default_notify_minutes)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to update racing config: {}", e))?;

    Ok(())
}

/// Get all racing series
pub async fn get_all_series(pool: &Pool<Sqlite>) -> Result<Vec<RacingSeries>, String> {
    let rows = sqlx::query(
        "SELECT id, slug, name, category, ics_url, custom_ics_url, enabled, notify_enabled, notify_minutes, color, custom_color FROM racing_series ORDER BY category, name",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to get racing series: {}", e))?;

    Ok(rows
        .into_iter()
        .map(|r| RacingSeries {
            id: r.get("id"),
            slug: r.get("slug"),
            name: r.get("name"),
            category: r.get("category"),
            ics_url: r.get("ics_url"),
            custom_ics_url: r.get("custom_ics_url"),
            enabled: r.get::<i32, _>("enabled") == 1,
            notify_enabled: r.get::<i32, _>("notify_enabled") == 1,
            notify_minutes: r.get("notify_minutes"),
            color: r.get("color"),
            custom_color: r.get("custom_color"),
        })
        .collect())
}

/// Get only enabled series
pub async fn get_enabled_series(pool: &Pool<Sqlite>) -> Result<Vec<RacingSeries>, String> {
    let all = get_all_series(pool).await?;
    Ok(all.into_iter().filter(|s| s.enabled).collect())
}

/// Toggle a series enabled/disabled
pub async fn toggle_series(pool: &Pool<Sqlite>, slug: &str, enabled: bool) -> Result<(), String> {
    sqlx::query("UPDATE racing_series SET enabled = ? WHERE slug = ?")
        .bind(if enabled { 1 } else { 0 })
        .bind(slug)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to toggle series: {}", e))?;

    Ok(())
}

/// Update series color
pub async fn update_series_color(
    pool: &Pool<Sqlite>,
    slug: &str,
    color: Option<&str>,
) -> Result<(), String> {
    sqlx::query("UPDATE racing_series SET custom_color = ? WHERE slug = ?")
        .bind(color)
        .bind(slug)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to update series color: {}", e))?;

    Ok(())
}

/// Update series notification settings
pub async fn update_series_notification(
    pool: &Pool<Sqlite>,
    slug: &str,
    notify_enabled: bool,
    notify_minutes: i32,
) -> Result<(), String> {
    sqlx::query("UPDATE racing_series SET notify_enabled = ?, notify_minutes = ? WHERE slug = ?")
        .bind(if notify_enabled { 1 } else { 0 })
        .bind(notify_minutes)
        .bind(slug)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to update notification settings: {}", e))?;

    Ok(())
}

/// Update series custom ICS URL
pub async fn update_series_ics_url(
    pool: &Pool<Sqlite>,
    slug: &str,
    custom_url: Option<&str>,
) -> Result<(), String> {
    sqlx::query("UPDATE racing_series SET custom_ics_url = ? WHERE slug = ?")
        .bind(custom_url)
        .bind(slug)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to update ICS URL: {}", e))?;

    Ok(())
}

/// Get racing events for a date range (for calendar view)
pub async fn get_events_for_range(
    pool: &Pool<Sqlite>,
    start: &str,
    end: &str,
) -> Result<Vec<RacingEvent>, String> {
    let rows = sqlx::query(
        r#"
        SELECT e.id, e.series_slug, e.uid, e.event_title, e.session_name, e.circuit,
               e.start_time, e.end_time, e.description, e.notified
        FROM racing_events e
        JOIN racing_series s ON e.series_slug = s.slug
        WHERE s.enabled = 1 AND e.start_time >= ? AND e.start_time <= ?
        ORDER BY e.start_time
        "#,
    )
    .bind(start)
    .bind(end)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to get racing events: {}", e))?;

    Ok(rows.into_iter().map(row_to_event).collect())
}

/// Mark an event as notified
pub async fn mark_notified(pool: &Pool<Sqlite>, event_id: i64) -> Result<(), String> {
    sqlx::query("UPDATE racing_events SET notified = 1 WHERE id = ?")
        .bind(event_id)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to mark event notified: {}", e))?;

    Ok(())
}

/// Delete all events for a series (before re-import)
pub async fn delete_events_for_series(pool: &Pool<Sqlite>, slug: &str) -> Result<(), String> {
    sqlx::query("DELETE FROM racing_events WHERE series_slug = ?")
        .bind(slug)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to delete events: {}", e))?;

    Ok(())
}

/// Insert events (using INSERT OR REPLACE for upsert by unique constraint)
pub async fn upsert_events(pool: &Pool<Sqlite>, events: &[RacingEvent]) -> Result<(), String> {
    for event in events {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO racing_events
                (series_slug, uid, event_title, session_name, circuit, start_time, end_time, description, notified, fetched_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, 0, datetime('now'))
            "#,
        )
        .bind(&event.series_slug)
        .bind(&event.uid)
        .bind(&event.event_title)
        .bind(&event.session_name)
        .bind(&event.circuit)
        .bind(&event.start_time)
        .bind(&event.end_time)
        .bind(&event.description)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to upsert event: {}", e))?;
    }

    Ok(())
}

/// Refresh data for a single series: fetch ICS, parse, store
pub async fn refresh_series(pool: &Pool<Sqlite>, series: &RacingSeries) -> Result<usize, String> {
    let url = series
        .custom_ics_url
        .as_deref()
        .unwrap_or(&series.ics_url);

    let ics_text = api::fetch_ics(url).await?;
    let events = api::parse_ics(&ics_text, &series.slug);
    let count = events.len();

    // Clear old events and insert fresh
    delete_events_for_series(pool, &series.slug).await?;
    upsert_events(pool, &events).await?;

    Ok(count)
}

/// Refresh all enabled series
pub async fn refresh_all_enabled(pool: &Pool<Sqlite>) -> Result<usize, String> {
    let series_list = get_enabled_series(pool).await?;
    let mut total = 0;

    for series in &series_list {
        match refresh_series(pool, series).await {
            Ok(count) => {
                total += count;
            }
            Err(e) => {
                eprintln!(
                    "[Racing] Failed to refresh {}: {}",
                    series.name, e
                );
            }
        }

        // Small delay between requests to be polite
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }

    // Update last_refreshed timestamp
    sqlx::query("UPDATE racing_config SET last_refreshed = datetime('now'), updated_at = datetime('now') WHERE id = 1")
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to update refresh timestamp: {}", e))?;

    Ok(total)
}

/// Auto-start notification scheduler on app launch
pub async fn auto_start_scheduler(app: AppHandle) {
    // Wait for SQL plugin to finish applying migrations before querying racing tables
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let pool = match connection::get_pool(&app).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[Racing] Failed to get database pool: {}", e);
            return;
        }
    };

    let config = get_config(&pool).await;

    if config.notifications_enabled {
        scheduler::start_scheduler(app).await;
    }
}

fn row_to_event(r: sqlx::sqlite::SqliteRow) -> RacingEvent {
    RacingEvent {
        id: r.get("id"),
        series_slug: r.get("series_slug"),
        uid: r.get("uid"),
        event_title: r.get("event_title"),
        session_name: r.get("session_name"),
        circuit: r.get("circuit"),
        start_time: r.get("start_time"),
        end_time: r.get("end_time"),
        description: r.get("description"),
        notified: r.get::<i32, _>("notified") == 1,
    }
}
