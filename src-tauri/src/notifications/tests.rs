use super::*;
use sqlx::sqlite::SqlitePoolOptions;

async fn memory_pool() -> Pool<Sqlite> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::query("CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::raw_sql(include_str!("../../migrations/013_add_notifications.sql"))
        .execute(&pool)
        .await
        .unwrap();
    pool
}

#[test]
fn old_settings_payloads_keep_existing_values_and_default_quiet_hours_off() {
    let mut value = serde_json::to_value(NotificationSettings::default()).unwrap();
    let object = value.as_object_mut().unwrap();
    object.remove("quiet_hours_enabled");
    object.remove("quiet_hours_start");
    object.remove("quiet_hours_end");
    object.remove("auto_clear_after_days");
    object.insert("sound_volume".to_string(), serde_json::json!(75));
    object.insert("os_fallback".to_string(), serde_json::json!(true));
    let settings: NotificationSettings = serde_json::from_value(value).unwrap();
    assert!(!settings.quiet_hours_enabled);
    assert_eq!(settings.quiet_hours_start, "22:00");
    assert_eq!(settings.quiet_hours_end, "08:00");
    assert_eq!(settings.auto_clear_after_days, 0);
    assert_eq!(settings.sound_volume, 75);
    assert!(settings.os_fallback);
}

#[test]
fn quiet_hours_cover_daytime_and_overnight_with_exclusive_end() {
    let mut settings = NotificationSettings {
        quiet_hours_enabled: true,
        quiet_hours_start: "09:00".into(),
        quiet_hours_end: "17:00".into(),
        ..NotificationSettings::default()
    };
    let at = |hour, minute| NaiveTime::from_hms_opt(hour, minute, 0).unwrap();
    assert!(!is_quiet_hours_at(&settings, at(8, 59)));
    assert!(is_quiet_hours_at(&settings, at(9, 0)));
    assert!(is_quiet_hours_at(&settings, at(16, 59)));
    assert!(!is_quiet_hours_at(&settings, at(17, 0)));
    settings.quiet_hours_start = "22:00".into();
    settings.quiet_hours_end = "08:00".into();
    assert!(!is_quiet_hours_at(&settings, at(21, 59)));
    assert!(is_quiet_hours_at(&settings, at(22, 0)));
    assert!(is_quiet_hours_at(&settings, at(23, 59)));
    assert!(is_quiet_hours_at(&settings, at(0, 0)));
    assert!(is_quiet_hours_at(&settings, at(7, 59)));
    assert!(!is_quiet_hours_at(&settings, at(8, 0)));
    settings.quiet_hours_enabled = false;
    assert!(!is_quiet_hours_at(&settings, at(23, 0)));
}

#[tokio::test]
async fn existing_schema_defaults_and_quiet_hours_roundtrip_without_a_migration() {
    let pool = memory_pool().await;
    let old = get_settings_checked(&pool).await.unwrap();
    assert!(!old.quiet_hours_enabled);
    assert_eq!(old.auto_clear_after_days, 0);
    assert_eq!(old.popup_duration, 8000);
    let changed = NotificationSettings {
        quiet_hours_enabled: true,
        quiet_hours_start: "23:15".into(),
        quiet_hours_end: "07:45".into(),
        sound_volume: 65,
        os_fallback: true,
        tray_notifications: false,
        ..old
    };
    update_settings(&pool, &changed).await.unwrap();
    let loaded = get_settings_checked(&pool).await.unwrap();
    assert!(loaded.quiet_hours_enabled);
    assert_eq!(loaded.quiet_hours_start, "23:15");
    assert_eq!(loaded.quiet_hours_end, "07:45");
    assert_eq!(loaded.sound_volume, 65);
    assert!(loaded.os_fallback);
    assert!(!loaded.tray_notifications);
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM settings WHERE key LIKE 'notification_quiet_hours_%'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 3);
}

#[tokio::test]
async fn invalid_times_do_not_change_existing_preferences() {
    let pool = memory_pool().await;
    for value in ["24:00", "8:00", "08:60", "8 PM", ""] {
        let bad = NotificationSettings {
            sound_volume: 80,
            quiet_hours_enabled: true,
            quiet_hours_start: value.into(),
            ..NotificationSettings::default()
        };
        assert!(update_settings(&pool, &bad).await.is_err());
        assert_eq!(get_settings_checked(&pool).await.unwrap().sound_volume, 50);
    }
    let same = NotificationSettings {
        quiet_hours_enabled: true,
        quiet_hours_start: "08:00".into(),
        quiet_hours_end: "08:00".into(),
        ..NotificationSettings::default()
    };
    assert!(update_settings(&pool, &same).await.is_err());
}

#[tokio::test]
async fn preference_transaction_rolls_back_if_quiet_hours_cannot_be_saved() {
    let pool = memory_pool().await;
    sqlx::query("DROP TABLE settings")
        .execute(&pool)
        .await
        .unwrap();
    let changed = NotificationSettings {
        sound_volume: 80,
        quiet_hours_enabled: true,
        ..NotificationSettings::default()
    };
    assert!(update_settings(&pool, &changed).await.is_err());
    let volume: i32 =
        sqlx::query_scalar("SELECT sound_volume FROM notification_settings WHERE id=1")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(volume, 50);
    assert!(
        get_settings_checked(&pool).await.is_err(),
        "read failures must reach the Settings page"
    );
}

#[tokio::test]
async fn auto_clear_choices_roundtrip_in_existing_settings_table() {
    let pool = memory_pool().await;
    for days in [1, 7, 30, 0] {
        let settings = NotificationSettings {
            auto_clear_after_days: days,
            ..NotificationSettings::default()
        };
        update_settings(&pool, &settings).await.unwrap();
        assert_eq!(
            get_settings_checked(&pool)
                .await
                .unwrap()
                .auto_clear_after_days,
            days
        );
        let saved: String = sqlx::query_scalar(
            "SELECT value FROM settings WHERE key = 'notification_auto_clear_after_days'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(saved, days.to_string());
    }
}

#[tokio::test]
async fn invalid_auto_clear_choices_do_not_change_preferences() {
    let pool = memory_pool().await;
    insert_history(&pool, 1, Some("2020-01-01 00:00:00"), false, false).await;
    for days in [-1, 2, 31, i32::MAX] {
        let settings = NotificationSettings {
            auto_clear_after_days: days,
            sound_volume: 80,
            quiet_hours_enabled: true,
            ..NotificationSettings::default()
        };
        assert!(update_settings(&pool, &settings).await.is_err());
        let loaded = get_settings_checked(&pool).await.unwrap();
        assert_eq!(loaded.sound_volume, 50);
        assert!(!loaded.quiet_hours_enabled);
        assert_eq!(loaded.auto_clear_after_days, 0);
    }
    for saved in ["2", "not-a-number"] {
        sqlx::query("INSERT OR REPLACE INTO settings (key, value) VALUES ('notification_auto_clear_after_days', ?)")
            .bind(saved).execute(&pool).await.unwrap();
        assert!(get_settings_checked(&pool).await.is_err());
        assert!(clear_old_notifications_at(&pool, test_now()).await.is_err());
        assert_eq!(get_unread_count(&pool).await.unwrap(), 1);
    }
}

#[tokio::test]
async fn failed_auto_clear_save_rolls_back_other_preferences() {
    let pool = memory_pool().await;
    update_settings(&pool, &NotificationSettings::default())
        .await
        .unwrap();
    sqlx::query("CREATE TRIGGER fail_auto_clear_setting BEFORE INSERT ON settings WHEN NEW.key = 'notification_auto_clear_after_days' BEGIN SELECT RAISE(ABORT, 'test save failure'); END")
        .execute(&pool).await.unwrap();
    let settings = NotificationSettings {
        auto_clear_after_days: 7,
        sound_volume: 80,
        quiet_hours_enabled: true,
        quiet_hours_start: "23:00".into(),
        ..NotificationSettings::default()
    };
    assert!(update_settings(&pool, &settings).await.is_err());
    let loaded = get_settings_checked(&pool).await.unwrap();
    assert_eq!(loaded.sound_volume, 50);
    assert!(!loaded.quiet_hours_enabled);
    assert_eq!(loaded.quiet_hours_start, "22:00");
    assert_eq!(loaded.auto_clear_after_days, 0);
}

fn test_now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-10-24T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
}

async fn insert_history(
    pool: &Pool<Sqlite>,
    id: i64,
    created_at: Option<&str>,
    read: bool,
    dismissed: bool,
) {
    sqlx::query("INSERT INTO notifications (id, type, title, body, read, dismissed, created_at, expires_at) VALUES (?, 'system', 'Test', 'Keep this body', ?, ?, ?, '2020-01-01T00:00:00Z')")
        .bind(id).bind(read).bind(dismissed).bind(created_at)
        .execute(pool).await.unwrap();
}

#[tokio::test]
async fn auto_clear_uses_exact_day_week_and_thirty_day_boundaries_for_read_and_unread() {
    for days in [1, 7, 30] {
        let pool = memory_pool().await;
        update_settings(
            &pool,
            &NotificationSettings {
                auto_clear_after_days: days,
                ..NotificationSettings::default()
            },
        )
        .await
        .unwrap();
        let cutoff = test_now() - Duration::days(i64::from(days));
        let before = (cutoff - Duration::milliseconds(1)).to_rfc3339();
        let boundary = cutoff.to_rfc3339();
        let recent = (cutoff + Duration::milliseconds(1)).to_rfc3339();
        let future = (test_now() + Duration::days(1)).to_rfc3339();
        insert_history(&pool, 1, Some(&before), false, false).await;
        insert_history(&pool, 2, Some(&before), true, false).await;
        insert_history(&pool, 3, Some(&boundary), false, false).await;
        insert_history(&pool, 4, Some(&recent), false, false).await;
        insert_history(&pool, 5, Some(&future), false, false).await;
        insert_history(&pool, 6, Some(&before), false, true).await;

        assert_eq!(get_unread_count(&pool).await.unwrap(), 4);
        assert_eq!(
            get_notifications(&pool, 50, 0, false).await.unwrap().len(),
            5
        );
        let cleared = clear_old_notifications_at(&pool, test_now()).await.unwrap();
        assert_eq!(cleared, vec![1, 2]);
        assert_eq!(get_unread_count(&pool).await.unwrap(), 3);
        let active: Vec<i64> = get_notifications(&pool, 50, 0, false)
            .await
            .unwrap()
            .into_iter()
            .map(|notification| notification.id)
            .collect();
        assert_eq!(active.len(), 3);
        assert!([3, 4, 5].into_iter().all(|id| active.contains(&id)));
        assert!(clear_old_notifications_at(&pool, test_now())
            .await
            .unwrap()
            .is_empty());

        let stored: Vec<(i64, bool, String, String)> =
            sqlx::query_as("SELECT id, read, body, expires_at FROM notifications ORDER BY id")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(stored.len(), 6, "auto-clear only dismisses history");
        assert!(stored.iter().all(|(id, read, body, expires)| {
            *read == (*id == 2) && body == "Keep this body" && expires == "2020-01-01T00:00:00Z"
        }));
    }
}

#[tokio::test]
async fn auto_clear_normalizes_timezones_and_preserves_invalid_timestamps() {
    let pool = memory_pool().await;
    update_settings(
        &pool,
        &NotificationSettings {
            auto_clear_after_days: 1,
            ..NotificationSettings::default()
        },
    )
    .await
    .unwrap();
    for (id, timestamp) in [
        (1, Some("2026-10-23 11:59:59")),
        (2, Some("2026-10-23 11:59:59.999")),
        (3, Some("2026-10-23T13:59:59+02:00")),
        (4, Some("2026-10-23T06:59:59-05:00")),
        (5, Some("2026-10-23 12:00:00")),
        (6, Some("2026-10-23T14:00:00+02:00")),
        (7, Some("2026-10-23T07:00:00-05:00")),
        (8, Some("not a date")),
        (9, Some("2025-02-30 12:00:00")),
        (10, Some("2025-02-30T12:00:00Z")),
        (11, Some("")),
        (12, None),
        (13, Some("2026-10-23T12:00:00.001Z")),
    ] {
        insert_history(&pool, id, timestamp, false, false).await;
    }
    assert_eq!(
        clear_old_notifications_at(&pool, test_now()).await.unwrap(),
        vec![1, 2, 3, 4]
    );
    assert_eq!(get_unread_count(&pool).await.unwrap(), 9);
}

#[tokio::test]
async fn auto_clear_off_preserves_all_history_and_other_tables() {
    let pool = memory_pool().await;
    insert_history(&pool, 1, Some("2020-01-01 00:00:00"), false, false).await;
    insert_history(&pool, 2, Some("2020-01-01 00:00:00"), true, false).await;
    sqlx::query("CREATE TABLE watched_episodes (id INTEGER PRIMARY KEY, watched INTEGER)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO watched_episodes VALUES (1, 1)")
        .execute(&pool)
        .await
        .unwrap();
    assert!(clear_old_notifications_at(&pool, test_now())
        .await
        .unwrap()
        .is_empty());
    assert_eq!(
        get_notifications(&pool, 50, 0, false).await.unwrap().len(),
        2
    );

    update_settings(
        &pool,
        &NotificationSettings {
            auto_clear_after_days: 1,
            ..NotificationSettings::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(
        clear_old_notifications_at(&pool, test_now()).await.unwrap(),
        vec![1, 2]
    );
    let watched: i64 = sqlx::query_scalar("SELECT watched FROM watched_episodes WHERE id = 1")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(watched, 1);
    assert_eq!(
        get_settings_checked(&pool)
            .await
            .unwrap()
            .auto_clear_after_days,
        1
    );
}

#[tokio::test]
async fn failed_auto_clear_rolls_back_every_dismissal() {
    let pool = memory_pool().await;
    update_settings(
        &pool,
        &NotificationSettings {
            auto_clear_after_days: 1,
            ..NotificationSettings::default()
        },
    )
    .await
    .unwrap();
    insert_history(&pool, 1, Some("2020-01-01 00:00:00"), false, false).await;
    insert_history(&pool, 2, Some("2020-01-01 00:00:00"), false, false).await;
    sqlx::query("CREATE TRIGGER fail_auto_clear BEFORE UPDATE ON notifications WHEN NEW.id = 2 BEGIN SELECT RAISE(ABORT, 'test cleanup failure'); END")
        .execute(&pool).await.unwrap();
    assert!(clear_old_notifications_at(&pool, test_now()).await.is_err());
    assert_eq!(get_unread_count(&pool).await.unwrap(), 2);
}
