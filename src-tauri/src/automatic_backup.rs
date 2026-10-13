//! Opt-in JSON backups, using the active app pool and an isolated filename namespace.
use crate::{commands::backup, db::connection};
use chrono::{DateTime, Duration, Months, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};
use tauri::AppHandle;
use tokio::sync::Mutex;

const PREFERENCES: &str = "local_backup_preferences";
const HISTORY: &str = "local_backup_history";
static LOCK: Mutex<()> = Mutex::const_new(());
static RUNNING: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Frequency {
    #[default]
    Off,
    Daily,
    Weekly,
    Monthly,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct BackupPreferences {
    pub frequency: Frequency,
    pub folder: Option<String>,
    pub retention: u32,
    pub include_settings: bool,
}
impl Default for BackupPreferences {
    fn default() -> Self {
        Self {
            frequency: Frequency::Off,
            folder: None,
            retention: 5,
            include_settings: false,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
struct History {
    last_success_at: Option<DateTime<Utc>>,
    last_success_path: Option<String>,
    last_attempt_at: Option<DateTime<Utc>>,
    last_error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BackupStatus {
    pub preferences: BackupPreferences,
    pub running: bool,
    pub last_success_at: Option<DateTime<Utc>>,
    pub last_success_path: Option<String>,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub next_due_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct BackupWriteResult {
    pub path: String,
    pub created_at: DateTime<Utc>,
    pub version: String,
    pub warning: Option<String>,
}

fn profile() -> &'static str {
    crate::db::app_identifier()
}
fn prefix(profile: &str) -> &'static str {
    if profile == "com.tvc.app.dev" {
        "tvc-dev-auto-"
    } else {
        "tvc-auto-"
    }
}

fn filename(profile: &str, at: DateTime<Utc>, version: &str) -> String {
    format!(
        "{}{}{:09}Z-v{}.json",
        prefix(profile),
        at.format("%Y%m%dT%H%M%S"),
        at.timestamp_subsec_nanos(),
        version.trim_end_matches(".0")
    )
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

async fn read<T: serde::de::DeserializeOwned + Default>(
    pool: &SqlitePool,
    key: &str,
) -> Result<T, String> {
    let value: Option<String> = sqlx::query_scalar("SELECT value FROM settings WHERE key=?")
        .bind(key)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Could not read backup settings: {e}"))?;
    match value {
        Some(value) => serde_json::from_str(&value)
            .map_err(|e| format!("Could not read saved backup settings: {e}")),
        None => Ok(T::default()),
    }
}
async fn put<T: Serialize>(pool: &SqlitePool, key: &str, value: &T) -> Result<(), String> {
    let value = serde_json::to_string(value).map_err(|e| e.to_string())?;
    sqlx::query("INSERT INTO settings(key,value) VALUES (?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value").bind(key).bind(value).execute(pool).await.map_err(|e| format!("Could not save backup settings: {e}"))?;
    Ok(())
}

async fn status(pool: &SqlitePool) -> Result<BackupStatus, String> {
    let preferences: BackupPreferences = read(pool, PREFERENCES).await?;
    let history: History = read(pool, HISTORY).await?;
    let next_due_at = next_due(preferences.frequency, history.last_attempt_at, Utc::now());
    Ok(BackupStatus {
        preferences,
        running: RUNNING.load(Ordering::SeqCst),
        last_success_at: history.last_success_at,
        last_success_path: history.last_success_path,
        last_attempt_at: history.last_attempt_at,
        last_error: history.last_error,
        next_due_at,
    })
}

#[tauri::command]
pub async fn get_backup_status(app: AppHandle) -> Result<BackupStatus, String> {
    status(
        &connection::get_pool(&app)
            .await
            .map_err(|e| e.to_string())?,
    )
    .await
}

fn validate_folder(folder: &str, writable: bool) -> Result<PathBuf, String> {
    let path = Path::new(folder);
    if !path.is_absolute() {
        return Err("Choose an absolute backup folder path".into());
    }
    let metadata =
        fs::symlink_metadata(path).map_err(|e| format!("Could not access backup folder: {e}"))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("Choose an existing folder, rather than a file or symbolic link".into());
    }
    let path =
        fs::canonicalize(path).map_err(|e| format!("Could not resolve backup folder: {e}"))?;
    if writable {
        let probe = path.join(format!(
            ".{}write-check-{}",
            prefix(profile()),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&probe)
            .map_err(|e| format!("The backup folder is not writable: {e}"))?;
        drop(file);
        fs::remove_file(&probe)
            .map_err(|e| format!("Could not finish checking the backup folder: {e}"))?;
    }
    Ok(path)
}

fn validate_preferences(
    mut preferences: BackupPreferences,
    check_folder: bool,
) -> Result<BackupPreferences, String> {
    if !(1..=50).contains(&preferences.retention) {
        return Err("Keep between 1 and 50 automatic backups".into());
    }
    if let Some(folder) = preferences.folder.as_ref().filter(|_| check_folder) {
        preferences.folder = Some(
            validate_folder(folder, true)?
                .to_string_lossy()
                .into_owned(),
        );
    }
    if preferences.frequency != Frequency::Off && preferences.folder.is_none() {
        return Err("Choose a backup folder before enabling automatic backups".into());
    }
    Ok(preferences)
}

#[tauri::command]
pub async fn update_backup_preferences(
    app: AppHandle,
    preferences: BackupPreferences,
) -> Result<BackupStatus, String> {
    let _guard = LOCK
        .try_lock()
        .map_err(|_| "Wait for the current backup to finish")?;
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| e.to_string())?;
    let previous: BackupPreferences = read(&pool, PREFERENCES).await?;
    // Disabling the schedule must work even when an external backup drive is offline.
    let check_folder =
        preferences.frequency != Frequency::Off || preferences.folder != previous.folder;
    let preferences =
        tokio::task::spawn_blocking(move || validate_preferences(preferences, check_folder))
            .await
            .map_err(|e| e.to_string())??;
    put(&pool, PREFERENCES, &preferences).await?;
    status(&pool).await
}

// create_new prevents overwriting another file, including a symbolic link.
// A complete temporary file is linked into place where supported. On filesystems
// without hard links, the exclusive-copy fallback removes any incomplete new file.
fn write_new_json(path: &Path, value: &serde_json::Value) -> Result<(), String> {
    if path
        .extension()
        .and_then(|e| e.to_str())
        .is_none_or(|e| !e.eq_ignore_ascii_case("json"))
    {
        return Err("Backup files must use the .json extension".into());
    }
    let parent = path.parent().ok_or("Choose a backup folder")?;
    let folder = validate_folder(&parent.to_string_lossy(), false)?;
    let name = path.file_name().ok_or("Choose a backup filename")?;
    let path = folder.join(name);
    if fs::symlink_metadata(&path).is_ok() {
        return Err(
            "This file already exists. Choose a new backup filename so it can be kept.".into(),
        );
    }
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|e| format!("Could not prepare JSON backup: {e}"))?;
    // Validate the exact serialized format before any final file exists.
    let decoded: backup::BackupData =
        serde_json::from_slice(&bytes).map_err(|e| format!("Invalid backup format: {e}"))?;
    decoded.validate()?;
    let temporary = folder.join(format!(
        ".{}{}-{}.part",
        prefix(profile()),
        std::process::id(),
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|e| format!("Could not create backup: {e}"))?;
    let result = (|| -> std::io::Result<()> {
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        if fs::hard_link(&temporary, &path).is_err() {
            let mut target = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)?;
            let copy = target.write_all(&bytes).and_then(|_| target.sync_all());
            drop(target);
            if let Err(error) = copy {
                let _ = fs::remove_file(&path);
                return Err(error);
            }
        }
        Ok(())
    })();
    let cleanup = fs::remove_file(&temporary);
    result.map_err(|e| format!("Could not finish backup: {e}"))?;
    if let Err(error) = cleanup {
        eprintln!("[backup] Could not remove temporary file: {error}");
    }
    Ok(())
}

fn prune(
    folder: &Path,
    profile: &str,
    retention: usize,
    keep: Option<&Path>,
) -> Result<(), String> {
    let mut managed = Vec::new();
    for entry in fs::read_dir(folder)
        .map_err(|e| format!("Backup created, but older backups could not be listed: {e}"))?
    {
        let entry = entry.map_err(|e| format!("Could not inspect older backup: {e}"))?;
        let metadata = fs::symlink_metadata(entry.path()).map_err(|e| e.to_string())?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with(prefix(profile)) || !name.ends_with(".json") {
            continue;
        }
        // The filename and marker both have to identify a completed managed file.
        let Ok(bytes) = fs::read(entry.path()) else {
            continue;
        };
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
            continue;
        };
        if value["automatic_backup_profile"].as_str() != Some(profile) {
            continue;
        }
        let Some(version) = value["version"]
            .as_str()
            .filter(|v| matches!(*v, "3.0" | "4.0"))
        else {
            continue;
        };
        let Some(at) = value["exported_at"]
            .as_str()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.with_timezone(&Utc))
        else {
            continue;
        };
        if name != filename(profile, at, version) {
            continue;
        }
        let Ok(data) = serde_json::from_value::<backup::BackupData>(value) else {
            continue;
        };
        if data.validate().is_err() {
            continue;
        }
        managed.push((at, entry.path()));
    }
    // Keep the file just written even if this computer's clock moved backwards.
    managed.sort_by(|a, b| {
        (Some(b.1.as_path()) == keep)
            .cmp(&(Some(a.1.as_path()) == keep))
            .then_with(|| b.0.cmp(&a.0))
    });
    for (_, path) in managed.into_iter().skip(retention) {
        fs::remove_file(path).map_err(|e| {
            format!("Backup created, but an older managed backup could not be removed: {e}")
        })?;
    }
    Ok(())
}

struct Running;
impl Drop for Running {
    fn drop(&mut self) {
        RUNNING.store(false, Ordering::SeqCst);
    }
}

async fn write_backup(
    pool: &SqlitePool,
    manual_path: Option<String>,
    include_settings: bool,
    preferences: &BackupPreferences,
) -> Result<BackupWriteResult, String> {
    RUNNING.store(true, Ordering::SeqCst);
    let _running = Running;
    let mut history: History = read(pool, HISTORY).await?;
    history.last_attempt_at = Some(Utc::now());
    history.last_error = None;
    put(pool, HISTORY, &history).await?;
    let outcome: Result<BackupWriteResult, String> = async {
        let data = backup::export_with_settings(pool, include_settings).await?;
        let at = DateTime::parse_from_rfc3339(&data.exported_at)
            .map_err(|e| e.to_string())?
            .with_timezone(&Utc);
        let version = data.version.clone();
        let mut value = serde_json::to_value(&data).map_err(|e| e.to_string())?;
        let (path, managed) = match manual_path {
            Some(path) => (PathBuf::from(path), false),
            None => {
                let folder = preferences
                    .folder
                    .as_ref()
                    .ok_or("Choose a backup folder first")?;
                let folder = validate_folder(folder, false)?;
                value["automatic_backup_profile"] = serde_json::Value::String(profile().into());
                (folder.join(filename(profile(), at, &version)), true)
            }
        };
        let saved_path = path.to_string_lossy().into_owned();
        let profile = profile();
        let retention = preferences.retention as usize;
        let warning = tokio::task::spawn_blocking(move || {
            write_new_json(&path, &value)?;
            Ok::<_, String>(if managed {
                prune(path.parent().unwrap(), profile, retention, Some(&path)).err()
            } else {
                None
            })
        })
        .await
        .map_err(|e| e.to_string())??;
        Ok(BackupWriteResult {
            path: saved_path,
            created_at: at,
            version,
            warning,
        })
    }
    .await;
    match outcome {
        Ok(mut result) => {
            history.last_success_at = Some(result.created_at);
            history.last_success_path = Some(result.path.clone());
            history.last_error = result.warning.clone();
            if let Err(error) = put(pool, HISTORY, &history).await {
                let warning = format!("The backup file was created, but its saved status could not be updated: {error}");
                result.warning = Some(match result.warning {
                    Some(previous) => format!("{previous} {warning}"),
                    None => warning,
                });
            }
            Ok(result)
        }
        Err(error) => {
            history.last_error = Some(error.clone());
            if let Err(save_error) = put(pool, HISTORY, &history).await {
                return Err(format!(
                    "{error}. Could not save failure status: {save_error}"
                ));
            }
            Err(error)
        }
    }
}

#[tauri::command]
pub async fn create_backup_file(
    app: AppHandle,
    path: String,
    include_settings: bool,
) -> Result<BackupWriteResult, String> {
    let _guard = LOCK
        .try_lock()
        .map_err(|_| "Wait for the current backup to finish")?;
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| e.to_string())?;
    write_backup(
        &pool,
        Some(path),
        include_settings,
        &BackupPreferences::default(),
    )
    .await
}

#[tauri::command]
pub async fn create_local_backup(app: AppHandle) -> Result<BackupWriteResult, String> {
    let _guard = LOCK
        .try_lock()
        .map_err(|_| "Wait for the current backup to finish")?;
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| e.to_string())?;
    let preferences: BackupPreferences = read(&pool, PREFERENCES).await?;
    write_backup(&pool, None, preferences.include_settings, &preferences).await
}

async fn due(app: &AppHandle) -> Result<(), String> {
    let Ok(_guard) = LOCK.try_lock() else {
        return Ok(());
    };
    let pool = connection::get_pool(app).await.map_err(|e| e.to_string())?;
    let status = status(&pool).await?;
    if status.next_due_at.is_some_and(|at| at <= Utc::now()) {
        write_backup(
            &pool,
            None,
            status.preferences.include_settings,
            &status.preferences,
        )
        .await?;
    }
    Ok(())
}

pub async fn start_scheduler(app: AppHandle) {
    loop {
        if let Err(error) = due(&app).await {
            eprintln!("[backup] {error}");
        }
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}

#[cfg(test)]
mod tests;
