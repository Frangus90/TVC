use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::sync::OnceLock;
use tokio::sync::Mutex;
use tauri::{AppHandle, Manager};
use crate::error::AppError;
use super::get_db_filename;

// Global pool storage - initialized once and reused
static POOL: OnceLock<Mutex<Option<SqlitePool>>> = OnceLock::new();

pub async fn get_pool(app: &AppHandle) -> Result<SqlitePool, AppError> {
    let pool_guard = POOL.get_or_init(|| Mutex::new(None));
    let mut pool_option = pool_guard.lock().await;

    if let Some(ref pool) = *pool_option {
        // Check if pool is still healthy
        if pool.is_closed() {
            // Pool was closed, need to recreate
            *pool_option = None;
        } else {
            return Ok(pool.clone());
        }
    }

    // Create new pool if it doesn't exist or was closed
    let app_data_dir = app.path().app_data_dir()
        .map_err(|e| AppError::Internal(format!("Failed to get app data directory: {}", e)))?;

    let db_path = app_data_dir.join(get_db_filename());
    let db_url = format!("sqlite:{}", db_path.to_string_lossy());

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .map_err(|e| AppError::Database(format!("Failed to connect to database: {}", e)))?;

    let pool_clone = pool.clone();
    *pool_option = Some(pool);
    Ok(pool_clone)
}

