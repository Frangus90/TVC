mod arr;
mod commands;
mod db;
mod error;
mod notifications;
mod plex;
mod racing;
mod tmdb;
mod tvdb;

use db::migration_repair::{repair_migration_checksums, MigrationDef};
use db::get_db_connection_string;
use tauri::Manager;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri_plugin_sql::{Migration, MigrationKind};

// Migration SQL content - embedded at compile time
const MIGRATION_001: &str = include_str!("../migrations/001_initial.sql");
const MIGRATION_002: &str = include_str!("../migrations/002_add_indexes.sql");
const MIGRATION_003: &str = include_str!("../migrations/003_add_show_metadata.sql");
const MIGRATION_004: &str = include_str!("../migrations/004_add_episode_metadata.sql");
const MIGRATION_005: &str = include_str!("../migrations/005_add_show_rating.sql");
const MIGRATION_006: &str = include_str!("../migrations/006_add_movies.sql");
const MIGRATION_007: &str = include_str!("../migrations/007_add_change_history.sql");
const MIGRATION_008: &str = include_str!("../migrations/008_add_cast_crew.sql");
const MIGRATION_009: &str = include_str!("../migrations/009_rating_to_real.sql");
const MIGRATION_010: &str = include_str!("../migrations/010_add_arr_integration.sql");
const MIGRATION_011: &str = include_str!("../migrations/011_add_rank_order.sql");
const MIGRATION_012: &str = include_str!("../migrations/012_add_racing.sql");
const MIGRATION_013: &str = include_str!("../migrations/013_add_notifications.sql");
const MIGRATION_014: &str = include_str!("../migrations/014_add_tiers.sql");

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // CRITICAL: Repair migration checksums BEFORE SQL plugin initializes
    // This ensures old databases work with new versions even if migration files changed
    repair_migration_checksums(&[
        MigrationDef { version: 1, sql: MIGRATION_001 },
        MigrationDef { version: 2, sql: MIGRATION_002 },
        MigrationDef { version: 3, sql: MIGRATION_003 },
        MigrationDef { version: 4, sql: MIGRATION_004 },
        MigrationDef { version: 5, sql: MIGRATION_005 },
        MigrationDef { version: 6, sql: MIGRATION_006 },
        MigrationDef { version: 7, sql: MIGRATION_007 },
        MigrationDef { version: 8, sql: MIGRATION_008 },
        MigrationDef { version: 9, sql: MIGRATION_009 },
        MigrationDef { version: 10, sql: MIGRATION_010 },
        MigrationDef { version: 11, sql: MIGRATION_011 },
        MigrationDef { version: 12, sql: MIGRATION_012 },
        MigrationDef { version: 13, sql: MIGRATION_013 },
        MigrationDef { version: 14, sql: MIGRATION_014 },
    ]);

    let migrations = vec![
        Migration {
            version: 1,
            description: "create initial tables",
            sql: MIGRATION_001,
            kind: MigrationKind::Up,
        },
        Migration {
            version: 2,
            description: "add performance indexes",
            sql: MIGRATION_002,
            kind: MigrationKind::Up,
        },
        Migration {
            version: 3,
            description: "add show metadata columns",
            sql: MIGRATION_003,
            kind: MigrationKind::Up,
        },
        Migration {
            version: 4,
            description: "add episode metadata columns",
            sql: MIGRATION_004,
            kind: MigrationKind::Up,
        },
        Migration {
            version: 5,
            description: "add show rating column",
            sql: MIGRATION_005,
            kind: MigrationKind::Up,
        },
        Migration {
            version: 6,
            description: "add movies table and archive support",
            sql: MIGRATION_006,
            kind: MigrationKind::Up,
        },
        Migration {
            version: 7,
            description: "add change history table",
            sql: MIGRATION_007,
            kind: MigrationKind::Up,
        },
        Migration {
            version: 8,
            description: "add cast and crew tables",
            sql: MIGRATION_008,
            kind: MigrationKind::Up,
        },
        Migration {
            version: 9,
            description: "convert rating to real for half-stars",
            sql: MIGRATION_009,
            kind: MigrationKind::Up,
        },
        Migration {
            version: 10,
            description: "add arr integration tables",
            sql: MIGRATION_010,
            kind: MigrationKind::Up,
        },
        Migration {
            version: 11,
            description: "add rank_order for tier ordering",
            sql: MIGRATION_011,
            kind: MigrationKind::Up,
        },
        Migration {
            version: 12,
            description: "add racing calendar tables",
            sql: MIGRATION_012,
            kind: MigrationKind::Up,
        },
        Migration {
            version: 13,
            description: "add notification system",
            sql: MIGRATION_013,
            kind: MigrationKind::Up,
        },
        Migration {
            version: 14,
            description: "add tier system and tier-only decoupling",
            sql: MIGRATION_014,
            kind: MigrationKind::Up,
        },
    ];

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // When a second instance is launched, focus the main window
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations(&get_db_connection_string(), migrations)
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            // TV Show commands
            commands::search_shows,
            commands::get_show_details,
            commands::sync_episodes_for_show,
            commands::shows::add_show,
            commands::shows::remove_show,
            commands::shows::get_tracked_shows,
            commands::shows::get_archived_shows,
            commands::shows::archive_show,
            commands::shows::unarchive_show,
            commands::shows::update_show_rating,
            commands::shows::reorder_show_in_tier,
            // Episode commands
            commands::episodes::sync_show_episodes,
            commands::episodes::sync_all_shows,
            commands::episodes::mark_episode_watched,
            commands::episodes::mark_season_watched,
            commands::episodes::mark_show_watched,
            commands::episodes::get_episodes_for_range,
            commands::episodes::schedule_episode,
            commands::episodes::unschedule_episode,
            // Movie commands
            commands::movies::search_movies,
            commands::movies::add_movie,
            commands::movies::remove_movie,
            commands::movies::get_tracked_movies,
            commands::movies::get_archived_movies,
            commands::movies::get_movie_details,
            commands::movies::update_movie_rating,
            commands::movies::reorder_movie_in_tier,
            commands::movies::mark_movie_watched,
            commands::movies::schedule_movie,
            commands::movies::unschedule_movie,
            commands::movies::archive_movie,
            commands::movies::unarchive_movie,
            commands::movies::sync_movie,
            commands::movies::sync_all_movies,
            commands::movies::get_movies_for_range,
            // Statistics commands
            commands::statistics::get_watch_statistics,
            commands::statistics::get_episodes_watched_by_period,
            commands::statistics::get_completion_rates,
            commands::statistics::get_watch_history,
            // Duplicates commands
            commands::duplicates::find_duplicates,
            commands::duplicates::merge_duplicates,
            // Maintenance commands
            commands::maintenance::get_database_stats,
            commands::maintenance::cleanup_orphaned_episodes,
            commands::maintenance::cleanup_unaired_episodes,
            commands::maintenance::get_orphaned_episodes_preview,
            commands::maintenance::get_unaired_episodes_preview,
            commands::maintenance::optimize_database,
            commands::maintenance::run_full_cleanup,
            // Metadata commands
            commands::metadata::fetch_movie_cast_crew,
            commands::metadata::fetch_show_cast,
            commands::metadata::get_movie_cast_crew,
            commands::metadata::get_show_cast,
            commands::metadata::get_movie_trailer,
            commands::metadata::get_show_trailer,
            // Backup commands
            commands::backup::export_database,
            commands::backup::import_database,
            // Arr (Sonarr/Radarr) commands
            commands::arr::get_arr_servers,
            commands::arr::add_arr_server,
            commands::arr::update_arr_server,
            commands::arr::delete_arr_server,
            commands::arr::test_arr_server,
            commands::arr::get_sonarr_library,
            commands::arr::get_radarr_library,
            commands::arr::import_from_sonarr,
            commands::arr::import_from_radarr,
            // Plex scrobbler commands
            commands::plex::get_plex_config,
            commands::plex::update_plex_config,
            commands::plex::start_plex_server,
            commands::plex::stop_plex_server,
            commands::plex::get_plex_server_status,
            commands::plex::get_scrobble_log,
            // Racing calendar commands
            commands::racing::get_racing_series,
            commands::racing::toggle_racing_series,
            commands::racing::update_racing_series_color,
            commands::racing::update_racing_series_notification,
            commands::racing::update_racing_series_ics_url,
            commands::racing::get_racing_events_for_range,
            commands::racing::refresh_racing_data,
            commands::racing::refresh_single_racing_series,
            commands::racing::get_racing_config,
            commands::racing::update_racing_config,
            commands::racing::test_racing_notification,
            // Tier commands
            commands::tiers::get_tiers,
            commands::tiers::create_tier,
            commands::tiers::update_tier,
            commands::tiers::delete_tier,
            commands::tiers::reorder_tiers,
            commands::tiers::get_tier_preset,
            commands::tiers::apply_tier_preset,
            commands::tiers::get_tier_list_shows,
            commands::tiers::get_tier_list_movies,
            commands::tiers::add_show_tier_only,
            commands::tiers::add_movie_tier_only,
            commands::tiers::add_manual_show,
            commands::tiers::add_manual_movie,
            commands::tiers::promote_show_to_tracked,
            commands::tiers::promote_movie_to_tracked,
            commands::tiers::demote_show_to_tier_only,
            commands::tiers::demote_movie_to_tier_only,
            commands::tiers::update_show_tier,
            commands::tiers::update_movie_tier,
            // Notification commands
            commands::notifications::get_notification_settings,
            commands::notifications::update_notification_settings,
            commands::notifications::get_notifications,
            commands::notifications::get_unread_notification_count,
            commands::notifications::mark_notification_read,
            commands::notifications::mark_all_notifications_read,
            commands::notifications::dismiss_notification,
            commands::notifications::dismiss_all_notifications,
            commands::notifications::test_in_app_notification,
            // App commands
            commands::app::exit_app,
        ])
        .setup(|app| {
            // Setup system tray
            let show_item = MenuItem::with_id(app, "show", "Show TVC", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            let icon = app.default_window_icon()
                .ok_or_else(|| "Failed to get default window icon")?;
            let _tray = TrayIconBuilder::with_id("main")
                .icon(icon.clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Start Plex scrobbler if enabled
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                plex::auto_start_if_enabled(app_handle).await;
            });

            // Start racing notification scheduler
            let app_handle2 = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                racing::auto_start_scheduler(app_handle2).await;
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Hide window instead of closing (minimize to tray)
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
