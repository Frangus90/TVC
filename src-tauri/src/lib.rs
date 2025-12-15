mod commands;
mod db;
mod error;
mod tvdb;

use db::migration_repair::{repair_migration_checksums, MigrationDef};
use tauri_plugin_sql::{Migration, MigrationKind};

// Migration SQL content - embedded at compile time
const MIGRATION_001: &str = include_str!("../migrations/001_initial.sql");
const MIGRATION_002: &str = include_str!("../migrations/002_add_indexes.sql");
const MIGRATION_003: &str = include_str!("../migrations/003_add_show_metadata.sql");
const MIGRATION_004: &str = include_str!("../migrations/004_add_episode_metadata.sql");
const MIGRATION_005: &str = include_str!("../migrations/005_add_show_rating.sql");

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
    ];

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations("sqlite:tvc.db", migrations)
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            commands::search_shows,
            commands::get_show_details,
            commands::sync_episodes_for_show,
            commands::shows::add_show,
            commands::shows::remove_show,
            commands::shows::get_tracked_shows,
            commands::shows::update_show_rating,
            commands::episodes::sync_show_episodes,
            commands::episodes::mark_episode_watched,
            commands::episodes::get_episodes_for_range,
            commands::episodes::schedule_episode,
            commands::episodes::unschedule_episode,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
