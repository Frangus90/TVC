mod handler;
mod matcher;
pub mod models;

use axum::{routing::post, Router};
use sqlx::{Pool, Sqlite, Row};
use tauri::AppHandle;
use tokio::sync::Mutex;

use crate::db::connection;
use handler::AppState;
use models::PlexConfig;

/// Global state for the running webhook server
static SERVER_HANDLE: Mutex<Option<tokio::task::JoinHandle<()>>> = Mutex::const_new(None);
static SERVER_PORT: Mutex<Option<u16>> = Mutex::const_new(None);

/// Get Plex scrobbler configuration from database
pub async fn get_config(pool: &Pool<Sqlite>) -> PlexConfig {
    let row = sqlx::query("SELECT enabled, port FROM plex_scrobbler_config WHERE id = 1")
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

    match row {
        Some(r) => PlexConfig {
            enabled: r.get::<i32, _>("enabled") == 1,
            port: r.get::<i32, _>("port") as u16,
        },
        None => PlexConfig::default(),
    }
}

/// Update Plex scrobbler configuration
pub async fn update_config(pool: &Pool<Sqlite>, config: &PlexConfig) -> Result<(), String> {
    sqlx::query(
        r#"
        INSERT OR REPLACE INTO plex_scrobbler_config (id, enabled, port, updated_at)
        VALUES (1, ?, ?, datetime('now'))
        "#,
    )
    .bind(if config.enabled { 1 } else { 0 })
    .bind(config.port as i32)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to update config: {}", e))?;

    Ok(())
}

/// Start the Plex webhook server
pub async fn start_server(app: AppHandle, port: u16) -> Result<(), String> {
    // Check if already running
    {
        let handle = SERVER_HANDLE.lock().await;
        if handle.is_some() {
            return Err("Server already running".to_string());
        }
    }

    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let state = AppState {
        pool,
        app_handle: app,
    };

    let app_router = Router::new()
        .route("/webhook/plex", post(handler::handle_webhook))
        .with_state(state);

    let addr = format!("127.0.0.1:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("Failed to bind to port {}: {}", port, e))?;

    // Removed debug println! - server start is logged via status command if needed

    let handle = tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app_router).await {
            eprintln!("[Plex] Server error: {}", e);
        }
    });

    // Store the handle
    {
        let mut server_handle = SERVER_HANDLE.lock().await;
        *server_handle = Some(handle);
    }
    {
        let mut server_port = SERVER_PORT.lock().await;
        *server_port = Some(port);
    }

    Ok(())
}

/// Stop the Plex webhook server
pub async fn stop_server() -> Result<(), String> {
    let mut handle = SERVER_HANDLE.lock().await;

    if let Some(h) = handle.take() {
        h.abort();
    }

    let mut port = SERVER_PORT.lock().await;
    *port = None;

    Ok(())
}

/// Check if server is running
pub async fn is_server_running() -> bool {
    let handle = SERVER_HANDLE.lock().await;
    handle.is_some()
}

/// Get the current server port (if running)
pub async fn get_server_port() -> Option<u16> {
    let port = SERVER_PORT.lock().await;
    *port
}

/// Auto-start the webhook server if enabled in config
pub async fn auto_start_if_enabled(app: AppHandle) {
    let pool = match connection::get_pool(&app).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[Plex] Failed to get database pool for auto-start: {}", e);
            return;
        }
    };

    let config = get_config(&pool).await;

    if config.enabled {
        // Auto-start server if enabled (removed debug println!)
        if let Err(e) = start_server(app, config.port).await {
            eprintln!("[Plex] Failed to auto-start server: {}", e);
        }
    }
}
