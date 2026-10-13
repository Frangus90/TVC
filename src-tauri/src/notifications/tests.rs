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
    object.insert("sound_volume".to_string(), serde_json::json!(75));
    object.insert("os_fallback".to_string(), serde_json::json!(true));
    let settings: NotificationSettings = serde_json::from_value(value).unwrap();
    assert!(!settings.quiet_hours_enabled);
    assert_eq!(settings.quiet_hours_start, "22:00");
    assert_eq!(settings.quiet_hours_end, "08:00");
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
