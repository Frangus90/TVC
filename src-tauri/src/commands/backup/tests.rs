use super::*;
use serde_json::{json, Value};

async fn database() -> sqlx::SqlitePool {
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
    let restored = replace_from_pool(&target, &decoded, &HashMap::new())
        .await
        .unwrap();
    assert_eq!(restored.result.shows_imported, 2);
    assert_eq!(restored.result.episodes_imported, 2);
    assert_eq!(restored.result.movies_imported, 1);
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
    drop(guard);
    assert!(crate::library_sync::lock_for_restore().is_ok());
}
