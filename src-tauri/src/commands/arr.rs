use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::AppHandle;

use crate::arr::{
    ArrServer, ArrServerRequest, ArrSystemStatus, ImportResult, LibraryItem,
    RadarrClient, SonarrClient,
};
use crate::db::connection;
use crate::tmdb;
use crate::tvdb;

// ============================================================================
// Server Management Commands
// ============================================================================

/// Get all configured arr servers
#[tauri::command]
pub async fn get_arr_servers(app: AppHandle) -> Result<Vec<ArrServer>, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let rows = sqlx::query(
        r#"
        SELECT id, name, type, base_url, api_key, is_active,
               auto_sync_enabled, sync_interval_hours, last_synced, added_at
        FROM arr_servers
        ORDER BY name
        "#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to get arr servers: {}", e))?;

    let servers: Vec<ArrServer> = rows
        .into_iter()
        .map(|row| ArrServer {
            id: row.get("id"),
            name: row.get("name"),
            server_type: row.get("type"),
            base_url: row.get("base_url"),
            api_key: row.get("api_key"),
            is_active: row.get::<i32, _>("is_active") == 1,
            auto_sync_enabled: row.get::<i32, _>("auto_sync_enabled") == 1,
            sync_interval_hours: row.get("sync_interval_hours"),
            last_synced: row.get("last_synced"),
            added_at: row.get("added_at"),
        })
        .collect();

    Ok(servers)
}

/// Add a new arr server
#[tauri::command]
pub async fn add_arr_server(app: AppHandle, server: ArrServerRequest) -> Result<i64, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let result = sqlx::query(
        r#"
        INSERT INTO arr_servers (name, type, base_url, api_key)
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(&server.name)
    .bind(&server.server_type)
    .bind(&server.base_url)
    .bind(&server.api_key)
    .execute(&pool)
    .await
    .map_err(|e| format!("Failed to add arr server: {}", e))?;

    Ok(result.last_insert_rowid())
}

/// Update an existing arr server
#[tauri::command]
pub async fn update_arr_server(
    app: AppHandle,
    id: i64,
    server: ArrServerRequest,
) -> Result<(), String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    sqlx::query(
        r#"
        UPDATE arr_servers
        SET name = ?, type = ?, base_url = ?, api_key = ?
        WHERE id = ?
        "#,
    )
    .bind(&server.name)
    .bind(&server.server_type)
    .bind(&server.base_url)
    .bind(&server.api_key)
    .bind(id)
    .execute(&pool)
    .await
    .map_err(|e| format!("Failed to update arr server: {}", e))?;

    Ok(())
}

/// Delete an arr server
#[tauri::command]
pub async fn delete_arr_server(app: AppHandle, id: i64) -> Result<(), String> {
    crate::commands::validation::validate_id(id)?;
    
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    sqlx::query("DELETE FROM arr_servers WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to delete arr server: {}", e))?;

    Ok(())
}

/// Test connection to an arr server
#[tauri::command]
pub async fn test_arr_server(
    base_url: String,
    api_key: String,
    server_type: String,
) -> Result<ArrSystemStatus, String> {
    match server_type.as_str() {
        "sonarr" => {
            let client = SonarrClient::new(&base_url, &api_key);
            client
                .test_connection()
                .await
                .map_err(|e| format!("Connection failed: {}", e))
        }
        "radarr" => {
            let client = RadarrClient::new(&base_url, &api_key);
            client
                .test_connection()
                .await
                .map_err(|e| format!("Connection failed: {}", e))
        }
        _ => Err(format!("Unknown server type: {}", server_type)),
    }
}

// ============================================================================
// Library Preview Commands
// ============================================================================

/// Get library items from a Sonarr server for preview/import
#[tauri::command]
pub async fn get_sonarr_library(
    app: AppHandle,
    server_id: i64,
) -> Result<Vec<LibraryItem>, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    // Get server details
    let row = sqlx::query("SELECT base_url, api_key FROM arr_servers WHERE id = ? AND type = 'sonarr'")
        .bind(server_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let (base_url, api_key): (String, String) = match row {
        Some(r) => (r.get("base_url"), r.get("api_key")),
        None => return Err("Sonarr server not found".to_string()),
    };

    // Get all TVDB IDs already tracked
    let tracked_ids: Vec<i64> = sqlx::query_scalar("SELECT id FROM shows")
        .fetch_all(&pool)
        .await
        .map_err(|e| format!("Failed to get tracked shows: {}", e))?;

    // Get library from Sonarr
    let client = SonarrClient::new(&base_url, &api_key);
    let series = client
        .get_series()
        .await
        .map_err(|e| format!("Failed to get Sonarr library: {}", e))?;

    let items: Vec<LibraryItem> = series
        .into_iter()
        .map(|s| {
            let tvdb_id = s.tvdb_id;
            let poster_url = s.poster_url();
            LibraryItem {
                arr_id: s.id,
                title: s.title,
                year: s.year,
                poster_url,
                monitored: s.monitored,
                tvdb_id,
                tmdb_id: None,
                already_tracked: tvdb_id.map(|id| tracked_ids.contains(&id)).unwrap_or(false),
            }
        })
        .collect();

    Ok(items)
}

/// Get library items from a Radarr server for preview/import
#[tauri::command]
pub async fn get_radarr_library(
    app: AppHandle,
    server_id: i64,
) -> Result<Vec<LibraryItem>, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    // Get server details
    let row = sqlx::query("SELECT base_url, api_key FROM arr_servers WHERE id = ? AND type = 'radarr'")
        .bind(server_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let (base_url, api_key): (String, String) = match row {
        Some(r) => (r.get("base_url"), r.get("api_key")),
        None => return Err("Radarr server not found".to_string()),
    };

    // Get all TMDB IDs already tracked
    let tracked_ids: Vec<i64> = sqlx::query_scalar("SELECT id FROM movies")
        .fetch_all(&pool)
        .await
        .map_err(|e| format!("Failed to get tracked movies: {}", e))?;

    // Get library from Radarr
    let client = RadarrClient::new(&base_url, &api_key);
    let movies = client
        .get_movies()
        .await
        .map_err(|e| format!("Failed to get Radarr library: {}", e))?;

    let items: Vec<LibraryItem> = movies
        .into_iter()
        .map(|m| {
            let tmdb_id = m.tmdb_id;
            let poster_url = m.poster_url();
            LibraryItem {
                arr_id: m.id,
                title: m.title,
                year: m.year,
                poster_url,
                monitored: m.monitored,
                tvdb_id: None,
                tmdb_id,
                already_tracked: tmdb_id.map(|id| tracked_ids.contains(&id)).unwrap_or(false),
            }
        })
        .collect();

    Ok(items)
}

// ============================================================================
// Import Commands
// ============================================================================

/// Import request with selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportRequest {
    pub server_id: i64,
    pub items: Vec<ImportItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportItem {
    pub arr_id: i64,
    pub tvdb_id: Option<i64>,
    pub tmdb_id: Option<i64>,
}

/// Import selected shows from Sonarr
#[tauri::command]
pub async fn import_from_sonarr(
    app: AppHandle,
    request: ImportRequest,
) -> Result<ImportResult, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let mut result = ImportResult {
        total: request.items.len(),
        imported: 0,
        skipped: 0,
        failed: 0,
        errors: Vec::new(),
    };

    for item in request.items {
        // Skip items without TVDB ID
        let tvdb_id = match item.tvdb_id {
            Some(id) => id,
            None => {
                result.skipped += 1;
                result.errors.push(format!(
                    "Skipped arr_id {}: No TVDB ID available",
                    item.arr_id
                ));
                continue;
            }
        };

        // Check if already tracked
        let exists: Option<i64> = sqlx::query_scalar("SELECT id FROM shows WHERE id = ?")
            .bind(tvdb_id)
            .fetch_optional(&pool)
            .await
            .map_err(|e| format!("Database error: {}", e))?;

        if exists.is_some() {
            result.skipped += 1;
            continue;
        }

        // Fetch show details from TVDB and add to database
        match add_show_from_tvdb(&app, tvdb_id).await {
            Ok(_) => {
                // Record the import
                let _ = sqlx::query(
                    r#"
                    INSERT INTO sonarr_imports (show_id, sonarr_series_id, arr_server_id, monitored)
                    VALUES (?, ?, ?, 1)
                    "#,
                )
                .bind(tvdb_id)
                .bind(item.arr_id)
                .bind(request.server_id)
                .execute(&pool)
                .await;

                result.imported += 1;
            }
            Err(e) => {
                result.failed += 1;
                result.errors.push(format!("Failed to import TVDB {}: {}", tvdb_id, e));
            }
        }
    }

    // Update last_synced timestamp
    let _ = sqlx::query("UPDATE arr_servers SET last_synced = datetime('now') WHERE id = ?")
        .bind(request.server_id)
        .execute(&pool)
        .await;

    Ok(result)
}

/// Import selected movies from Radarr
#[tauri::command]
pub async fn import_from_radarr(
    app: AppHandle,
    request: ImportRequest,
) -> Result<ImportResult, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let mut result = ImportResult {
        total: request.items.len(),
        imported: 0,
        skipped: 0,
        failed: 0,
        errors: Vec::new(),
    };

    for item in request.items {
        // Skip items without TMDB ID
        let tmdb_id = match item.tmdb_id {
            Some(id) => id,
            None => {
                result.skipped += 1;
                result.errors.push(format!(
                    "Skipped arr_id {}: No TMDB ID available",
                    item.arr_id
                ));
                continue;
            }
        };

        // Check if already tracked
        let exists: Option<i64> = sqlx::query_scalar("SELECT id FROM movies WHERE id = ?")
            .bind(tmdb_id)
            .fetch_optional(&pool)
            .await
            .map_err(|e| format!("Database error: {}", e))?;

        if exists.is_some() {
            result.skipped += 1;
            continue;
        }

        // Fetch movie details from TMDB and add to database
        match add_movie_from_tmdb(&app, tmdb_id).await {
            Ok(_) => {
                // Record the import
                let _ = sqlx::query(
                    r#"
                    INSERT INTO radarr_imports (movie_id, radarr_movie_id, arr_server_id, monitored)
                    VALUES (?, ?, ?, 1)
                    "#,
                )
                .bind(tmdb_id)
                .bind(item.arr_id)
                .bind(request.server_id)
                .execute(&pool)
                .await;

                result.imported += 1;
            }
            Err(e) => {
                result.failed += 1;
                result.errors.push(format!("Failed to import TMDB {}: {}", tmdb_id, e));
            }
        }
    }

    // Update last_synced timestamp
    let _ = sqlx::query("UPDATE arr_servers SET last_synced = datetime('now') WHERE id = ?")
        .bind(request.server_id)
        .execute(&pool)
        .await;

    Ok(result)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Add a show from TVDB (simplified version of shows::add_show)
async fn add_show_from_tvdb(app: &AppHandle, tvdb_id: i64) -> Result<(), String> {
    let pool = connection::get_pool(app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    // Fetch show details from TVDB
    let show = tvdb::get_series_extended(tvdb_id)
        .await
        .map_err(|e| format!("Failed to fetch show from TVDB: {}", e))?;

    let airs_days_json = show.airs_days
        .as_ref()
        .and_then(|days| serde_json::to_string(days).ok());

    sqlx::query(
        r#"
        INSERT OR REPLACE INTO shows
        (id, name, slug, status, poster_url, first_aired, overview, airs_time, airs_days, runtime, added_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))
        "#,
    )
    .bind(show.id)
    .bind(&show.name)
    .bind(show.slug.as_ref())
    .bind(show.status.as_ref().and_then(|s| s.name.as_ref()))
    .bind(show.image.as_ref())
    .bind(show.first_aired.as_ref())
    .bind(show.overview.as_ref())
    .bind(show.airs_time.as_ref())
    .bind(airs_days_json.as_deref())
    .bind(show.average_runtime)
    .execute(&pool)
    .await
    .map_err(|e| format!("Failed to add show: {}", e))?;

    Ok(())
}

/// Add a movie from TMDB (simplified version of movies::add_movie)
async fn add_movie_from_tmdb(app: &AppHandle, tmdb_id: i64) -> Result<(), String> {
    let pool = connection::get_pool(app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    // Fetch movie details from TMDB
    let (movie_details, release_dates) = tmdb::get_movie_with_release_dates(tmdb_id, "US")
        .await
        .map_err(|e| format!("Failed to fetch movie from TMDB: {}", e))?;

    let genres_json = movie_details.genres_string();

    sqlx::query(
        r#"
        INSERT OR REPLACE INTO movies
        (id, title, tagline, overview, poster_url, backdrop_url, release_date,
         digital_release_date, physical_release_date, runtime, status, genres,
         vote_average, added_at, last_synced)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))
        "#,
    )
    .bind(movie_details.id)
    .bind(&movie_details.title)
    .bind(movie_details.tagline.as_ref())
    .bind(movie_details.overview.as_ref())
    .bind(movie_details.poster_url())
    .bind(movie_details.backdrop_url())
    .bind(release_dates.theatrical.as_ref())
    .bind(release_dates.digital.as_ref())
    .bind(release_dates.physical.as_ref())
    .bind(movie_details.runtime)
    .bind(movie_details.status.as_ref())
    .bind(genres_json.as_ref())
    .bind(movie_details.vote_average)
    .execute(&pool)
    .await
    .map_err(|e| format!("Failed to add movie: {}", e))?;

    Ok(())
}
