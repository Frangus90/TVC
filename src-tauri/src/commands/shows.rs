use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row};
use tauri::AppHandle;
use crate::db::{connection, tvdb_remap};
use crate::tmdb;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct TrackedShow {
    pub id: i64,
    pub name: String,
    pub poster_url: Option<String>,
    pub status: Option<String>,
    pub color: Option<String>,
    pub notes: Option<String>,
    pub tags: Option<String>,
    pub rating: Option<f64>,
    pub rank_order: Option<i32>,
    pub tier_id: Option<i64>,
    pub tier_only: bool,
    pub unmigrated: bool,
}

#[tauri::command]
pub async fn add_show(app: AppHandle, id: i64) -> Result<(), String> {
    crate::commands::validation::validate_id(id)?;

    let show_details = tmdb::get_tv_details(id)
        .await
        .map_err(|e| format!("Failed to fetch show details: {}", e))?;

    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    sqlx::query(
        r#"
        INSERT INTO shows
        (id, name, status, poster_url, first_aired, overview, network, runtime, added_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))
        ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            status = excluded.status,
            poster_url = excluded.poster_url,
            first_aired = excluded.first_aired,
            overview = excluded.overview,
            network = excluded.network,
            runtime = excluded.runtime
        "#,
    )
    .bind(show_details.id)
    .bind(&show_details.name)
    .bind(show_details.status.as_ref())
    .bind(show_details.poster_url())
    .bind(show_details.first_air_date.as_ref())
    .bind(show_details.overview.as_ref())
    .bind(show_details.network_name())
    .bind(show_details.runtime())
    .execute(&pool)
    .await
    .map_err(|e| format!("Failed to add show: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn remove_show(app: AppHandle, id: i64) -> Result<(), String> {
    crate::commands::validation::validate_id(id)?;
    
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    sqlx::query("DELETE FROM shows WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to remove show: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn get_tracked_shows(app: AppHandle) -> Result<Vec<TrackedShow>, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let rows = sqlx::query(
        r#"
        SELECT id, name, poster_url, status, color, notes, tags, rating, rank_order, tier_id, tier_only, unmigrated
        FROM shows
        WHERE (archived = 0 OR archived IS NULL) AND tier_only = 0
        ORDER BY name
        LIMIT 10000
        "#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to get tracked shows: {}", e))?;

    let shows = rows.into_iter().map(|row| TrackedShow {
        id: row.get("id"),
        name: row.get("name"),
        poster_url: row.get("poster_url"),
        status: row.get("status"),
        color: row.get("color"),
        notes: row.get("notes"),
        tags: row.get("tags"),
        rating: row.get("rating"),
        rank_order: row.get("rank_order"),
        tier_id: row.get("tier_id"),
        tier_only: row.get::<i32, _>("tier_only") == 1,
        unmigrated: row.get::<i32, _>("unmigrated") == 1,
    }).collect();

    Ok(shows)
}

#[tauri::command]
pub async fn get_archived_shows(app: AppHandle) -> Result<Vec<TrackedShow>, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let rows = sqlx::query(
        r#"
        SELECT id, name, poster_url, status, color, notes, tags, rating, rank_order, tier_id, tier_only, unmigrated
        FROM shows
        WHERE archived = 1 AND tier_only = 0
        ORDER BY name
        LIMIT 10000
        "#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to get archived shows: {}", e))?;

    let shows = rows.into_iter().map(|row| TrackedShow {
        id: row.get("id"),
        name: row.get("name"),
        poster_url: row.get("poster_url"),
        status: row.get("status"),
        color: row.get("color"),
        notes: row.get("notes"),
        tags: row.get("tags"),
        rating: row.get("rating"),
        rank_order: row.get("rank_order"),
        tier_id: row.get("tier_id"),
        tier_only: row.get::<i32, _>("tier_only") == 1,
        unmigrated: row.get::<i32, _>("unmigrated") == 1,
    }).collect();

    Ok(shows)
}

#[tauri::command]
pub async fn archive_show(app: AppHandle, id: i64) -> Result<(), String> {
    crate::commands::validation::validate_id(id)?;
    
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    sqlx::query("UPDATE shows SET archived = 1 WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to archive show: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn unarchive_show(app: AppHandle, id: i64) -> Result<(), String> {
    crate::commands::validation::validate_id(id)?;
    
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    sqlx::query("UPDATE shows SET archived = 0 WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to unarchive show: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn update_show_rating(
    app: AppHandle,
    id: i64,
    rating: Option<f64>,
) -> Result<(), String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    sqlx::query(
        r#"
        UPDATE shows
        SET rating = ?, rank_order = NULL
        WHERE id = ?
        "#,
    )
    .bind(rating)
    .bind(id)
    .execute(&pool)
    .await
    .map_err(|e| format!("Failed to update show rating: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn reorder_show_in_tier(
    app: AppHandle,
    id: i64,
    new_rank_order: i32,
) -> Result<(), String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    sqlx::query("UPDATE shows SET rank_order = ? WHERE id = ?")
        .bind(new_rank_order)
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to reorder show: {}", e))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Quarantine resolution (rows left over from the TVDB->TMDB live remap)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct UnmigratedShow {
    pub id: i64,
    pub legacy_tvdb_id: Option<i64>,
    pub name: String,
    pub poster_url: Option<String>,
    pub first_aired: Option<String>,
}

#[tauri::command]
pub async fn get_unmigrated_shows(app: AppHandle) -> Result<Vec<UnmigratedShow>, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let rows = sqlx::query(
        "SELECT id, legacy_tvdb_id, name, poster_url, first_aired \
         FROM shows WHERE unmigrated = 1 ORDER BY name",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to load unmigrated shows: {}", e))?;

    let shows = rows
        .into_iter()
        .map(|row| UnmigratedShow {
            id: row.get("id"),
            legacy_tvdb_id: row.get("legacy_tvdb_id"),
            name: row.get("name"),
            poster_url: row.get("poster_url"),
            first_aired: row.get("first_aired"),
        })
        .collect();

    Ok(shows)
}

#[tauri::command]
pub async fn resolve_unmigrated_show(
    app: AppHandle,
    old_id: i64,
    new_tmdb_id: i64,
) -> Result<tvdb_remap::RemapOutcome, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    tvdb_remap::remap_single_show(&pool, old_id, new_tmdb_id).await
}

#[tauri::command]
pub async fn delete_unmigrated_show(app: AppHandle, id: i64) -> Result<(), String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("begin: {}", e))?;

    sqlx::query("PRAGMA defer_foreign_keys = ON")
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("defer_foreign_keys: {}", e))?;

    // Cascade-delete dangling history rows the FK cascade doesn't cover.
    sqlx::query(
        "DELETE FROM change_history \
         WHERE entity_type = 'episode' \
         AND entity_id IN (SELECT id FROM episodes WHERE show_id = ?)",
    )
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("history (episodes) cleanup: {}", e))?;

    sqlx::query(
        "DELETE FROM change_history WHERE entity_type = 'show' AND entity_id = ?",
    )
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("history (show) cleanup: {}", e))?;

    sqlx::query(
        "DELETE FROM title_mappings WHERE media_type = 'show' AND tvc_id = ?",
    )
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("title_mappings cleanup: {}", e))?;

    sqlx::query(
        "DELETE FROM plex_scrobble_log \
         WHERE matched_entity_type = 'show' AND matched_entity_id = ?",
    )
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("plex log cleanup: {}", e))?;

    sqlx::query("DELETE FROM sonarr_imports WHERE show_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("sonarr_imports cleanup: {}", e))?;

    sqlx::query("DELETE FROM shows WHERE id = ? AND unmigrated = 1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("delete show: {}", e))?;

    tx.commit().await.map_err(|e| format!("commit: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn search_tmdb_tv(query: String) -> Result<Vec<tmdb::TvShowSearchResult>, String> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    tmdb::search_tv(&query).await.map_err(|e| e.to_string())
}

/// Dev-only: re-run the TVDB->TMDB migration and seed N synthetic quarantine
/// rows so the resolver UI can be exercised on demand. Negative legacy ids
/// guarantee /find returns None, so the rows land in the quarantine bucket.
#[tauri::command]
pub async fn dev_force_rerun_migration(
    app: AppHandle,
    fake_quarantine_count: i64,
) -> Result<i64, String> {
    if !cfg!(debug_assertions) {
        return Err("Only available in dev builds".to_string());
    }

    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let mut tx = pool.begin().await.map_err(|e| format!("begin: {}", e))?;

    sqlx::query(
        "UPDATE shows SET unmigrated = 1 WHERE id > 0 AND legacy_tvdb_id IS NOT NULL",
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("reset shows: {}", e))?;

    for i in 0..fake_quarantine_count {
        let fake_id = 2_000_000_000 + i;
        let fake_tvdb = -1 - i;
        sqlx::query(
            "INSERT OR REPLACE INTO shows (id, name, status, unmigrated, legacy_tvdb_id) \
             VALUES (?, ?, 'Unknown', 1, ?)",
        )
        .bind(fake_id)
        .bind(format!("[Dev] Fake Quarantine #{}", i + 1))
        .bind(fake_tvdb)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("insert fake: {}", e))?;
    }

    sqlx::query(
        "INSERT OR REPLACE INTO settings (key, value) \
         VALUES ('tvdb_to_tmdb_migration_complete', '0')",
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("flag: {}", e))?;

    tx.commit().await.map_err(|e| format!("commit: {}", e))?;

    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        crate::db::tvdb_remap::run_migration_if_needed(app_clone).await;
    });

    Ok(fake_quarantine_count)
}

