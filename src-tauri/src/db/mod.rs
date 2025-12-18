// Database operations module
pub mod connection;
pub mod migration_repair;

/// Get the database filename based on build mode.
/// - Debug (dev): tvc_dev.db
/// - Release (production): tvc.db
pub fn get_db_filename() -> &'static str {
    if cfg!(debug_assertions) {
        "tvc_dev.db"
    } else {
        "tvc.db"
    }
}

/// Get the SQLite connection string for tauri-plugin-sql
pub fn get_db_connection_string() -> String {
    format!("sqlite:{}", get_db_filename())
}
