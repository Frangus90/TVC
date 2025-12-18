//! Migration repair module - ensures old databases work with new versions
//!
//! This module runs BEFORE tauri-plugin-sql initializes to fix checksum mismatches
//! that occur when migration files are modified after release.

use rusqlite::Connection;
use sha2::{Digest, Sha384};
use std::path::PathBuf;
use super::get_db_filename;

/// Migration definition with version and SQL content
pub struct MigrationDef {
    pub version: i64,
    pub sql: &'static str,
}

/// Get the database path in the app data directory
fn get_db_path() -> Option<PathBuf> {
    let data_dir = dirs::data_dir()?;
    Some(data_dir.join("com.tvc.app").join(get_db_filename()))
}

/// Calculate SHA-384 checksum of migration SQL (matches sqlx format)
fn calculate_checksum(sql: &str) -> Vec<u8> {
    let mut hasher = Sha384::new();
    hasher.update(sql.as_bytes());
    hasher.finalize().to_vec()
}

/// Repair migration checksums in an existing database
///
/// This function:
/// 1. Opens the database directly (bypassing tauri-plugin-sql)
/// 2. Checks if _sqlx_migrations table exists
/// 3. Updates checksums to match current migration file contents
/// 4. Closes the database so tauri-plugin-sql can open it normally
pub fn repair_migration_checksums(migrations: &[MigrationDef]) {
    let db_path = match get_db_path() {
        Some(path) => path,
        None => {
            eprintln!("[migration_repair] Could not determine app data directory");
            return;
        }
    };

    // If database doesn't exist, nothing to repair
    if !db_path.exists() {
        return;
    }

    let conn = match Connection::open(&db_path) {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("[migration_repair] Failed to open database: {}", e);
            return;
        }
    };

    // Check if _sqlx_migrations table exists
    let table_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='_sqlx_migrations'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if !table_exists {
        return;
    }

    // Update checksums for each migration
    for migration in migrations {
        let new_checksum = calculate_checksum(migration.sql);

        // Check if this migration version exists in the table
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM _sqlx_migrations WHERE version = ?",
                [migration.version],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if exists {
            // Update the checksum
            if let Err(e) = conn.execute(
                "UPDATE _sqlx_migrations SET checksum = ? WHERE version = ?",
                rusqlite::params![new_checksum, migration.version],
            ) {
                eprintln!(
                    "[migration_repair] Failed to update checksum for v{}: {}",
                    migration.version, e
                );
            }
        }
    }
}
