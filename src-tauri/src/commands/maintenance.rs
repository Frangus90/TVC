use crate::db::connection;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use tauri::AppHandle;

// Only unused records are candidates. History can still reference a legacy TVDB
// episode ID after remapping, so both identities must be checked.
const PERSONAL_DATA_FREE: &str = "COALESCE(e.watched, 0) = 0
    AND trim(COALESCE(e.watched_at, '')) = ''
    AND trim(COALESCE(e.scheduled_date, '')) = ''
    AND e.rating IS NULL
    AND trim(COALESCE(e.tags, '')) IN ('', '[]')
    AND NOT EXISTS (SELECT 1 FROM change_history h WHERE h.entity_type = 'episode'
        AND (h.entity_id = e.id OR h.entity_id = e.legacy_tvdb_id))
    AND NOT EXISTS (SELECT 1 FROM plex_scrobble_log p WHERE p.matched_entity_type = 'episode'
        AND (p.matched_entity_id = e.id OR p.matched_entity_id = e.legacy_tvdb_id))";
const ORPHANED_OWNER: &str = "NOT EXISTS (SELECT 1 FROM shows s WHERE s.id = e.show_id)
    AND NOT EXISTS (SELECT 1 FROM shows s WHERE s.legacy_tvdb_id = e.show_id AND COALESCE(s.unmigrated, 0) != 0)";
const UNDATED_OWNER: &str = "trim(COALESCE(e.aired, '')) = ''
    AND EXISTS (SELECT 1 FROM shows s WHERE s.id = e.show_id
        AND COALESCE(s.tier_only, 0) = 0 AND COALESCE(s.unmigrated, 0) = 0)";

#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub total_shows: i64,
    pub total_episodes: i64,
    pub total_movies: i64,
    pub orphaned_episodes: i64,
    pub unaired_unscheduled_episodes: i64,
    pub database_size_bytes: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CleanupResult {
    pub orphaned_episodes_removed: i64,
    pub unaired_episodes_removed: i64,
    pub optimization_error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CleanupEpisodePreview {
    pub id: i64,
    pub show_name: String,
    pub season_number: i32,
    pub episode_number: i32,
    pub name: Option<String>,
}

fn predicate(orphaned: bool) -> String {
    format!(
        "{} AND {}",
        PERSONAL_DATA_FREE,
        if orphaned {
            ORPHANED_OWNER
        } else {
            UNDATED_OWNER
        }
    )
}

fn delete_statement(orphaned: bool) -> String {
    format!(
        "DELETE FROM episodes WHERE id IN (SELECT e.id FROM episodes e WHERE {})",
        predicate(orphaned)
    )
}

async fn candidate_count(pool: &SqlitePool, orphaned: bool) -> Result<i64, String> {
    sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM episodes e WHERE {}",
        predicate(orphaned)
    ))
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Could not count cleanup candidates: {}", e))
}

async fn database_stats(pool: &SqlitePool) -> Result<DatabaseStats, String> {
    let total_shows = sqlx::query_scalar("SELECT COUNT(*) FROM shows")
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Could not count shows: {}", e))?;
    let total_episodes = sqlx::query_scalar("SELECT COUNT(*) FROM episodes")
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Could not count episodes: {}", e))?;
    let total_movies = sqlx::query_scalar("SELECT COUNT(*) FROM movies")
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Could not count movies: {}", e))?;
    let database_size_bytes = sqlx::query_scalar(
        "SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Could not read database size: {}", e))?;
    Ok(DatabaseStats {
        total_shows,
        total_episodes,
        total_movies,
        orphaned_episodes: candidate_count(pool, true).await?,
        unaired_unscheduled_episodes: candidate_count(pool, false).await?,
        database_size_bytes,
    })
}

#[tauri::command]
pub async fn get_database_stats(app: AppHandle) -> Result<DatabaseStats, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;
    database_stats(&pool).await
}

#[tauri::command]
pub async fn cleanup_orphaned_episodes(app: AppHandle) -> Result<i64, String> {
    let _guard = crate::library_sync::lock_for_maintenance()?;
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;
    sqlx::query(&delete_statement(true))
        .execute(&pool)
        .await
        .map(|result| result.rows_affected() as i64)
        .map_err(|e| format!("Could not remove unused orphaned episodes: {}", e))
}

#[tauri::command]
pub async fn cleanup_unaired_episodes(app: AppHandle) -> Result<i64, String> {
    let _guard = crate::library_sync::lock_for_maintenance()?;
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;
    sqlx::query(&delete_statement(false))
        .execute(&pool)
        .await
        .map(|result| result.rows_affected() as i64)
        .map_err(|e| format!("Could not remove unused episodes without dates: {}", e))
}

async fn preview(pool: &SqlitePool, orphaned: bool) -> Result<Vec<CleanupEpisodePreview>, String> {
    let rows = sqlx::query(&format!("SELECT e.id, COALESCE(s.name, 'Unknown Show (ID: ' || e.show_id || ')') AS show_name,
        COALESCE(e.season_number, 0) AS season_number, COALESCE(e.episode_number, 0) AS episode_number, e.name
        FROM episodes e LEFT JOIN shows s ON s.id = e.show_id
        WHERE {} ORDER BY e.show_id, e.season_number, e.episode_number, e.id LIMIT 50", predicate(orphaned)))
        .fetch_all(pool).await.map_err(|e| format!("Could not load cleanup preview: {}", e))?;
    Ok(rows
        .into_iter()
        .map(|row| CleanupEpisodePreview {
            id: row.get("id"),
            show_name: row.get("show_name"),
            season_number: row.get("season_number"),
            episode_number: row.get("episode_number"),
            name: row.get("name"),
        })
        .collect())
}

#[tauri::command]
pub async fn get_orphaned_episodes_preview(
    app: AppHandle,
) -> Result<Vec<CleanupEpisodePreview>, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;
    preview(&pool, true).await
}

#[tauri::command]
pub async fn get_unaired_episodes_preview(
    app: AppHandle,
) -> Result<Vec<CleanupEpisodePreview>, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;
    preview(&pool, false).await
}

#[tauri::command]
pub async fn optimize_database(app: AppHandle) -> Result<(), String> {
    let _guard = crate::library_sync::lock_for_maintenance()?;
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;
    sqlx::query("VACUUM")
        .execute(&pool)
        .await
        .map_err(|e| format!("Could not optimize database: {}", e))?;
    sqlx::query("ANALYZE")
        .execute(&pool)
        .await
        .map_err(|e| format!("Storage was optimized, but query analysis failed: {}", e))?;
    Ok(())
}

async fn cleanup_records(pool: &SqlitePool) -> Result<CleanupResult, String> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Could not start cleanup: {}", e))?;
    let orphaned = sqlx::query(&delete_statement(true))
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Could not remove orphaned episodes: {}", e))?
        .rows_affected() as i64;
    let unaired = sqlx::query(&delete_statement(false))
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Could not remove episodes without dates: {}", e))?
        .rows_affected() as i64;
    tx.commit()
        .await
        .map_err(|e| format!("Could not commit cleanup: {}", e))?;
    Ok(CleanupResult {
        orphaned_episodes_removed: orphaned,
        unaired_episodes_removed: unaired,
        optimization_error: None,
    })
}

fn optimization_outcome(
    mut cleanup: CleanupResult,
    optimization: Result<(), String>,
) -> CleanupResult {
    cleanup.optimization_error = optimization.err();
    cleanup
}

#[tauri::command]
pub async fn run_full_cleanup(app: AppHandle) -> Result<CleanupResult, String> {
    let _guard = crate::library_sync::lock_for_maintenance()?;
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;
    let cleanup = cleanup_records(&pool).await?;
    let optimization = sqlx::query("VACUUM")
        .execute(&pool)
        .await
        .map(|_| ())
        .map_err(|e| {
            format!(
                "Episodes were removed, but storage optimization failed: {}",
                e
            )
        });
    Ok(optimization_outcome(cleanup, optimization))
}

#[cfg(test)]
mod tests {
    use super::{candidate_count, cleanup_records, database_stats, optimization_outcome, preview};
    use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};

    async fn fixture() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::raw_sql("CREATE TABLE shows(id INTEGER PRIMARY KEY,name TEXT,tier_only INTEGER,unmigrated INTEGER,legacy_tvdb_id INTEGER);
            CREATE TABLE episodes(id INTEGER PRIMARY KEY,show_id INTEGER,season_number INTEGER DEFAULT 1,episode_number INTEGER DEFAULT 1,name TEXT,aired TEXT,scheduled_date TEXT,watched INTEGER DEFAULT 0,watched_at TEXT,rating INTEGER,tags TEXT,legacy_tvdb_id INTEGER);
            CREATE TABLE movies(id INTEGER PRIMARY KEY);
            CREATE TABLE change_history(entity_type TEXT,entity_id INTEGER);
            CREATE TABLE plex_scrobble_log(matched_entity_type TEXT,matched_entity_id INTEGER);
            INSERT INTO shows VALUES(1,'Tracked',0,0,NULL),(2,'Tier only',1,0,NULL),(3,'Unresolved',0,1,3),(400,'Legacy owner',0,1,404);
            INSERT INTO episodes(id,show_id,aired) VALUES(1,999,'2000-01-01'),(2,1,NULL),(3,1,'2026-12-01');
            INSERT INTO episodes(id,show_id,tags,watched_at,scheduled_date) VALUES(4,1,'[]','','');
            INSERT INTO episodes(id,show_id,watched) VALUES(10,999,1),(20,1,1);
            INSERT INTO episodes(id,show_id,watched_at) VALUES(11,999,'2026-01-01'),(21,1,'2026-01-01');
            INSERT INTO episodes(id,show_id,scheduled_date) VALUES(12,999,'2026-01-01'),(22,1,'2026-01-01');
            INSERT INTO episodes(id,show_id,rating) VALUES(13,999,0),(23,1,3);
            INSERT INTO episodes(id,show_id,tags) VALUES(14,999,'[\"favourite\"]'),(24,1,'[\"favourite\"]');
            INSERT INTO episodes(id,show_id) VALUES(15,999),(25,1);
            INSERT INTO episodes(id,show_id,legacy_tvdb_id) VALUES(16,999,916),(26,1,926),(18,999,918);
            INSERT INTO episodes(id,show_id) VALUES(17,999),(30,2),(31,3),(32,404);
            INSERT INTO change_history VALUES('episode',15),('episode',25),('episode',916),('episode',926);
            INSERT INTO plex_scrobble_log VALUES('episode',17),('episode',918);").execute(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn counts_previews_and_deletes_agree_and_preserve_personal_or_legacy_records() {
        let pool = fixture().await;
        let stats = database_stats(&pool).await.unwrap();
        assert_eq!(
            (stats.orphaned_episodes, stats.unaired_unscheduled_episodes),
            (1, 2)
        );
        assert_eq!(
            preview(&pool, true)
                .await
                .unwrap()
                .into_iter()
                .map(|row| row.id)
                .collect::<Vec<_>>(),
            vec![1]
        );
        assert_eq!(
            preview(&pool, false)
                .await
                .unwrap()
                .into_iter()
                .map(|row| row.id)
                .collect::<Vec<_>>(),
            vec![2, 4]
        );
        let before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM episodes")
            .fetch_one(&pool)
            .await
            .unwrap();
        let result = cleanup_records(&pool).await.unwrap();
        assert_eq!(
            (
                result.orphaned_episodes_removed,
                result.unaired_episodes_removed
            ),
            (1, 2)
        );
        let after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM episodes")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(before - after, 3);
        assert_eq!(candidate_count(&pool, true).await.unwrap(), 0);
        assert_eq!(candidate_count(&pool, false).await.unwrap(), 0);
        for id in [
            3, 10, 11, 12, 13, 14, 15, 16, 17, 18, 20, 21, 22, 23, 24, 25, 26, 30, 31, 32,
        ] {
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM episodes WHERE id=?")
                    .bind(id)
                    .fetch_one(&pool)
                    .await
                    .unwrap(),
                1,
                "protected episode {id}"
            );
        }
    }

    #[tokio::test]
    async fn capped_preview_does_not_understate_total_cleanup_candidates() {
        let pool = fixture().await;
        for id in 1000..1070 {
            sqlx::query("INSERT INTO episodes(id,show_id) VALUES(?,999)")
                .bind(id)
                .execute(&pool)
                .await
                .unwrap();
        }
        assert_eq!(preview(&pool, true).await.unwrap().len(), 50);
        assert_eq!(database_stats(&pool).await.unwrap().orphaned_episodes, 71);
        assert_eq!(
            cleanup_records(&pool)
                .await
                .unwrap()
                .orphaned_episodes_removed,
            71
        );
    }

    #[tokio::test]
    async fn second_cleanup_failure_rolls_back_first_deletion() {
        let pool = fixture().await;
        sqlx::raw_sql("CREATE TRIGGER reject_delete BEFORE DELETE ON episodes WHEN OLD.id=2 BEGIN SELECT RAISE(ABORT,'fixture failure'); END;").execute(&pool).await.unwrap();
        assert!(cleanup_records(&pool).await.is_err());
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM episodes WHERE id=1")
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );
        assert_eq!(candidate_count(&pool, false).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn optimization_failure_reports_committed_cleanup_counts() {
        let pool = fixture().await;
        let committed = cleanup_records(&pool).await.unwrap();
        let result = optimization_outcome(committed, Err("fixture VACUUM failure".into()));
        assert_eq!(
            (
                result.orphaned_episodes_removed,
                result.unaired_episodes_removed
            ),
            (1, 2)
        );
        assert_eq!(
            result.optimization_error.as_deref(),
            Some("fixture VACUUM failure")
        );
        assert_eq!(candidate_count(&pool, true).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn failed_statistics_or_preview_queries_are_errors_not_zero_results() {
        let pool = fixture().await;
        sqlx::query("DROP TABLE movies")
            .execute(&pool)
            .await
            .unwrap();
        assert!(database_stats(&pool)
            .await
            .unwrap_err()
            .contains("count movies"));
        sqlx::query("DROP TABLE change_history")
            .execute(&pool)
            .await
            .unwrap();
        assert!(preview(&pool, false).await.is_err());
    }
}
