use crate::db::connection;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use tauri::AppHandle;

#[derive(Debug, Serialize, Deserialize)]
pub struct WatchStatistics {
    pub total_watch_time_minutes: i64,
    pub episodes_watched: i64,
    pub movies_watched: i64,
    pub shows_completed: i64,
    pub shows_in_progress: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PeriodStats {
    pub date: String,
    pub episodes_count: i64,
    pub movies_count: i64,
    pub total_runtime: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShowCompletion {
    pub show_id: i64,
    pub show_name: String,
    pub poster_url: Option<String>,
    pub total_episodes: i64,
    pub watched_episodes: i64,
    pub completion_percentage: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WatchHistoryItem {
    pub source: String,
    pub item_type: String,
    pub id: i64,
    pub detail_id: Option<i64>,
    pub name: String,
    pub show_name: Option<String>,
    pub season_number: Option<i32>,
    pub episode_number: Option<i32>,
    pub year: Option<i32>,
    pub watched_at: String,
    pub poster_url: Option<String>,
    pub runtime: Option<i32>,
}

#[tauri::command]
pub async fn get_watch_statistics(app: AppHandle) -> Result<WatchStatistics, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    // Get total watch time from episodes
    let episode_time: i64 =
        sqlx::query(r#"SELECT COALESCE(SUM(runtime), 0) as total FROM episodes WHERE watched = 1"#)
            .fetch_one(&pool)
            .await
            .map(|row| row.get("total"))
            .unwrap_or(0);

    // Get total watch time from movies
    let movie_time: i64 =
        sqlx::query(r#"SELECT COALESCE(SUM(runtime), 0) as total FROM movies WHERE watched = 1"#)
            .fetch_one(&pool)
            .await
            .map(|row| row.get("total"))
            .unwrap_or(0);

    // Get episodes watched count
    let episodes_watched: i64 =
        sqlx::query(r#"SELECT COUNT(*) as count FROM episodes WHERE watched = 1"#)
            .fetch_one(&pool)
            .await
            .map(|row| row.get("count"))
            .unwrap_or(0);

    // Get movies watched count
    let movies_watched: i64 =
        sqlx::query(r#"SELECT COUNT(*) as count FROM movies WHERE watched = 1"#)
            .fetch_one(&pool)
            .await
            .map(|row| row.get("count"))
            .unwrap_or(0);

    // Get shows completed (all episodes watched)
    let shows_completed: i64 = sqlx::query(
        r#"
        SELECT COUNT(*) as count FROM (
            SELECT show_id
            FROM episodes
            GROUP BY show_id
            HAVING COUNT(*) = SUM(watched)
        )
        "#,
    )
    .fetch_one(&pool)
    .await
    .map(|row| row.get("count"))
    .unwrap_or(0);

    // Get shows in progress (some but not all episodes watched)
    let shows_in_progress: i64 = sqlx::query(
        r#"
        SELECT COUNT(*) as count FROM (
            SELECT show_id
            FROM episodes
            GROUP BY show_id
            HAVING SUM(watched) > 0 AND SUM(watched) < COUNT(*)
        )
        "#,
    )
    .fetch_one(&pool)
    .await
    .map(|row| row.get("count"))
    .unwrap_or(0);

    Ok(WatchStatistics {
        total_watch_time_minutes: episode_time + movie_time,
        episodes_watched,
        movies_watched,
        shows_completed,
        shows_in_progress,
    })
}

#[tauri::command]
pub async fn get_episodes_watched_by_period(
    app: AppHandle,
    start_date: String,
    end_date: String,
    group_by: Option<String>,
) -> Result<Vec<PeriodStats>, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {e}"))?;
    period_stats(&pool, &start_date, &end_date, group_by.as_deref()).await
}

pub(crate) async fn period_stats(
    pool: &sqlx::SqlitePool,
    start_date: &str,
    end_date: &str,
    group_by: Option<&str>,
) -> Result<Vec<PeriodStats>, String> {
    super::validation::validate_date(start_date)?;
    super::validation::validate_date(end_date)?;
    if start_date > end_date {
        return Err("Start date must not follow end date".into());
    }
    let date_format = match group_by {
        Some("month") => "%Y-%m",
        Some("year") => "%Y",
        _ => "%Y-%m-%d", // day is default
    };

    // SQLite doesn't support FULL OUTER JOIN, so we emulate it with UNION
    let rows = sqlx::query(&format!(
        r#"
        WITH episode_stats AS (
            SELECT
                strftime('{}', watched_at, 'localtime') as period,
                COUNT(*) as ep_count,
                COALESCE(SUM(runtime), 0) as ep_runtime
            FROM episodes
            WHERE watched = 1
              AND watched_at IS NOT NULL
              AND date(watched_at, 'localtime') >= ?
              AND date(watched_at, 'localtime') <= ?
            GROUP BY period
        ),
        movie_stats AS (
            SELECT
                strftime('{}', watched_at, 'localtime') as period,
                COUNT(*) as mv_count,
                COALESCE(SUM(runtime), 0) as mv_runtime
            FROM movies
            WHERE watched = 1
              AND watched_at IS NOT NULL
              AND date(watched_at, 'localtime') >= ?
              AND date(watched_at, 'localtime') <= ?
            GROUP BY period
        ),
        all_periods AS (
            SELECT period FROM episode_stats
            UNION
            SELECT period FROM movie_stats
        )
        SELECT
            p.period as date,
            COALESCE(e.ep_count, 0) as episodes_count,
            COALESCE(m.mv_count, 0) as movies_count,
            COALESCE(e.ep_runtime, 0) + COALESCE(m.mv_runtime, 0) as total_runtime
        FROM all_periods p
        LEFT JOIN episode_stats e ON p.period = e.period
        LEFT JOIN movie_stats m ON p.period = m.period
        ORDER BY date
        "#,
        date_format, date_format
    ))
    .bind(&start_date)
    .bind(&end_date)
    .bind(&start_date)
    .bind(&end_date)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to get period stats: {}", e))?;

    let stats: Vec<PeriodStats> = rows
        .into_iter()
        .map(|row| PeriodStats {
            date: row.get("date"),
            episodes_count: row.get("episodes_count"),
            movies_count: row.get("movies_count"),
            total_runtime: row.get("total_runtime"),
        })
        .collect();

    Ok(stats)
}

#[tauri::command]
pub async fn get_completion_rates(app: AppHandle) -> Result<Vec<ShowCompletion>, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let rows = sqlx::query(
        r#"
        SELECT
            s.id as show_id,
            s.name as show_name,
            s.poster_url,
            COUNT(e.id) as total_episodes,
            SUM(e.watched) as watched_episodes
        FROM shows s
        LEFT JOIN episodes e ON s.id = e.show_id
        WHERE s.archived = 0
        GROUP BY s.id
        ORDER BY s.name
        "#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to get completion rates: {}", e))?;

    let completions: Vec<ShowCompletion> = rows
        .into_iter()
        .map(|row| {
            let total: i64 = row.get("total_episodes");
            let watched: i64 = row.get("watched_episodes");
            let percentage = if total > 0 {
                (watched as f64 / total as f64) * 100.0
            } else {
                0.0
            };

            ShowCompletion {
                show_id: row.get("show_id"),
                show_name: row.get("show_name"),
                poster_url: row.get("poster_url"),
                total_episodes: total,
                watched_episodes: watched,
                completion_percentage: percentage,
            }
        })
        .collect();

    Ok(completions)
}

#[tauri::command]
pub async fn get_watch_history(
    app: AppHandle,
    limit: Option<i32>,
) -> Result<Vec<WatchHistoryItem>, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    get_watch_history_from_pool(&pool, limit.unwrap_or(50)).await
}

#[tauri::command]
pub async fn resolve_plex_history_item(
    app: AppHandle,
    history_id: i64,
    media_type: String,
    entity_id: i64,
) -> Result<(), String> {
    crate::commands::validation::validate_id(history_id)?;
    crate::commands::validation::validate_id(entity_id)?;

    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {e}"))?;
    resolve_plex_history_item_in_pool(&pool, history_id, &media_type, entity_id).await
}

async fn resolve_plex_history_item_in_pool(
    pool: &SqlitePool,
    history_id: i64,
    media_type: &str,
    entity_id: i64,
) -> Result<(), String> {
    let (entity_type, target_exists) = match media_type {
        "episode" => (
            "show",
            sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM shows WHERE id = ?)")
                .bind(entity_id)
                .fetch_one(pool)
                .await
                .map_err(|e| format!("Failed to check show: {e}"))?,
        ),
        "movie" => (
            "movie",
            sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM movies WHERE id = ?)")
                .bind(entity_id)
                .fetch_one(pool)
                .await
                .map_err(|e| format!("Failed to check movie: {e}"))?,
        ),
        _ => return Err("Unsupported Plex history media type".into()),
    };

    if !target_exists {
        return Err("The selected title is not saved in TVC".into());
    }

    let result = sqlx::query(
        "UPDATE plex_scrobble_log SET matched_entity_type = ?, matched_entity_id = ?, match_method = 'manual' WHERE id = ? AND event_type = 'media.scrobble' AND media_type = ? AND matched_entity_id IS NULL",
    )
    .bind(entity_type)
    .bind(entity_id)
    .bind(history_id)
    .bind(media_type)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to link Plex history item: {e}"))?;

    if result.rows_affected() == 0 {
        return Err("The Plex history item was not found or is already linked".into());
    }

    Ok(())
}

async fn get_watch_history_from_pool(
    pool: &SqlitePool,
    limit: i32,
) -> Result<Vec<WatchHistoryItem>, String> {
    let limit = limit.clamp(1, 500);

    let rows = sqlx::query(
        r#"
        WITH history AS (
            SELECT
                'library' AS source,
                'episode' AS item_type,
                e.id,
                s.id AS detail_id,
                e.name,
                s.name AS show_name,
                e.season_number,
                e.episode_number,
                NULL AS year,
                e.watched_at,
                s.poster_url,
                e.runtime
            FROM episodes e
            JOIN shows s ON e.show_id = s.id
            WHERE e.watched = 1 AND e.watched_at IS NOT NULL
              AND NOT EXISTS (
                  SELECT 1 FROM plex_scrobble_log p
                  WHERE (
                    (p.matched_entity_type = 'episode' AND p.matched_entity_id = e.id)
                    OR (p.matched_entity_type = 'show' AND p.matched_entity_id = s.id)
                  )
                    AND datetime(p.scrobbled_at) = datetime(e.watched_at)
              )

            UNION ALL

            SELECT
                'library' AS source,
                'movie' AS item_type,
                m.id,
                m.id AS detail_id,
                m.title AS name,
                NULL AS show_name,
                NULL AS season_number,
                NULL AS episode_number,
                NULL AS year,
                m.watched_at,
                m.poster_url,
                m.runtime
            FROM movies m
            WHERE m.watched = 1 AND m.watched_at IS NOT NULL
              AND NOT EXISTS (
                  SELECT 1 FROM plex_scrobble_log p
                  WHERE p.matched_entity_type = 'movie'
                    AND p.matched_entity_id = m.id
                    AND datetime(p.scrobbled_at) = datetime(m.watched_at)
              )

            UNION ALL

            SELECT
                'plex' AS source,
                'episode' AS item_type,
                p.id,
                s.id AS detail_id,
                p.raw_title AS name,
                COALESCE(p.show_name, s.name) AS show_name,
                p.season_number,
                p.episode_number,
                p.year,
                p.scrobbled_at AS watched_at,
                s.poster_url,
                e.runtime
            FROM plex_scrobble_log p
            LEFT JOIN episodes e
              ON p.matched_entity_type = 'episode' AND p.matched_entity_id = e.id
            LEFT JOIN shows s
              ON (p.matched_entity_type = 'show' AND p.matched_entity_id = s.id)
              OR (p.matched_entity_type = 'episode' AND e.show_id = s.id)
            WHERE p.event_type = 'media.scrobble' AND p.media_type = 'episode'

            UNION ALL

            SELECT
                'plex' AS source,
                'movie' AS item_type,
                p.id,
                m.id AS detail_id,
                COALESCE(m.title, p.raw_title) AS name,
                NULL AS show_name,
                NULL AS season_number,
                NULL AS episode_number,
                p.year,
                p.scrobbled_at AS watched_at,
                m.poster_url,
                m.runtime
            FROM plex_scrobble_log p
            LEFT JOIN movies m
              ON p.matched_entity_type = 'movie' AND p.matched_entity_id = m.id
            WHERE p.event_type = 'media.scrobble' AND p.media_type = 'movie'
        ), ranked AS (
            SELECT history.*,
                   ROW_NUMBER() OVER (
                       PARTITION BY item_type
                       ORDER BY julianday(watched_at) DESC, id DESC
                   ) AS item_rank
            FROM history
        )
        SELECT * FROM ranked
        WHERE item_rank <= ?
        ORDER BY julianday(watched_at) DESC, id DESC
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to get watch history: {}", e))?;

    let history: Vec<WatchHistoryItem> = rows
        .into_iter()
        .map(|row| WatchHistoryItem {
            source: row.get("source"),
            item_type: row.get("item_type"),
            id: row.get("id"),
            detail_id: row.get("detail_id"),
            name: row
                .get::<Option<String>, _>("name")
                .unwrap_or_else(|| "Unknown".to_string()),
            show_name: row.get("show_name"),
            season_number: row.get("season_number"),
            episode_number: row.get("episode_number"),
            year: row.get("year"),
            watched_at: row.get("watched_at"),
            poster_url: row.get("poster_url"),
            runtime: row.get("runtime"),
        })
        .collect();

    Ok(history)
}

#[cfg(test)]
mod watch_history_tests {
    use super::{get_watch_history_from_pool, resolve_plex_history_item_in_pool};
    use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};

    async fn fixture() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::raw_sql(
            "CREATE TABLE shows(id INTEGER PRIMARY KEY, name TEXT, poster_url TEXT);
             CREATE TABLE episodes(id INTEGER PRIMARY KEY, show_id INTEGER, name TEXT,
                 season_number INTEGER, episode_number INTEGER, watched INTEGER,
                 watched_at TEXT, runtime INTEGER);
             CREATE TABLE movies(id INTEGER PRIMARY KEY, title TEXT, watched INTEGER,
                 watched_at TEXT, poster_url TEXT, runtime INTEGER);
             CREATE TABLE plex_scrobble_log(id INTEGER PRIMARY KEY, event_type TEXT,
                 media_type TEXT, raw_title TEXT, show_name TEXT, season_number INTEGER,
                 episode_number INTEGER, year INTEGER, matched_entity_type TEXT,
                 matched_entity_id INTEGER, match_method TEXT, scrobbled_at TEXT);
             INSERT INTO shows VALUES (10, 'Tracked Show', 'show-poster'), (20, 'Other Show', NULL);
             INSERT INTO episodes VALUES (101, 10, 'First Episode', 1, 1, 1, '2026-11-01 10:00:00', 42);
             INSERT INTO episodes VALUES (102, 10, 'Second Episode', 1, 2, 1, '2026-11-02 10:00:00', 43);
             INSERT INTO movies VALUES (201, 'Tracked Movie', 1, '2026-11-03 10:00:00', 'movie-poster', 120);
             INSERT INTO movies VALUES (202, 'Untracked Movie', 0, NULL, 'new-movie-poster', 95);
             INSERT INTO plex_scrobble_log
                 (id,event_type,media_type,raw_title,show_name,season_number,episode_number,year,matched_entity_type,matched_entity_id,match_method,scrobbled_at)
             VALUES
                 (1, 'media.scrobble', 'episode', 'First Episode', 'Tracked Show', 1, 1, 2020, 'episode', 101, 'title', '2026-11-01 10:00:00'),
                 (2, 'media.scrobble', 'episode', 'Untracked Episode', 'Outside TVC', 2, 5, 2024, NULL, NULL, NULL, '2026-11-04 10:00:00'),
                 (3, 'media.scrobble', 'movie', 'Tracked Movie', NULL, NULL, NULL, 2021, 'movie', 201, 'title', '2026-11-03 10:00:00'),
                 (4, 'media.scrobble', 'movie', 'Untracked Movie', NULL, NULL, NULL, 1987, NULL, NULL, NULL, '2026-11-05 10:00:00'),
                 (5, 'media.scrobble', 'movie', 'Tracked Movie', NULL, NULL, NULL, 2021, 'movie', 201, 'title', '2026-11-06 10:00:00');",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool
    }

    #[tokio::test]
    async fn history_includes_unmatched_plex_plays_and_deduplicates_the_initial_match() {
        let pool = fixture().await;
        let history = get_watch_history_from_pool(&pool, 50).await.unwrap();

        assert_eq!(history.len(), 6);
        let unmatched_movie = history
            .iter()
            .find(|item| item.name == "Untracked Movie")
            .unwrap();
        assert_eq!(unmatched_movie.source, "plex");
        assert_eq!(unmatched_movie.item_type, "movie");
        assert_eq!(unmatched_movie.detail_id, None);
        assert_eq!(unmatched_movie.year, Some(1987));

        let unmatched_episode = history
            .iter()
            .find(|item| item.name == "Untracked Episode")
            .unwrap();
        assert_eq!(unmatched_episode.show_name.as_deref(), Some("Outside TVC"));
        assert_eq!(unmatched_episode.detail_id, None);

        let first_episode = history
            .iter()
            .find(|item| item.name == "First Episode")
            .unwrap();
        assert_eq!(first_episode.source, "plex");
        assert_eq!(first_episode.detail_id, Some(10));
        assert_eq!(history.iter().filter(|item| item.name == "Tracked Movie").count(), 2,
            "the matching initial play replaces the watched-state row, while a later rewatch remains");
    }

    #[tokio::test]
    async fn history_limit_is_independent_for_tv_and_movies_and_results_are_recent_first() {
        let pool = fixture().await;
        let history = get_watch_history_from_pool(&pool, 1).await.unwrap();

        assert_eq!(history.len(), 2);
        assert_eq!(history[0].name, "Tracked Movie");
        assert_eq!(history[1].name, "Untracked Episode");
    }

    #[tokio::test]
    async fn unmatched_plex_plays_can_be_linked_to_tier_only_titles_for_details() {
        let pool = fixture().await;
        sqlx::query("INSERT INTO episodes VALUES (303, 20, 'New Episode', 2, 5, 1, '2026-11-04 10:00:00', 42)")
            .execute(&pool)
            .await
            .unwrap();

        resolve_plex_history_item_in_pool(&pool, 2, "episode", 20)
            .await
            .unwrap();
        resolve_plex_history_item_in_pool(&pool, 4, "movie", 202)
            .await
            .unwrap();

        let history = get_watch_history_from_pool(&pool, 50).await.unwrap();
        let episode = history
            .iter()
            .find(|item| item.name == "Untracked Episode")
            .unwrap();
        assert_eq!(episode.detail_id, Some(20));
        assert_eq!(episode.show_name.as_deref(), Some("Outside TVC"));
        assert_eq!(
            history
                .iter()
                .filter(|item| item.watched_at == "2026-11-04 10:00:00")
                .count(),
            1,
            "linking the Plex play suppresses its duplicate watched episode"
        );

        let movie = history
            .iter()
            .find(|item| item.name == "Untracked Movie")
            .unwrap();
        assert_eq!(movie.detail_id, Some(202));
        assert_eq!(movie.poster_url.as_deref(), Some("new-movie-poster"));
    }

    #[tokio::test]
    async fn plex_history_link_rejects_wrong_media_and_missing_targets() {
        let pool = fixture().await;

        assert!(resolve_plex_history_item_in_pool(&pool, 2, "movie", 202)
            .await
            .is_err());
        assert!(resolve_plex_history_item_in_pool(&pool, 2, "episode", 999)
            .await
            .is_err());
        assert!(resolve_plex_history_item_in_pool(&pool, 2, "unknown", 20)
            .await
            .is_err());
    }
}
