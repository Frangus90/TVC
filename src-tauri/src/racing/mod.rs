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
        "SELECT id, slug, name, category, ics_url, fallback_ics_url, custom_ics_url, enabled, notify_enabled, notify_minutes, color, custom_color FROM racing_series ORDER BY category, name",
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
            fallback_ics_url: r.get("fallback_ics_url"),
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
    let start_time =
        chrono::DateTime::parse_from_rfc3339(start).map_err(|_| "Invalid range start")?;
    let end_time = chrono::DateTime::parse_from_rfc3339(end).map_err(|_| "Invalid range end")?;
    if start_time >= end_time {
        return Err("Range end must follow start".into());
    }
    let rows = sqlx::query(
        r#"
        SELECT e.id, e.series_slug, e.uid, e.event_title, e.session_name, e.circuit,
               e.start_time, e.end_time, e.description, e.notified
        FROM racing_events e
        JOIN racing_series s ON e.series_slug = s.slug
        WHERE s.enabled = 1 AND julianday(e.start_time) >= julianday(?) AND julianday(e.start_time) < julianday(?)
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

/// Replace a validated feed atomically; retain identities and notification state for unchanged times.
pub(crate) async fn replace_events(
    pool: &Pool<Sqlite>,
    slug: &str,
    events: &[RacingEvent],
) -> Result<(), String> {
    if events.is_empty() || events.iter().any(|event| event.series_slug != slug) {
        return Err("Refusing an empty or mixed-series feed".into());
    }
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let old: Vec<(i64, String)> =
        sqlx::query_as("SELECT id, uid FROM racing_events WHERE series_slug = ?")
            .bind(slug)
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
    for event in events {
        sqlx::query("INSERT INTO racing_events (series_slug, uid, event_title, session_name, circuit, start_time, end_time, description, notified, fetched_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, 0, datetime('now'))
            ON CONFLICT(series_slug, uid) DO UPDATE SET
            event_title = excluded.event_title, session_name = excluded.session_name, circuit = excluded.circuit,
            notified = CASE WHEN racing_events.start_time = excluded.start_time THEN racing_events.notified ELSE 0 END,
            start_time = excluded.start_time, end_time = excluded.end_time, description = excluded.description, fetched_at = excluded.fetched_at")
            .bind(slug).bind(&event.uid).bind(&event.event_title).bind(&event.session_name).bind(&event.circuit)
            .bind(&event.start_time).bind(&event.end_time).bind(&event.description)
            .execute(&mut *tx).await.map_err(|e| format!("Failed to save feed: {e}"))?;
    }
    for (id, uid) in old {
        if !events.iter().any(|event| event.uid == uid) {
            sqlx::query("DELETE FROM racing_events WHERE id = ?")
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
        }
    }
    tx.commit().await.map_err(|e| e.to_string())
}

/// Refresh data for a single series: fetch ICS, parse, store.
///
/// A user-set `custom_ics_url` overrides everything. Otherwise the primary
/// `ics_url` is tried first and the `fallback_ics_url` is attempted only if
/// the primary fetch fails (network error or non-2xx).
pub async fn refresh_series(pool: &Pool<Sqlite>, series: &RacingSeries) -> Result<usize, String> {
    let urls: Vec<&str> = if let Some(custom) = series.custom_ics_url.as_deref() {
        vec![custom]
    } else {
        let mut v = vec![series.ics_url.as_str()];
        if let Some(fb) = series.fallback_ics_url.as_deref() {
            v.push(fb);
        }
        v
    };

    let mut last_err = String::from("no ICS URL configured");
    for url in &urls {
        match api::fetch_ics(url).await {
            Ok(ics_text) => {
                let events = match api::parse_ics(&ics_text, &series.slug) {
                    Ok(events) => events,
                    Err(error) => {
                        last_err = error;
                        continue;
                    }
                };
                let count = events.len();
                replace_events(pool, &series.slug, &events).await?;
                return Ok(count);
            }
            Err(e) => {
                eprintln!("[Racing] {} fetch failed ({}): {}", series.name, url, e);
                last_err = e;
            }
        }
    }
    Err(last_err)
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct RefreshReport {
    pub attempted_at: String,
    pub events: usize,
    pub succeeded: usize,
    pub failures: Vec<RefreshFailure>,
}
#[derive(serde::Serialize, serde::Deserialize)]
pub struct RefreshFailure {
    pub slug: String,
    pub name: String,
    pub error: String,
}

pub async fn refresh_all_enabled(pool: &Pool<Sqlite>) -> Result<RefreshReport, String> {
    let series_list = get_enabled_series(pool).await?;
    let mut report = RefreshReport {
        attempted_at: chrono::Utc::now().to_rfc3339(),
        ..Default::default()
    };
    for series in &series_list {
        match refresh_series(pool, series).await {
            Ok(count) => {
                report.events += count;
                report.succeeded += 1;
            }
            Err(error) => report.failures.push(RefreshFailure {
                slug: series.slug.clone(),
                name: series.name.clone(),
                error,
            }),
        }
    }
    if report.failures.is_empty() && report.succeeded > 0 {
        sqlx::query("UPDATE racing_config SET last_refreshed = ?, updated_at = datetime('now') WHERE id = 1")
            .bind(&report.attempted_at).execute(pool).await.map_err(|e| e.to_string())?;
    }
    sqlx::query("INSERT OR REPLACE INTO settings (key, value) VALUES ('racing_refresh_report', ?)")
        .bind(serde_json::to_string(&report).map_err(|e| e.to_string())?)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(report)
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

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn all_failed_refreshes_keep_last_success_and_persist_named_errors() {
        let pool = crate::audit_tests::database().await;
        sqlx::raw_sql("UPDATE racing_series SET enabled=0; UPDATE racing_series SET enabled=1,custom_ics_url='invalid://fixture' WHERE slug='f1'; UPDATE racing_config SET last_refreshed='2026-01-01T12:00:00Z';")
            .execute(&pool).await.unwrap();
        let report = super::refresh_all_enabled(&pool).await.unwrap();
        assert_eq!(report.succeeded, 0);
        assert_eq!(report.failures.len(), 1);
        assert_eq!(report.failures[0].slug, "f1");
        assert_eq!(
            super::get_config(&pool).await.last_refreshed.as_deref(),
            Some("2026-01-01T12:00:00Z")
        );
        let saved: String =
            sqlx::query_scalar("SELECT value FROM settings WHERE key='racing_refresh_report'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(saved.contains("Formula 1"));
        assert!(saved.contains(&report.attempted_at));
    }
}
