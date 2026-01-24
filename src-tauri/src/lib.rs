mod arr;
mod commands;
mod db;
mod error;
mod plex;
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
    ];

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_process::init())
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
            // History commands
            commands::history::get_change_history,
            commands::history::get_change_history_stats,
            commands::history::clear_change_history,
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
        ])
        .setup(|app| {
            // Setup system tray
            let show_item = MenuItem::with_id(app, "show", "Show TVC", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            let _tray = TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
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
