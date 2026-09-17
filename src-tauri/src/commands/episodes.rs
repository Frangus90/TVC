use crate::db::connection;
use crate::tmdb;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row};
use std::collections::HashMap;
use tauri::AppHandle;

/// User-controlled per-episode state. Preserved across re-syncs by keying on
/// (season, episode) — TMDB episode IDs can change when the show is updated.
#[derive(Debug, Clone, Default)]
struct PreservedEpisodeState {
    watched: bool,
    watched_at: Option<String>,
    scheduled_date: Option<String>,
    rating: Option<f64>,
    tags: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Episode {
    pub id: i64,
    pub show_id: i64,
    #[sqlx(rename = "show_name")]
    pub show_name: String,
    pub network: Option<String>,
    pub season_number: i32,
    pub episode_number: i32,
    pub name: Option<String>,
    pub aired: Option<String>,
    pub scheduled_date: Option<String>,
    pub watched: bool,
    pub poster_url: Option<String>,
}

#[tauri::command]
pub async fn mark_episode_watched(
    app: AppHandle,
    episode_id: i64,
    watched: bool,
) -> Result<(), String> {
    crate::commands::validation::validate_id(episode_id)?;

    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    crate::watch_history::episodes(
        &pool,
        crate::watch_history::Episodes::One(episode_id),
        watched,
    )
    .await
}

#[tauri::command]
pub async fn get_episodes_for_range(
    app: AppHandle,
    start_date: String,
    end_date: String,
) -> Result<Vec<Episode>, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    episodes_for_range(&pool, &start_date, &end_date).await
}

pub(crate) async fn episodes_for_range(
    pool: &sqlx::SqlitePool,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<Episode>, String> {
    super::validation::validate_date(start_date)?;
    super::validation::validate_date(end_date)?;
    if start_date > end_date {
        return Err("Start date must not follow end date".into());
    }
    let rows = sqlx::query(
        r#"
        SELECT
            e.id,
            e.show_id,
            s.name as show_name,
            s.network,
            COALESCE(e.season_number, 0) as season_number,
            COALESCE(e.episode_number, 0) as episode_number,
            e.name,
            e.aired,
            e.scheduled_date,
            e.watched,
            s.poster_url
        FROM episodes e
        JOIN shows s ON e.show_id = s.id
        WHERE s.tier_only = 0 AND COALESCE(s.archived, 0) = 0
          AND COALESCE(e.scheduled_date, e.aired) >= ?
          AND COALESCE(e.scheduled_date, e.aired) <= ?
        ORDER BY COALESCE(e.scheduled_date, e.aired), s.name
        LIMIT 10000
        "#,
    )
    .bind(&start_date)
    .bind(&end_date)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to get episodes for range: {}", e))?;

    let episodes: Vec<Episode> = rows
        .into_iter()
        .map(|row| Episode {
            id: row.get("id"),
            show_id: row.get("show_id"),
            show_name: row.get("show_name"),
            network: row.get("network"),
            season_number: row.get("season_number"),
            episode_number: row.get("episode_number"),
            name: row.get("name"),
            aired: row.get("aired"),
            scheduled_date: row.get("scheduled_date"),
            watched: row.get::<i32, _>("watched") == 1,
            poster_url: row.get("poster_url"),
        })
        .collect();

    Ok(episodes)
}

#[tauri::command]
pub async fn sync_show_episodes(app: AppHandle, show_id: i64) -> Result<(), String> {
    crate::commands::ensure_show_is_mapped(&app, show_id).await?;

    tmdb::invalidate_tv_show_cache(show_id).await;

    let show_details = tmdb::get_tv_details(show_id)
        .await
        .map_err(|e| format!("Failed to fetch show details: {}", e))?;

    let episodes = tmdb::get_tv_episodes(show_id)
        .await
        .map_err(|e| format!("Failed to fetch episodes: {}", e))?;

    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    apply_show_refresh(&pool, &show_details, episodes).await
}

async fn apply_show_refresh(
    pool: &sqlx::SqlitePool,
    show_details: &tmdb::TvShowDetails,
    episodes: Vec<tmdb::TvEpisode>,
) -> Result<(), String> {
    let show_id = show_details.id;
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    let preserved = snapshot_episode_state(&mut tx, show_id).await?;
    if episodes.is_empty() && !preserved.is_empty() {
        return Err("TMDB returned no episodes; the saved episodes were kept".into());
    }

    if let Err(e) = sqlx::query(
        r#"
        UPDATE shows SET
            name = ?,
            status = ?,
            poster_url = ?,
            first_aired = ?,
            network = ?,
            overview = ?,
            runtime = ?,
            last_synced = datetime('now')
        WHERE id = ?
        "#,
    )
    .bind(&show_details.name)
    .bind(show_details.status.as_ref())
    .bind(show_details.poster_url())
    .bind(show_details.first_air_date.as_ref())
    .bind(show_details.network_name())
    .bind(show_details.overview.as_ref())
    .bind(show_details.runtime())
    .bind(show_id)
    .execute(&mut *tx)
    .await
    {
        let _ = tx.rollback().await;
        return Err(format!("Failed to update show: {}", e));
    }

    // Remove disposable cached rows, retaining unmatched episodes with personal state.
    // Returned episodes are replaced by season/episode below, even if provider IDs change.
    if let Err(e) = sqlx::query("DELETE FROM episodes WHERE show_id = ? AND COALESCE(watched,0) = 0 AND watched_at IS NULL AND scheduled_date IS NULL AND rating IS NULL AND COALESCE(tags, '') IN ('', '[]')")
        .bind(show_id)
        .execute(&mut *tx)
        .await
    {
        let _ = tx.rollback().await;
        return Err(format!("Failed to clear old episodes: {}", e));
    }

    let mut keys = std::collections::HashSet::new();
    for ep in episodes {
        if !keys.insert((ep.season_number, ep.episode_number)) {
            return Err("TMDB returned a duplicate season/episode; saved data was kept".into());
        }
        sqlx::query(
            "DELETE FROM episodes WHERE show_id = ? AND season_number = ? AND episode_number = ?",
        )
        .bind(show_id)
        .bind(ep.season_number)
        .bind(ep.episode_number)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Could not replace episode: {e}"))?;
        let scheduled_date = match preserved.get(&(ep.season_number, ep.episode_number)) {
            Some(state) => state.scheduled_date.clone(),
            None => None,
        };
        let state = preserved
            .get(&(ep.season_number, ep.episode_number))
            .cloned()
            .unwrap_or_default();

        if let Err(e) = sqlx::query(
            r#"
            INSERT INTO episodes
            (id, show_id, season_number, episode_number, name, overview, aired,
             runtime, image_url, scheduled_date, watched, watched_at, rating, tags)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(ep.id)
        .bind(show_id)
        .bind(ep.season_number)
        .bind(ep.episode_number)
        .bind(ep.name.as_ref())
        .bind(ep.overview.as_ref())
        .bind(ep.air_date.as_ref())
        .bind(ep.runtime)
        .bind(ep.image_url())
        .bind(scheduled_date)
        .bind(if state.watched { 1 } else { 0 })
        .bind(state.watched_at)
        .bind(state.rating)
        .bind(state.tags)
        .execute(&mut *tx)
        .await
        {
            let _ = tx.rollback().await;
            return Err(format!("Failed to sync episode: {}", e));
        }
    }

    tx.commit()
        .await
        .map_err(|e| format!("Failed to commit transaction: {}", e))?;

    Ok(())
}

async fn snapshot_episode_state(
    connection: &mut sqlx::SqliteConnection,
    show_id: i64,
) -> Result<HashMap<(i32, i32), PreservedEpisodeState>, String> {
    let rows = sqlx::query(
        r#"
        SELECT season_number, episode_number, watched, watched_at,
               scheduled_date, rating, tags
        FROM episodes
        WHERE show_id = ?
        "#,
    )
    .bind(show_id)
    .fetch_all(connection)
    .await
    .map_err(|e| format!("Failed to snapshot episode state: {}", e))?;

    let mut map = HashMap::new();
    for row in rows {
        let key = (
            row.get::<i32, _>("season_number"),
            row.get::<i32, _>("episode_number"),
        );
        map.insert(
            key,
            PreservedEpisodeState {
                watched: row.get::<i32, _>("watched") == 1,
                watched_at: row.get("watched_at"),
                scheduled_date: row.get("scheduled_date"),
                rating: row.get("rating"),
                tags: row.get("tags"),
            },
        );
    }
    Ok(map)
}

#[cfg(test)]
mod sync_tests {
    use super::*;
    use serde_json::json;

    async fn database() -> sqlx::SqlitePool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        for migration in [
            include_str!("../../migrations/001_initial.sql"),
            include_str!("../../migrations/002_add_indexes.sql"),
            include_str!("../../migrations/004_add_episode_metadata.sql"),
        ] {
            sqlx::raw_sql(migration).execute(&pool).await.unwrap();
        }
        sqlx::raw_sql("INSERT INTO shows (id, name) VALUES (1, 'Original');
            INSERT INTO episodes (id, show_id, season_number, episode_number, watched, watched_at, scheduled_date, rating, tags)
            VALUES (10, 1, 1, 1, 1, '2026-09-15T20:00:00Z', '2026-09-14', 8.5, '[\"favorite\"]'),
                   (11, 1, 1, 2, 0, NULL, NULL, NULL, NULL);")
            .execute(&pool).await.unwrap();
        pool
    }

    fn details() -> tmdb::TvShowDetails {
        serde_json::from_value(json!({"id": 1, "name": "Updated"})).unwrap()
    }

    fn episode(id: i64, number: i32) -> tmdb::TvEpisode {
        serde_json::from_value(json!({"id": id, "season_number": 1, "episode_number": number, "air_date": "2026-09-16"})).unwrap()
    }

    #[tokio::test]
    async fn refresh_preserves_user_state_when_provider_ids_change() {
        let pool = database().await;
        apply_show_refresh(
            &pool,
            &details(),
            vec![episode(100, 1), episode(101, 2), episode(102, 3)],
        )
        .await
        .unwrap();
        let row = sqlx::query("SELECT * FROM episodes WHERE id = 100")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row.get::<i32, _>("watched"), 1);
        assert_eq!(row.get::<String, _>("watched_at"), "2026-09-15T20:00:00Z");
        assert_eq!(row.get::<String, _>("scheduled_date"), "2026-09-14");
        assert_eq!(row.get::<f64, _>("rating"), 8.5);
        assert_eq!(row.get::<String, _>("tags"), "[\"favorite\"]");
        let dates: Vec<Option<String>> =
            sqlx::query_scalar("SELECT scheduled_date FROM episodes ORDER BY episode_number")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(dates, vec![Some("2026-09-14".into()), None, None]);
        let name: String = sqlx::query_scalar("SELECT name FROM shows WHERE id = 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(name, "Updated");
    }

    #[tokio::test]
    async fn provider_removed_episode_with_personal_state_survives_refresh() {
        let pool = database().await;
        apply_show_refresh(&pool, &details(), vec![episode(101, 2)])
            .await
            .unwrap();
        let row =
            sqlx::query("SELECT watched_at, scheduled_date, rating FROM episodes WHERE id=10")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(row.get::<String, _>("watched_at"), "2026-09-15T20:00:00Z");
        assert_eq!(row.get::<f64, _>("rating"), 8.5);
    }

    #[tokio::test]
    async fn empty_response_and_write_failure_leave_saved_library_unchanged() {
        let pool = database().await;
        assert!(apply_show_refresh(&pool, &details(), vec![])
            .await
            .unwrap_err()
            .contains("saved episodes were kept"));
        // Duplicate IDs fail after deleting the old rows and inserting the first new row.
        // The transaction must restore both the original metadata and watched episodes.
        assert!(
            apply_show_refresh(&pool, &details(), vec![episode(100, 1), episode(100, 2)])
                .await
                .is_err()
        );
        let ids: Vec<i64> = sqlx::query_scalar("SELECT id FROM episodes ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert_eq!(ids, vec![10, 11]);
        let name: String = sqlx::query_scalar("SELECT name FROM shows WHERE id = 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(name, "Original");
        let watched: i32 = sqlx::query_scalar("SELECT watched FROM episodes WHERE id = 10")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(watched, 1);
    }
}

#[tauri::command]
pub async fn schedule_episode(app: AppHandle, episode_id: i64, date: String) -> Result<(), String> {
    crate::commands::validation::validate_id(episode_id)?;
    crate::commands::validation::validate_date(&date)?;

    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    sqlx::query("UPDATE episodes SET scheduled_date = ? WHERE id = ?")
        .bind(&date)
        .bind(episode_id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to schedule episode: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn unschedule_episode(app: AppHandle, episode_id: i64) -> Result<(), String> {
    crate::commands::validation::validate_id(episode_id)?;

    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    sqlx::query("UPDATE episodes SET scheduled_date = NULL WHERE id = ?")
        .bind(episode_id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to unschedule episode: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn mark_season_watched(
    app: AppHandle,
    show_id: i64,
    season_number: i32,
    watched: bool,
) -> Result<(), String> {
    crate::commands::validation::validate_id(show_id)?;

    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    crate::watch_history::episodes(
        &pool,
        crate::watch_history::Episodes::Season(show_id, season_number),
        watched,
    )
    .await
}

#[tauri::command]
pub async fn mark_show_watched(app: AppHandle, show_id: i64, watched: bool) -> Result<(), String> {
    crate::commands::validation::validate_id(show_id)?;

    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    crate::watch_history::episodes(
        &pool,
        crate::watch_history::Episodes::Show(show_id),
        watched,
    )
    .await
}
