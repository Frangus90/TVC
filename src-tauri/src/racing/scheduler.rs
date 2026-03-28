use chrono::Utc;
use sqlx::Row;
use tauri::AppHandle;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::db::connection;
use crate::notifications;

/// Handles for all pending notification tasks
static SCHEDULER_HANDLES: Mutex<Vec<JoinHandle<()>>> = Mutex::const_new(Vec::new());

/// Start the notification scheduler — loads upcoming events and spawns timers
pub async fn start_scheduler(app: AppHandle) {
    let pool = match connection::get_pool(&app).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[Racing] Scheduler failed to get pool: {}", e);
            return;
        }
    };

    // Get upcoming events with their series notification settings
    let rows = sqlx::query(
        r#"
        SELECT e.id, e.event_title, e.session_name, e.circuit, e.start_time,
               s.name as series_name, s.notify_minutes
        FROM racing_events e
        JOIN racing_series s ON e.series_slug = s.slug
        WHERE s.enabled = 1 AND s.notify_enabled = 1 AND e.notified = 0
              AND e.start_time > datetime('now')
        ORDER BY e.start_time
        "#,
    )
    .fetch_all(&pool)
    .await;

    let rows = match rows {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[Racing] Failed to load events for scheduling: {}", e);
            return;
        }
    };

    let mut handles = SCHEDULER_HANDLES.lock().await;

    for row in rows {
        let event_id: i64 = row.get("id");
        let event_title: String = row.get("event_title");
        let session_name: Option<String> = row.get("session_name");
        let circuit: Option<String> = row.get("circuit");
        let start_time_str: String = row.get("start_time");
        let series_name: String = row.get("series_name");
        let notify_minutes: i32 = row.get("notify_minutes");

        // Parse the start time
        let start_time = match parse_event_time(&start_time_str) {
            Some(t) => t,
            None => continue,
        };

        // Calculate when to fire the notification
        let fire_time = start_time - chrono::Duration::minutes(notify_minutes as i64);
        let now = Utc::now();

        if fire_time <= now {
            // Notification window has passed, skip
            continue;
        }

        let delay = (fire_time - now).to_std().unwrap_or_default();

        let app_clone = app.clone();
        let handle = tokio::spawn(async move {
            tokio::time::sleep(delay).await;

            // Build notification content
            let session_label = session_name
                .as_deref()
                .unwrap_or("Session");
            let title = format!("{}: {} Starting Soon", series_name, session_label);
            let body = match circuit.as_deref() {
                Some(c) => format!(
                    "{} at {} starts in {} minutes",
                    event_title, c, notify_minutes
                ),
                None => format!(
                    "{} starts in {} minutes",
                    event_title, notify_minutes
                ),
            };

            // Send via central dispatch (handles in-app, OS, and tray logic)
            let _ = notifications::send_notification(
                &app_clone,
                &notifications::models::CreateNotification {
                    r#type: "racing".to_string(),
                    title,
                    body,
                    icon: None,
                    reference_id: Some(event_id.to_string()),
                    reference_type: Some("racing_event".to_string()),
                    expires_at: None,
                },
            )
            .await;

            // Mark racing event as notified
            if let Ok(pool) = connection::get_pool(&app_clone).await {
                let _ = super::mark_notified(&pool, event_id).await;
            }
        });

        handles.push(handle);
    }

    let count = handles.len();
    if count > 0 {
        eprintln!("[Racing] Scheduled {} notification(s)", count);
    }
}

/// Cancel all pending notification timers
pub async fn cancel_all() {
    let mut handles = SCHEDULER_HANDLES.lock().await;
    for handle in handles.drain(..) {
        handle.abort();
    }
}

/// Reschedule all notifications (cancel existing, reload from DB)
pub async fn reschedule(app: AppHandle) {
    cancel_all().await;
    start_scheduler(app).await;
}

/// Parse an event time string (ISO 8601) to a DateTime<Utc>
fn parse_event_time(time_str: &str) -> Option<chrono::DateTime<Utc>> {
    // Try full datetime with Z suffix
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(time_str, "%Y-%m-%dT%H:%M:%SZ") {
        return Some(dt.and_utc());
    }

    // Try without Z
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(time_str, "%Y-%m-%dT%H:%M:%S") {
        return Some(dt.and_utc());
    }

    // Try date-only (assume midnight UTC)
    if let Ok(d) = chrono::NaiveDate::parse_from_str(time_str, "%Y-%m-%d") {
        let dt = d
            .and_hms_opt(0, 0, 0)
            .expect("midnight is valid");
        return Some(dt.and_utc());
    }

    eprintln!("[Racing] Could not parse event time: {}", time_str);
    None
}
