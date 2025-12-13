mod commands;
mod db;
mod tvdb;

use tauri_plugin_sql::{Migration, MigrationKind};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let migrations = vec![Migration {
        version: 1,
        description: "create initial tables",
        sql: include_str!("../migrations/001_initial.sql"),
        kind: MigrationKind::Up,
    }];

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
            commands::add_show,
            commands::remove_show,
            commands::get_tracked_shows,
            commands::sync_episodes_for_show,
            commands::sync_show_episodes,
            commands::mark_episode_watched,
            commands::get_episodes_for_range,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
