use super::*;
use serde_json::json;

struct TestFolder(PathBuf);
impl TestFolder {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "tvc-backup-tests-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for TestFolder {
    fn drop(&mut self) {
        assert_eq!(self.0.parent(), Some(std::env::temp_dir().as_path()));
        assert!(self
            .0
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("tvc-backup-tests-"));
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn at(value: &str) -> DateTime<Utc> {
    value.parse().unwrap()
}
fn data(at: DateTime<Utc>) -> serde_json::Value {
    json!({ "version":"3.0", "exported_at":at.to_rfc3339(), "shows":[],"episodes":[],"movies":[],"tiers":[],
        "personal_data":{"ceremonies":[],"categories":[],"nominees":[],"predictions":[],"title_mappings":[],"scrobble_history":[],"change_history":[]} })
}
fn managed(folder: &Path, profile: &str, at: DateTime<Utc>) -> PathBuf {
    let mut value = data(at);
    value["automatic_backup_profile"] = json!(profile);
    let path = folder.join(filename(profile, at, "3.0"));
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    path
}

#[test]
fn defaults_are_opt_in_and_intervals_handle_month_end() {
    let prefs = BackupPreferences::default();
    assert_eq!(prefs.frequency, Frequency::Off);
    assert_eq!(prefs.retention, 5);
    assert!(!prefs.include_settings);
    let start = at("2028-01-31T12:30:00Z");
    assert_eq!(next_due(Frequency::Off, Some(start), start), None);
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
}

#[test]
fn file_creation_validates_format_and_preserves_existing_files() {
    let folder = TestFolder::new();
    let path = folder.0.join("backup.json");
    let value = data(at("2026-10-13T10:00:00Z"));
    write_new_json(&path, &value).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&fs::read(&path).unwrap()).unwrap(),
        value
    );
    assert!(write_new_json(&path, &json!({})).is_err());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&fs::read(&path).unwrap()).unwrap(),
        value
    );
    assert!(write_new_json(&folder.0.join("bad.exe"), &value).is_err());
    assert!(write_new_json(&folder.0.join("invalid.json"), &json!({"version":"99.0"})).is_err());
    assert!(!folder.0.join("invalid.json").exists());
    assert_eq!(
        fs::read_dir(&folder.0).unwrap().count(),
        1,
        "no temporary files remain"
    );
    assert!(validate_folder("relative", false).is_err());
    assert!(validate_folder(path.to_str().unwrap(), false).is_err());
}

#[test]
fn retention_removes_only_exact_completed_files_for_its_profile() {
    let folder = TestFolder::new();
    let old = managed(&folder.0, "com.tvc.app.dev", at("2026-10-10T10:00:00Z"));
    let middle = managed(&folder.0, "com.tvc.app.dev", at("2026-10-11T10:00:00Z"));
    let newest = managed(&folder.0, "com.tvc.app.dev", at("2026-10-12T10:00:00Z"));
    let release = managed(&folder.0, "com.tvc.app", at("2026-10-09T10:00:00Z"));
    let unrelated = folder.0.join(filename(
        "com.tvc.app.dev",
        at("2026-10-08T10:00:00Z"),
        "3.0",
    ));
    fs::write(
        &unrelated,
        serde_json::to_vec(&data(at("2026-10-08T10:00:00Z"))).unwrap(),
    )
    .unwrap();
    let mismatch = folder.0.join(filename(
        "com.tvc.app.dev",
        at("2026-10-07T10:00:00Z"),
        "3.0",
    ));
    let mut value = data(at("2026-10-06T10:00:00Z"));
    value["automatic_backup_profile"] = json!("com.tvc.app.dev");
    fs::write(&mismatch, serde_json::to_vec(&value).unwrap()).unwrap();
    let directory = folder.0.join(filename(
        "com.tvc.app.dev",
        at("2026-10-05T10:00:00Z"),
        "3.0",
    ));
    fs::create_dir(&directory).unwrap();
    let manual = folder.0.join("my-backup.json");
    fs::write(&manual, b"personal file").unwrap();
    let link = folder.0.join(filename(
        "com.tvc.app.dev",
        at("2026-10-04T10:00:00Z"),
        "3.0",
    ));
    #[cfg(unix)]
    let linked = std::os::unix::fs::symlink(&newest, &link).is_ok();
    #[cfg(windows)]
    let linked = std::os::windows::fs::symlink_file(&newest, &link).is_ok();
    prune(&folder.0, "com.tvc.app.dev", 2, None).unwrap();
    assert!(!old.exists());
    assert!(middle.exists());
    assert!(newest.exists());
    for path in [release, unrelated, mismatch, directory, manual] {
        assert!(path.exists(), "{}", path.display());
    }
    if linked {
        assert!(fs::symlink_metadata(link).unwrap().file_type().is_symlink());
    }
    let just_written = managed(&folder.0, "com.tvc.app.dev", at("2020-01-01T10:00:00Z"));
    prune(&folder.0, "com.tvc.app.dev", 1, Some(&just_written)).unwrap();
    assert!(
        just_written.exists(),
        "the new successful backup survives a backwards clock change"
    );
}

#[tokio::test]
async fn failed_backup_retains_success_and_delays_automatic_retry() {
    let _guard = LOCK.lock().await;
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::raw_sql("CREATE TABLE settings(key TEXT PRIMARY KEY,value TEXT NOT NULL)")
        .execute(&pool)
        .await
        .unwrap();
    let success = at("2026-10-01T10:00:00Z");
    let history = History {
        last_success_at: Some(success),
        last_success_path: Some("previous.json".into()),
        ..History::default()
    };
    put(&pool, HISTORY, &history).await.unwrap();
    let prefs = BackupPreferences {
        frequency: Frequency::Daily,
        ..BackupPreferences::default()
    };
    put(&pool, PREFERENCES, &prefs).await.unwrap();
    // The disposable database deliberately has no library schema: export fails.
    assert!(write_backup(&pool, None, false, &prefs).await.is_err());
    let saved = status(&pool).await.unwrap();
    assert_eq!(saved.last_success_at, Some(success));
    assert_eq!(saved.last_success_path.as_deref(), Some("previous.json"));
    assert!(saved.last_error.is_some());
    assert!(saved.next_due_at.unwrap() > Utc::now());
    assert!(!saved.running);
}

#[tokio::test]
async fn managed_backup_records_success_and_keeps_it_when_a_later_write_fails() {
    let _guard = LOCK.lock().await;
    let folder = TestFolder::new();
    let pool = backup::test_database().await;
    let prefs = BackupPreferences {
        folder: Some(folder.0.to_string_lossy().into()),
        retention: 1,
        include_settings: true,
        frequency: Frequency::Weekly,
    };
    put(&pool, PREFERENCES, &prefs).await.unwrap();
    let saved = write_backup(&pool, None, true, &prefs).await.unwrap();
    assert_eq!(saved.version, "4.0");
    assert!(saved.warning.is_none());
    let value: serde_json::Value = serde_json::from_slice(&fs::read(&saved.path).unwrap()).unwrap();
    assert_eq!(value["automatic_backup_profile"], profile());
    assert!(value["portable_settings"].is_object());
    let successful = status(&pool).await.unwrap();
    assert_eq!(successful.last_success_at, Some(saved.created_at));
    assert_eq!(
        successful.last_success_path.as_deref(),
        Some(saved.path.as_str())
    );
    let invalid = folder
        .0
        .join("missing-folder")
        .join("backup.json")
        .to_string_lossy()
        .into_owned();
    assert!(write_backup(&pool, Some(invalid), false, &prefs)
        .await
        .is_err());
    let failed = status(&pool).await.unwrap();
    assert_eq!(failed.last_success_at, successful.last_success_at);
    assert_eq!(failed.last_success_path, successful.last_success_path);
    assert!(Path::new(&saved.path).is_file());
    assert_eq!(fs::read_dir(&folder.0).unwrap().count(), 1);
}

#[test]
fn schedules_require_a_writable_folder_and_bounded_retention() {
    let folder = TestFolder::new();
    assert!(validate_preferences(
        BackupPreferences {
            frequency: Frequency::Daily,
            ..BackupPreferences::default()
        },
        true
    )
    .is_err());
    assert!(validate_preferences(
        BackupPreferences {
            retention: 0,
            ..BackupPreferences::default()
        },
        true
    )
    .is_err());
    let preferences = validate_preferences(
        BackupPreferences {
            frequency: Frequency::Weekly,
            folder: Some(folder.0.to_string_lossy().into()),
            ..BackupPreferences::default()
        },
        true,
    )
    .unwrap();
    assert!(preferences.folder.is_some());
    assert_eq!(fs::read_dir(&folder.0).unwrap().count(), 0);
    // Existing offline paths do not prevent turning an automatic schedule off.
    assert!(validate_preferences(
        BackupPreferences {
            folder: Some(folder.0.join("offline").to_string_lossy().into()),
            ..BackupPreferences::default()
        },
        false
    )
    .is_ok());
}
