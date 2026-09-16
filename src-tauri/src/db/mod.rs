// Database operations module
pub mod connection;
pub mod migration_repair;
pub mod tvdb_remap;

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

pub fn sql_plugin_config() -> serde_json::Value {
    serde_json::json!({ "preload": [get_db_connection_string()] })
}

#[cfg(test)]
mod tests {
    #[test]
    fn sql_preload_matches_active_database() {
        assert_eq!(
            super::sql_plugin_config()["preload"][0],
            super::get_db_connection_string()
        );
        if cfg!(debug_assertions) {
            assert_eq!(super::get_db_filename(), "tvc_dev.db");
            assert_eq!(
                super::sql_plugin_config()["preload"][0],
                "sqlite:tvc_dev.db"
            );
        }
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap();
        assert_eq!(config["plugins"]["sql"]["preload"], serde_json::json!([]));
    }
}
