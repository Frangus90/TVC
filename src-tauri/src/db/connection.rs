use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use tauri::{AppHandle, Manager};
use crate::error::AppError;

pub async fn get_pool(app: &AppHandle) -> Result<SqlitePool, AppError> {
    // Use the same database path as tauri-plugin-sql
    // The plugin uses "sqlite:tvc.db" which resolves to app data directory
    let app_data_dir = app.path().app_data_dir()
        .map_err(|e| AppError::Internal(format!("Failed to get app data directory: {}", e)))?;
    
    let db_path = app_data_dir.join("tvc.db");
    let db_url = format!("sqlite:{}", db_path.to_string_lossy());

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .map_err(|e| AppError::Database(format!("Failed to connect to database: {}", e)))?;

    Ok(pool)
}

