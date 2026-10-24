use super::*;
use serde_json::{json, Value};

pub(super) async fn database() -> sqlx::SqlitePool {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    for migration in [
        crate::MIGRATION_001,
        crate::MIGRATION_002,
        crate::MIGRATION_003,
        crate::MIGRATION_004,
        crate::MIGRATION_005,
        crate::MIGRATION_006,
        crate::MIGRATION_007,
        crate::MIGRATION_008,
        crate::MIGRATION_009,
        crate::MIGRATION_010,
        crate::MIGRATION_011,
        crate::MIGRATION_012,
        crate::MIGRATION_013,
        crate::MIGRATION_014,
        crate::MIGRATION_015,
        crate::MIGRATION_016,
        crate::MIGRATION_017,
        crate::MIGRATION_018,
        crate::MIGRATION_019,
    ] {
        sqlx::raw_sql(migration).execute(&pool).await.unwrap();
    }
    pool
}

async fn seed(pool: &sqlx::SqlitePool) {
    sqlx::raw_sql(r#"
        INSERT INTO shows (id, name, notes, tags, rating, tier_id, rank_order, legacy_tvdb_id)
        VALUES (101, 'Ørnen', 'My notes', '["favorite"]', 4.5, 1, 2, 900);
        INSERT INTO shows (id, name, tier_only, archived, tier_id, rank_order)
        VALUES (-7, 'Manual show', 1, 1, 2, 1);
        INSERT INTO episodes (id, show_id, season_number, episode_number, name, watched, watched_at, scheduled_date, rating, tags, legacy_tvdb_id)
        VALUES (201, 101, 1, 1, 'Début', 1, '2026-07-09 18:30:00', '2026-07-10', 8.5, '["rewatch"]', 901),
               (202, 101, 1, 2, 'Next', 0, NULL, NULL, NULL, NULL, NULL);
        INSERT INTO movies (id, title, watched, watched_at, scheduled_date, rating, notes, tags, tier_id, rank_order, archived)
        VALUES (301, 'Movie', 1, '2026-08-01T20:15:00Z', '2026-08-01', 4.5, 'Keep', '["favorite"]', 2, 4, 1);
        INSERT INTO award_ceremonies (id, award_type, edition, name, year, ceremony_date, nominations_date, status, wiki_title)
        VALUES (1, 'emmys', 78, '78th Primetime Emmy Awards', 2026, '2026-09-14', '2026-07-09', 'past', '78th Primetime Emmy Awards');
        INSERT INTO award_categories (id, ceremony_id, name, display_order)
        VALUES (2, 1, 'Comedy', 1), (3, 1, 'Drama', 2);
        INSERT INTO award_nominees (id, category_id, title, detail, is_winner, source_key)
        VALUES (4, 2, 'Saved original nominee', 'Original detail', 1, 'saved-key'),
               (5, 3, 'Unreported nominee', NULL, NULL, 'pending-key');
        INSERT INTO award_predictions (id, category_id, nominee_id, created_at, updated_at)
        VALUES (6, 2, 4, '2026-07-09 13:00:00', '2026-07-09 13:05:00'),
               (7, 3, 5, '2026-07-10 13:00:00', '2026-07-10 13:00:00');
        INSERT INTO title_mappings (id, plex_title, media_type, tvc_id, tvc_title)
        VALUES (1, 'My Plex name', 'show', 101, 'Ørnen');
        INSERT INTO plex_scrobble_log (id, event_type, media_type, raw_title, show_name, season_number, episode_number, matched_entity_type, matched_entity_id, match_method, scrobbled_at)
        VALUES (1, 'media.scrobble', 'episode', 'Début', 'Ørnen', 1, 1, 'episode', 201, 'mapping', '2026-07-09 18:30:00');
        INSERT INTO change_history (id, entity_type, entity_id, change_type, old_value, new_value, changed_at, user_action)
        VALUES (1, 'episode', 201, 'watched', '0', '1', '2026-07-09 18:30:00', 'manual'),
               (2, 'movie', 999, 'watched', '0', '1', '2026-01-01 18:00:00', 'manual');
    "#).execute(pool).await.unwrap();
}

async fn snapshot(pool: &sqlx::SqlitePool) -> Value {
    let mut value = serde_json::to_value(export_from_pool(pool).await.unwrap()).unwrap();
    value.as_object_mut().unwrap().remove("exported_at");
    value
}

#[tokio::test]
async fn full_backup_round_trip_preserves_personal_state_and_source_snapshots() {
    let source = database().await;
    seed(&source).await;
    sqlx::query("INSERT INTO settings (key, value) VALUES ('private-preference', 'source-value')")
        .execute(&source)
        .await
        .unwrap();
    let before = snapshot(&source).await;
    assert!(!before.to_string().contains("source-value"));
    let backup = export_from_pool(&source).await.unwrap();
    assert_eq!(backup.version, "3.0");
    let encoded = serde_json::to_string(&backup).unwrap();
    let decoded: BackupData = serde_json::from_str(&encoded).unwrap();
    let target = database().await;
    sqlx::raw_sql(
        "INSERT INTO settings (key, value) VALUES ('private-preference', 'target-value'),
        ('library_sync_shows_frequency', '\"weekly\"'), ('library_sync_shows_report', '{}');",
    )
    .execute(&target)
    .await
    .unwrap();
    for kind in ["shows", "movies"] {
        for suffix in ["last_full_report", "history", "last_stopped_at"] {
            sqlx::query(
                "INSERT OR REPLACE INTO settings(key,value) VALUES (?, 'old-library-state')",
            )
            .bind(format!("library_sync_{kind}_{suffix}"))
            .execute(&target)
            .await
            .unwrap();
        }
    }
    let restored = replace_from_pool(&target, &decoded, &HashMap::new())
        .await
        .unwrap();
    assert_eq!(restored.result.shows_imported, 2);
    assert_eq!(restored.result.episodes_imported, 2);
    assert_eq!(restored.result.movies_imported, 1);
    for kind in ["shows", "movies"] {
        for suffix in ["last_full_report", "history", "last_stopped_at"] {
            assert!(
                sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key=?")
                    .bind(format!("library_sync_{kind}_{suffix}"))
                    .fetch_optional(&target)
                    .await
                    .unwrap()
                    .is_none()
            );
        }
    }
    assert_eq!(restored.result.predictions_imported, 2);
    assert_eq!(snapshot(&target).await, before);
    let settings: Vec<(String, String)> = sqlx::query_as("SELECT key, value FROM settings")
        .fetch_all(&target)
        .await
        .unwrap();
    assert!(settings.contains(&("private-preference".into(), "target-value".into())));
    assert!(settings.contains(&("library_sync_shows_frequency".into(), "\"weekly\"".into())));
    assert!(!settings
        .iter()
        .any(|(key, _)| key == "library_sync_shows_report"));
}

#[tokio::test]
async fn malformed_backups_never_replace_existing_data() {
    let pool = database().await;
    seed(&pool).await;
    let before = snapshot(&pool).await;
    let original = serde_json::to_value(export_from_pool(&pool).await.unwrap()).unwrap();
    let mut cases = vec![];
    let mut bad = original.clone();
    bad["version"] = json!("99.0");
    cases.push(bad);
    let mut bad = original.clone();
    bad["personal_data"] = Value::Null;
    cases.push(bad);
    let mut bad = original.clone();
    bad["version"] = json!("2.0");
    cases.push(bad);
    for field in ["shows", "episodes", "movies", "tiers"] {
        let mut bad = original.clone();
        let duplicate = bad[field][0].clone();
        bad[field].as_array_mut().unwrap().push(duplicate);
        cases.push(bad);
    }
    let mut bad = original.clone();
    bad["episodes"][0]["show_id"] = json!(999);
    cases.push(bad);
    let mut bad = original.clone();
    bad["shows"][0]["tier_id"] = json!(999);
    cases.push(bad);
    let mut bad = original.clone();
    bad["personal_data"]["predictions"][0]["nominee_id"] = json!(5);
    cases.push(bad);
    let mut bad = original.clone();
    bad["personal_data"]["categories"][0]["ceremony_id"] = json!(999);
    cases.push(bad);
    let mut bad = original.clone();
    bad["personal_data"]["nominees"][0]["is_winner"] = json!(9);
    cases.push(bad);
    for value in cases {
        let backup: BackupData = serde_json::from_value(value).unwrap();
        assert!(replace_from_pool(&pool, &backup, &HashMap::new())
            .await
            .is_err());
        assert_eq!(snapshot(&pool).await, before);
    }
    let mut missing = original;
    missing["personal_data"]
        .as_object_mut()
        .unwrap()
        .remove("predictions");
    assert!(serde_json::from_value::<BackupData>(missing).is_err());
}

#[tokio::test]
async fn sql_failures_after_deletion_roll_back_library_predictions_and_history() {
    for table in [
        "shows",
        "episodes",
        "movies",
        "award_predictions",
        "title_mappings",
        "settings",
    ] {
        let pool = database().await;
        seed(&pool).await;
        let before = snapshot(&pool).await;
        let mut backup = export_from_pool(&pool).await.unwrap();
        backup.shows[0].name = "Changed name".into();
        // This fails at different stages of the real restore, after DELETEs have
        // run. The final settings write even follows all prediction/history rows.
        sqlx::raw_sql(&format!("CREATE TRIGGER fail_restore BEFORE INSERT ON {table} BEGIN SELECT RAISE(ABORT, 'injected failure'); END;"))
            .execute(&pool).await.unwrap();
        assert!(
            replace_from_pool(&pool, &backup, &HashMap::new())
                .await
                .is_err(),
            "{table}"
        );
        assert_eq!(snapshot(&pool).await, before, "rollback failed at {table}");
    }
}

#[tokio::test]
async fn deferred_foreign_key_errors_roll_back_instead_of_committing() {
    let pool = database().await;
    seed(&pool).await;
    let before = snapshot(&pool).await;
    let backup = export_from_pool(&pool).await.unwrap();
    sqlx::raw_sql(
        "CREATE TRIGGER invalid_reference AFTER INSERT ON movies BEGIN
        INSERT INTO episodes (id, show_id, name) VALUES (999, 999, 'Missing parent'); END;",
    )
    .execute(&pool)
    .await
    .unwrap();
    let error = replace_from_pool(&pool, &backup, &HashMap::new())
        .await
        .err()
        .unwrap();
    assert!(error.contains("invalid references"), "{error}");
    assert_eq!(snapshot(&pool).await, before);
}

#[tokio::test]
async fn older_backups_keep_awards_and_report_legacy_id_collisions() {
    let pool = database().await;
    seed(&pool).await;
    let mut backup = export_from_pool(&pool).await.unwrap();
    backup.version = "2.0".into();
    backup.personal_data = None;
    let before_awards = snapshot(&pool).await["personal_data"]["predictions"].clone();
    let result = replace_from_pool(&pool, &backup, &HashMap::new())
        .await
        .unwrap();
    assert_eq!(result.result.shows_imported, 2);
    let after = snapshot(&pool).await;
    assert_eq!(after["personal_data"]["predictions"], before_awards);
    assert_eq!(after["personal_data"]["title_mappings"], json!([]));

    backup.version = "1.0".into();
    backup.shows.reverse();
    // Map the positive show onto the manual show's occupied ID: this must fail
    // without skipping either row or committing the already-cleared library.
    let remap = HashMap::from([(101, Some(-7))]);
    assert!(replace_from_pool(&pool, &backup, &remap).await.is_err());
    assert_eq!(snapshot(&pool).await, after);
    let result = replace_from_pool(&pool, &backup, &HashMap::new())
        .await
        .unwrap();
    assert_eq!(result.result.quarantined, 1);
    assert_eq!(result.result.episodes_imported, 2);

    let mut legacy = serde_json::to_value(backup).unwrap();
    legacy.as_object_mut().unwrap().remove("version");
    assert_eq!(
        serde_json::from_value::<BackupData>(legacy)
            .unwrap()
            .version,
        "1.0"
    );
}

#[test]
fn restore_and_bulk_sync_share_the_same_exclusion_lock() {
    let guard = crate::library_sync::lock_for_restore().unwrap();
    assert!(crate::library_sync::lock_for_restore().is_err());
    assert_eq!(
        crate::library_sync::lock_for_maintenance().unwrap_err(),
        "Wait for the current sync or restore before cleanup"
    );
    drop(guard);
    let maintenance = crate::library_sync::lock_for_maintenance().unwrap();
    assert!(crate::library_sync::lock_for_restore().is_err());
    assert!(crate::library_sync::lock_for_maintenance().is_err());
    drop(maintenance);
    assert!(crate::library_sync::lock_for_restore().is_ok());
}

#[tokio::test]
async fn portable_backup_transfers_only_allowlisted_preferences() {
    let source = database().await;
    seed(&source).await;
    let mut general = crate::commands::preferences::GeneralPreferences::default();
    general.release_country = "NO".into();
    general.calendar_view = "agenda".into();
    general.default_tier_list = "movies".into();
    sqlx::query("INSERT OR REPLACE INTO settings(key,value) VALUES ('general_preferences',?)")
        .bind(serde_json::to_string(&general).unwrap())
        .execute(&source)
        .await
        .unwrap();
    sqlx::raw_sql("INSERT OR REPLACE INTO settings(key,value) VALUES ('theme_accentColor','#abc'),('theme_fontSize','1.2'),('theme_posterSize','large'),('tier_preset','custom'),('notification_quiet_hours_enabled','true'),('notification_quiet_hours_start','23:00'),('notification_quiet_hours_end','07:00'),('tmdb_api_key','secret-not-portable'),('local_backup_preferences','private-folder'),('library_sync_movies_frequency','\"daily\"');
        UPDATE racing_series SET enabled=1,notify_minutes=60,custom_color='#123456',custom_ics_url='https://private.invalid/secret-feed' WHERE slug='f1';
        UPDATE notification_settings SET sound_volume=25;"
    ).execute(&source).await.unwrap();
    let backup = export_with_settings(&source, true).await.unwrap();
    assert_eq!(backup.version, "4.0");
    backup.validate().unwrap();
    assert_eq!(
        backup.portable_settings.as_ref().unwrap().general["default_tier_list"],
        "movies"
    );
    let encoded = serde_json::to_string(&backup).unwrap();
    for excluded in [
        "secret-not-portable",
        "private-folder",
        "secret-feed",
        "library_sync_movies_frequency",
    ] {
        assert!(!encoded.contains(excluded));
    }
    let decoded: BackupData = serde_json::from_str(&encoded).unwrap();
    let target = database().await;
    sqlx::raw_sql("INSERT OR REPLACE INTO settings(key,value) VALUES ('tmdb_api_key','keep-target-secret'),('local_backup_preferences','keep-target-folder'),('library_sync_movies_frequency','\"weekly\"'); UPDATE racing_series SET custom_ics_url='https://target.invalid/feed' WHERE slug='f1';")
        .execute(&target).await.unwrap();
    let result = replace_from_pool(&target, &decoded, &HashMap::new())
        .await
        .unwrap()
        .result;
    assert!(result.settings_restored);
    assert!(result.restart_required);
    let restored = export_with_settings(&target, true).await.unwrap();
    assert_eq!(
        serde_json::to_value(&restored.portable_settings).unwrap(),
        serde_json::to_value(&backup.portable_settings).unwrap()
    );
    for (key, expected) in [
        ("tmdb_api_key", "keep-target-secret"),
        ("local_backup_preferences", "keep-target-folder"),
        ("library_sync_movies_frequency", "\"weekly\""),
    ] {
        assert_eq!(
            sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key=?")
                .bind(key)
                .fetch_one(&target)
                .await
                .unwrap(),
            expected
        );
    }
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT custom_ics_url FROM racing_series WHERE slug='f1'")
            .fetch_one(&target)
            .await
            .unwrap(),
        "https://target.invalid/feed"
    );
}

#[tokio::test]
async fn older_v4_backups_and_saved_preferences_default_the_tier_list_to_shows() {
    let source = database().await;
    let mut general =
        serde_json::to_value(crate::commands::preferences::GeneralPreferences::default()).unwrap();
    general.as_object_mut().unwrap().remove("default_tier_list");
    general["release_country"] = json!("NO");
    sqlx::query("INSERT INTO settings(key,value) VALUES ('general_preferences',?)")
        .bind(general.to_string())
        .execute(&source)
        .await
        .unwrap();

    let mut backup = export_with_settings(&source, true).await.unwrap();
    let settings = backup.portable_settings.as_mut().unwrap();
    assert_eq!(settings.general["default_tier_list"], "shows");
    settings
        .general
        .as_object_mut()
        .unwrap()
        .remove("default_tier_list");
    backup.validate().unwrap();

    let target = database().await;
    let mut preferences = crate::commands::preferences::GeneralPreferences::default();
    preferences.default_tier_list = "movies".into();
    sqlx::query("INSERT INTO settings(key,value) VALUES ('general_preferences',?)")
        .bind(serde_json::to_string(&preferences).unwrap())
        .execute(&target)
        .await
        .unwrap();
    replace_from_pool(&target, &backup, &HashMap::new())
        .await
        .unwrap();
    let restored = crate::commands::preferences::preferences_from_pool(&target)
        .await
        .unwrap();
    assert_eq!(restored.default_tier_list, "shows");
    assert_eq!(restored.release_country, "NO");
}

#[tokio::test]
async fn portable_notification_auto_clear_choices_round_trip_without_a_schema_change() {
    for days in [0, 1, 7, 30] {
        let source = database().await;
        sqlx::query(
            "INSERT INTO settings(key,value) VALUES ('notification_auto_clear_after_days',?)",
        )
        .bind(days.to_string())
        .execute(&source)
        .await
        .unwrap();
        let backup = export_with_settings(&source, true).await.unwrap();
        assert_eq!(backup.version, "4.0");
        assert_eq!(
            backup.portable_settings.as_ref().unwrap().notifications["auto_clear_after_days"],
            days
        );
        let decoded: BackupData =
            serde_json::from_str(&serde_json::to_string(&backup).unwrap()).unwrap();
        decoded.validate().unwrap();
        let target = database().await;
        replace_from_pool(&target, &decoded, &HashMap::new())
            .await
            .unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, String>(
                "SELECT value FROM settings WHERE key='notification_auto_clear_after_days'"
            )
            .fetch_one(&target)
            .await
            .unwrap(),
            days.to_string()
        );
        assert_eq!(
            export_with_settings(&target, true)
                .await
                .unwrap()
                .portable_settings
                .as_ref()
                .unwrap()
                .notifications["auto_clear_after_days"],
            days
        );
    }
}

#[tokio::test]
async fn older_v4_notifications_restore_with_automatic_clearing_off() {
    let source = database().await;
    let mut backup = export_with_settings(&source, true).await.unwrap();
    assert_eq!(
        backup.portable_settings.as_ref().unwrap().notifications["auto_clear_after_days"],
        0
    );
    backup
        .portable_settings
        .as_mut()
        .unwrap()
        .notifications
        .as_object_mut()
        .unwrap()
        .remove("auto_clear_after_days");
    let decoded: BackupData =
        serde_json::from_str(&serde_json::to_string(&backup).unwrap()).unwrap();
    decoded.validate().unwrap();

    let target = database().await;
    sqlx::query(
        "INSERT INTO settings(key,value) VALUES ('notification_auto_clear_after_days','1')",
    )
    .execute(&target)
    .await
    .unwrap();
    replace_from_pool(&target, &decoded, &HashMap::new())
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT value FROM settings WHERE key='notification_auto_clear_after_days'"
        )
        .fetch_one(&target)
        .await
        .unwrap(),
        "0"
    );
}

#[tokio::test]
async fn invalid_portable_notification_auto_clear_settings_reject_before_library_writes() {
    let pool = database().await;
    seed(&pool).await;
    sqlx::query(
        "INSERT INTO settings(key,value) VALUES ('notification_auto_clear_after_days','7')",
    )
    .execute(&pool)
    .await
    .unwrap();
    let encoded = serde_json::to_value(export_with_settings(&pool, true).await.unwrap()).unwrap();
    let before = snapshot(&pool).await;
    for invalid in [
        json!(-1),
        json!(2),
        json!(31),
        json!(1.5),
        json!("7"),
        json!(null),
        json!(true),
        json!([]),
        json!({}),
    ] {
        let mut value = encoded.clone();
        value["shows"][0]["name"] = json!("Must not be restored");
        value["portable_settings"]["notifications"]["auto_clear_after_days"] = invalid.clone();
        let backup: BackupData = serde_json::from_value(value).unwrap();
        assert!(backup.validate().is_err(), "invalid choice {invalid}");
        assert!(replace_from_pool(&pool, &backup, &HashMap::new())
            .await
            .is_err());
        assert_eq!(snapshot(&pool).await, before);
        assert_eq!(
            sqlx::query_scalar::<_, String>(
                "SELECT value FROM settings WHERE key='notification_auto_clear_after_days'"
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            "7"
        );
    }
    for old in [false, true] {
        let mut value = encoded.clone();
        if old {
            value["portable_settings"]["notifications"]
                .as_object_mut()
                .unwrap()
                .remove("auto_clear_after_days");
        }
        value["portable_settings"]["notifications"]["unsupported_notification_key"] = json!(true);
        let backup: BackupData = serde_json::from_value(value).unwrap();
        assert!(backup.validate().is_err());
        assert!(replace_from_pool(&pool, &backup, &HashMap::new())
            .await
            .is_err());
        assert_eq!(snapshot(&pool).await, before);
    }
}

#[tokio::test]
async fn portable_preferences_validate_before_writes_and_roll_back_with_library() {
    let pool = database().await;
    seed(&pool).await;
    let mut backup = export_with_settings(&pool, true).await.unwrap();
    let before_library = snapshot(&pool).await;
    let before_settings = serde_json::to_value(&backup.portable_settings).unwrap();
    backup.shows[0].name = "Must roll back".into();
    let settings = backup.portable_settings.as_mut().unwrap();
    settings.appearance["accentColor"] = json!("#123456");
    settings.general["api_key"] = json!("forbidden");
    assert!(replace_from_pool(&pool, &backup, &HashMap::new())
        .await
        .is_err());
    assert_eq!(snapshot(&pool).await, before_library);
    backup
        .portable_settings
        .as_mut()
        .unwrap()
        .general
        .as_object_mut()
        .unwrap()
        .remove("api_key");
    sqlx::raw_sql("CREATE TRIGGER fail_preferences BEFORE UPDATE ON notification_settings BEGIN SELECT RAISE(ABORT,'injected preference failure'); END;")
        .execute(&pool).await.unwrap();
    assert!(replace_from_pool(&pool, &backup, &HashMap::new())
        .await
        .is_err());
    assert_eq!(snapshot(&pool).await, before_library);
    assert_eq!(
        serde_json::to_value(
            export_with_settings(&pool, true)
                .await
                .unwrap()
                .portable_settings
        )
        .unwrap(),
        before_settings
    );
}

#[tokio::test]
async fn portable_schema_rejects_unknown_groups_invalid_values_and_old_versions() {
    let pool = database().await;
    let backup = export_with_settings(&pool, true).await.unwrap();
    let encoded = serde_json::to_value(backup).unwrap();
    let mut unknown = encoded.clone();
    unknown["portable_settings"]["credentials"] = json!({"token":"bad"});
    assert!(serde_json::from_value::<BackupData>(unknown).is_err());
    for (group, key, invalid) in [
        ("appearance", "accentColor", json!("red")),
        ("appearance", "fontSize", json!(10)),
        ("general", "close_action", json!("execute")),
        ("general", "default_tier_list", json!("all")),
        ("general", "default_tier_list", json!(null)),
        ("notifications", "quiet_hours_start", json!("10 PM")),
        ("notifications", "sound_volume", json!(999)),
        ("notifications", "sound_enabled", json!("true")),
    ] {
        let mut value = encoded.clone();
        value["portable_settings"][group][key] = invalid;
        assert!(
            serde_json::from_value::<BackupData>(value)
                .unwrap()
                .validate()
                .is_err(),
            "{group}.{key}"
        );
    }
    for version in ["1.0", "2.0", "3.0"] {
        let mut value = encoded.clone();
        value["version"] = json!(version);
        assert!(serde_json::from_value::<BackupData>(value)
            .unwrap()
            .validate()
            .is_err());
    }
    let mut missing = encoded;
    missing.as_object_mut().unwrap().remove("portable_settings");
    assert!(serde_json::from_value::<BackupData>(missing)
        .unwrap()
        .validate()
        .is_err());
}

#[tokio::test]
async fn portable_empty_racing_scope_disables_target_series_without_touching_feeds() {
    let pool = database().await;
    let mut backup = export_with_settings(&pool, true).await.unwrap();
    backup
        .portable_settings
        .as_mut()
        .unwrap()
        .racing
        .series
        .clear();
    sqlx::raw_sql("UPDATE racing_series SET enabled=1,custom_ics_url='https://keep.invalid/feed' WHERE slug='f1'").execute(&pool).await.unwrap();
    replace_from_pool(&pool, &backup, &HashMap::new())
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM racing_series WHERE enabled=1")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT custom_ics_url FROM racing_series WHERE slug='f1'")
            .fetch_one(&pool)
            .await
            .unwrap(),
        "https://keep.invalid/feed"
    );
}

#[tokio::test]
async fn missing_portable_setting_targets_never_commit_partial_library_restores() {
    for table in ["notification_settings", "racing_config"] {
        let pool = database().await;
        seed(&pool).await;
        let mut backup = export_with_settings(&pool, true).await.unwrap();
        backup.shows[0].name = "Must not commit".into();
        sqlx::query(&format!("DELETE FROM {table} WHERE id=1"))
            .execute(&pool)
            .await
            .unwrap();
        let before = snapshot(&pool).await;
        assert!(replace_from_pool(&pool, &backup, &HashMap::new())
            .await
            .is_err());
        assert_eq!(snapshot(&pool).await, before);
    }
}
