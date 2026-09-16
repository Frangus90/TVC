//! Bulk TMDB refreshes and opt-in schedules. All state uses the active app database.
use chrono::{DateTime, Duration, Months, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sqlx::SqlitePool;
use std::{
    future::Future,
    sync::atomic::{AtomicU8, Ordering},
};
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

use crate::{commands, db::connection};

static SYNC_LOCK: Mutex<()> = Mutex::const_new(());
static ACTIVE: AtomicU8 = AtomicU8::new(0);

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Library {
    Shows,
    Movies,
}

impl Library {
    fn key(self, suffix: &str) -> String {
        format!(
            "library_sync_{}_{}",
            match self {
                Self::Shows => "shows",
                Self::Movies => "movies",
            },
            suffix
        )
    }
    fn code(self) -> u8 {
        match self {
            Self::Shows => 1,
            Self::Movies => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Frequency {
    #[default]
    Off,
    Daily,
    Weekly,
    Monthly,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Failure {
    id: i64,
    title: String,
    error: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Report {
    kind: Library,
    source: String,
    started_at: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
    total: usize,
    succeeded: usize,
    skipped: usize,
    failures: Vec<Failure>,
    current_title: Option<String>,
    error: Option<String>,
}

#[derive(Serialize)]
pub struct Status {
    kind: Library,
    frequency: Frequency,
    next_sync_at: Option<DateTime<Utc>>,
    running: bool,
    interrupted: bool,
    report: Option<Report>,
}

// Reset even if a task is cancelled or exits early.
struct ActiveRun;
impl Drop for ActiveRun {
    fn drop(&mut self) {
        ACTIVE.store(0, Ordering::SeqCst);
    }
}

async fn read_setting<T: DeserializeOwned>(
    pool: &SqlitePool,
    key: &str,
) -> Result<Option<T>, String> {
    let value: Option<String> = sqlx::query_scalar("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Cannot read sync settings: {e}"))?;
    value
        .map(|value| {
            serde_json::from_str(&value).map_err(|e| format!("Invalid sync setting {key}: {e}"))
        })
        .transpose()
}

async fn save_report(pool: &SqlitePool, report: &Report) -> Result<(), String> {
    sqlx::query("INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)")
        .bind(report.kind.key("report"))
        .bind(serde_json::to_string(report).map_err(|e| e.to_string())?)
        .execute(pool)
        .await
        .map_err(|e| format!("Cannot save sync results: {e}"))?;
    Ok(())
}

fn next_due(
    frequency: Frequency,
    last: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    if frequency == Frequency::Off {
        return None;
    }
    let Some(last) = last else {
        return Some(now);
    };
    match frequency {
        Frequency::Off => None,
        Frequency::Daily => Some(last + Duration::days(1)),
        Frequency::Weekly => Some(last + Duration::weeks(1)),
        Frequency::Monthly => last.checked_add_months(Months::new(1)),
    }
}

async fn status(pool: &SqlitePool, kind: Library) -> Result<Status, String> {
    let frequency = read_setting(pool, &kind.key("frequency"))
        .await?
        .unwrap_or_default();
    let last = read_setting(pool, &kind.key("last_full_attempt")).await?;
    let report: Option<Report> = read_setting(pool, &kind.key("report")).await?;
    let running = ACTIVE.load(Ordering::SeqCst) == kind.code();
    let interrupted = !running && report.as_ref().is_some_and(|r| r.finished_at.is_none());
    Ok(Status {
        kind,
        frequency,
        running,
        interrupted,
        next_sync_at: next_due(frequency, if interrupted { None } else { last }, Utc::now()),
        report,
    })
}

#[tauri::command]
pub async fn get_library_sync_status(app: AppHandle) -> Result<Vec<Status>, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| e.to_string())?;
    Ok(vec![
        status(&pool, Library::Shows).await?,
        status(&pool, Library::Movies).await?,
    ])
}

#[tauri::command]
pub async fn set_library_sync_schedule(
    app: AppHandle,
    kind: Library,
    frequency: Frequency,
) -> Result<(), String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)")
        .bind(kind.key("frequency"))
        .bind(serde_json::to_string(&frequency).map_err(|e| e.to_string())?)
        .execute(&pool)
        .await
        .map_err(|e| format!("Cannot save sync schedule: {e}"))?;
    Ok(())
}

// The callback keeps network requests separate from report persistence, so failure/retry
// behavior can be tested against a disposable database without contacting TMDB.
async fn sync_library<F, Fut>(
    pool: &SqlitePool,
    kind: Library,
    source: &str,
    mut sync_item: F,
) -> Result<Report, String>
where
    F: FnMut(i64) -> Fut,
    Fut: Future<Output = Result<(), String>>,
{
    let retry_ids = if source == "retry" {
        let previous: Report = read_setting(pool, &kind.key("report"))
            .await?
            .ok_or("No previous sync to retry")?;
        Some(
            previous
                .failures
                .into_iter()
                .map(|f| f.id)
                .collect::<Vec<_>>(),
        )
    } else {
        None
    };
    let mut report = Report {
        kind,
        source: source.into(),
        started_at: Utc::now(),
        finished_at: None,
        total: 0,
        succeeded: 0,
        skipped: 0,
        failures: vec![],
        current_title: None,
        error: None,
    };
    // Store the attempt and report together. A failed run won't retry every scheduler tick.
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    if retry_ids.is_none() {
        sqlx::query("INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)")
            .bind(kind.key("last_full_attempt"))
            .bind(serde_json::to_string(&report.started_at).map_err(|e| e.to_string())?)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
    }
    sqlx::query("INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)")
        .bind(kind.key("report"))
        .bind(serde_json::to_string(&report).map_err(|e| e.to_string())?)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;

    let result: Result<(), String> = async {
        let query = match kind {
            Library::Shows => "SELECT id, name, id > 0 AND unmigrated = 0 FROM shows ORDER BY name",
            Library::Movies => "SELECT id, title, id > 0 FROM movies ORDER BY title",
        };
        let mut items: Vec<(i64, String, bool)> = sqlx::query_as(query)
            .fetch_all(pool)
            .await
            .map_err(|e| format!("Cannot read library: {e}"))?;
        if let Some(ids) = retry_ids {
            items.retain(|item| ids.contains(&item.0));
        }
        report.total = items.len();
        for (id, title, eligible) in items {
            report.current_title = Some(title.clone());
            save_report(pool, &report).await?;
            if eligible {
                match sync_item(id).await {
                    Ok(()) => report.succeeded += 1,
                    Err(error) => report.failures.push(Failure { id, title, error }),
                }
            } else {
                report.skipped += 1;
            }
            report.current_title = None;
            save_report(pool, &report).await?;
        }
        Ok(())
    }
    .await;
    report.finished_at = Some(Utc::now());
    report.current_title = None;
    report.error = result.err();
    save_report(pool, &report).await?;
    Ok(report)
}

async fn run_one(
    app: &AppHandle,
    pool: &SqlitePool,
    kind: Library,
    source: &str,
) -> Result<(), String> {
    ACTIVE.store(kind.code(), Ordering::SeqCst);
    let _active = ActiveRun;
    let report = sync_library(pool, kind, source, |id| async move {
        let result = match kind {
            Library::Shows => commands::episodes::sync_show_episodes(app.clone(), id).await,
            Library::Movies => commands::movies::sync_movie(app.clone(), id).await,
        };
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        result
    })
    .await;
    // Even if saving the final report fails, completed items need a frontend refresh.
    app.emit("library-sync-finished", kind)
        .map_err(|e| format!("Cannot notify the UI of sync completion: {e}"))?;
    let report = report?;
    if let Some(error) = report.error {
        return Err(error);
    }
    Ok(())
}

#[tauri::command]
pub async fn run_library_sync(
    app: AppHandle,
    kind: Option<Library>,
    retry_failed: bool,
) -> Result<(), String> {
    let _guard = SYNC_LOCK
        .try_lock()
        .map_err(|_| "A library sync is already running")?;
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| e.to_string())?;
    if retry_failed && kind.is_none() {
        return Err("Choose a library to retry".into());
    }
    let kinds = kind.map_or_else(|| vec![Library::Shows, Library::Movies], |kind| vec![kind]);
    let mut errors = vec![];
    for kind in kinds {
        if let Err(error) = run_one(
            &app,
            &pool,
            kind,
            if retry_failed { "retry" } else { "manual" },
        )
        .await
        {
            errors.push(format!("{kind:?}: {error}"));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

async fn sync_due(app: &AppHandle) -> Result<(), String> {
    let Ok(_guard) = SYNC_LOCK.try_lock() else {
        return Ok(());
    };
    let pool = connection::get_pool(app).await.map_err(|e| e.to_string())?;
    for kind in [Library::Shows, Library::Movies] {
        let status = status(&pool, kind).await?;
        if status.next_sync_at.is_some_and(|due| due <= Utc::now()) {
            if let Err(error) = run_one(app, &pool, kind, "automatic").await {
                // Reports expose per-run failures in Data > Sync. Storage failures are
                // retried on the next tick; never pretend they were a successful sync.
                eprintln!("[library-sync] {kind:?}: {error}");
            }
        }
    }
    Ok(())
}

pub async fn start_scheduler(app: AppHandle) {
    loop {
        if let Err(error) = sync_due(&app).await {
            eprintln!("[library-sync] Cannot check schedules: {error}");
        }
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    async fn database() -> SqlitePool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::raw_sql(
            "CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            CREATE TABLE shows (id INTEGER PRIMARY KEY, name TEXT, unmigrated INTEGER DEFAULT 0);
            CREATE TABLE movies (id INTEGER PRIMARY KEY, title TEXT);
            INSERT INTO shows (id, name) VALUES (1, 'Alpha'), (2, 'Beta'), (-1, 'Manual');
            INSERT INTO shows (id, name, unmigrated) VALUES (3, 'Unmapped', 1);
            INSERT INTO movies VALUES (4, 'Movie'), (-2, 'Manual movie');",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool
    }

    fn at(value: &str) -> DateTime<Utc> {
        value.parse().unwrap()
    }

    #[test]
    fn intervals_and_month_end() {
        let start = at("2028-01-31T12:30:00Z");
        assert_eq!(next_due(Frequency::Off, None, start), None);
        assert_eq!(next_due(Frequency::Daily, None, start), Some(start));
        assert_eq!(
            next_due(Frequency::Daily, Some(start), start),
            Some(at("2028-02-01T12:30:00Z"))
        );
        assert_eq!(
            next_due(Frequency::Weekly, Some(start), start),
            Some(at("2028-02-07T12:30:00Z"))
        );
        assert_eq!(
            next_due(Frequency::Monthly, Some(start), start),
            Some(at("2028-02-29T12:30:00Z"))
        );
        let non_leap = at("2027-01-31T12:30:00Z");
        assert_eq!(
            next_due(Frequency::Monthly, Some(non_leap), non_leap),
            Some(at("2027-02-28T12:30:00Z"))
        );
    }

    #[tokio::test]
    async fn partial_failures_persist_and_retry_only_failed_titles() {
        let pool = database().await;
        let mut visited = vec![];
        let report = sync_library(&pool, Library::Shows, "manual", |id| {
            visited.push(id);
            async move {
                if id == 1 {
                    Err("TMDB timed out".into())
                } else {
                    Ok(())
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(visited, vec![1, 2]); // manual and unresolved rows never hit TMDB
        assert_eq!(
            (
                report.total,
                report.succeeded,
                report.skipped,
                report.failures.len()
            ),
            (4, 1, 2, 1)
        );
        let saved = status(&pool, Library::Shows).await.unwrap();
        assert_eq!(saved.frequency, Frequency::Off);
        assert!(saved.next_sync_at.is_none());
        let saved_report = saved.report.unwrap();
        assert_eq!(saved_report.failures[0].title, "Alpha");
        assert_eq!(saved_report.failures[0].error, "TMDB timed out");
        assert!(saved_report.finished_at.is_some());
        let anchor: Option<DateTime<Utc>> =
            read_setting(&pool, &Library::Shows.key("last_full_attempt"))
                .await
                .unwrap();
        let mut retried = vec![];
        let retry = sync_library(&pool, Library::Shows, "retry", |id| {
            retried.push(id);
            async { Ok(()) }
        })
        .await
        .unwrap();
        assert_eq!(retried, vec![1]);
        assert_eq!(
            (retry.total, retry.succeeded, retry.failures.len()),
            (1, 1, 0)
        );
        assert_eq!(
            read_setting::<DateTime<Utc>>(&pool, &Library::Shows.key("last_full_attempt"))
                .await
                .unwrap(),
            anchor
        );
        assert!(status(&pool, Library::Movies)
            .await
            .unwrap()
            .report
            .is_none());
    }

    #[tokio::test]
    async fn movies_skip_manual_entries_and_failure_does_not_loop_immediately() {
        let pool = database().await;
        sqlx::query("INSERT INTO settings VALUES (?, ?)")
            .bind(Library::Movies.key("frequency"))
            .bind(json!("daily").to_string())
            .execute(&pool)
            .await
            .unwrap();
        assert!(
            status(&pool, Library::Movies)
                .await
                .unwrap()
                .next_sync_at
                .unwrap()
                <= Utc::now()
        );
        let mut visited = vec![];
        let report = sync_library(&pool, Library::Movies, "automatic", |id| {
            visited.push(id);
            async { Err("offline".into()) }
        })
        .await
        .unwrap();
        assert_eq!(visited, vec![4]);
        assert_eq!((report.skipped, report.failures.len()), (1, 1));
        assert!(
            status(&pool, Library::Movies)
                .await
                .unwrap()
                .next_sync_at
                .unwrap()
                > Utc::now()
        );
        assert_eq!(
            status(&pool, Library::Shows).await.unwrap().frequency,
            Frequency::Off
        );
    }

    #[tokio::test]
    async fn interrupted_and_overdue_runs_catch_up_after_reopening() {
        let pool = database().await;
        let mut report = sync_library(&pool, Library::Shows, "manual", |_| async { Ok(()) })
            .await
            .unwrap();
        sqlx::query("INSERT INTO settings VALUES (?, ?)")
            .bind(Library::Shows.key("frequency"))
            .bind(json!("monthly").to_string())
            .execute(&pool)
            .await
            .unwrap();
        report.finished_at = None;
        save_report(&pool, &report).await.unwrap();
        let interrupted = status(&pool, Library::Shows).await.unwrap();
        assert!(interrupted.interrupted);
        assert!(interrupted.next_sync_at.unwrap() <= Utc::now());
        report.finished_at = Some(Utc::now());
        save_report(&pool, &report).await.unwrap();
        sqlx::query("UPDATE settings SET value = ? WHERE key = ?")
            .bind(json!(Utc::now() - Duration::days(60)).to_string())
            .bind(Library::Shows.key("last_full_attempt"))
            .execute(&pool)
            .await
            .unwrap();
        let overdue = status(&pool, Library::Shows).await.unwrap();
        assert!(!overdue.interrupted);
        assert!(overdue.next_sync_at.unwrap() < Utc::now());
    }

    #[tokio::test]
    async fn fatal_library_error_is_saved_instead_of_reporting_success() {
        let pool = database().await;
        sqlx::query("DROP TABLE movies")
            .execute(&pool)
            .await
            .unwrap();
        let report = sync_library(&pool, Library::Movies, "manual", |_| async {
            panic!("no titles can be loaded")
        })
        .await
        .unwrap();
        assert!(report.error.unwrap().contains("Cannot read library"));
        assert!(status(&pool, Library::Movies)
            .await
            .unwrap()
            .report
            .unwrap()
            .error
            .is_some());
    }
}
