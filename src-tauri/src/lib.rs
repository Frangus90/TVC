mod commands;
mod db;
mod error;
mod tvdb;

use tauri_plugin_sql::{Migration, MigrationKind};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let migrations = vec![
        Migration {
            version: 1,
            description: "create initial tables",
            sql: include_str!("../migrations/001_initial.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 2,
            description: "add performance indexes",
            sql: include_str!("../migrations/002_add_indexes.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 3,
            description: "add show metadata columns",
            sql: include_str!("../migrations/003_add_show_metadata.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 4,
            description: "add episode metadata columns",
            sql: include_str!("../migrations/004_add_episode_metadata.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 5,
            description: "add show rating column",
            sql: include_str!("../migrations/005_add_show_rating.sql"),
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
