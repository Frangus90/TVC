use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::AppHandle;
use crate::db::connection;

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
    pub item_type: String,
    pub id: i64,
    pub name: String,
    pub show_name: Option<String>,
    pub season_number: Option<i32>,
    pub episode_number: Option<i32>,
    pub watched_at: String,
    pub poster_url: Option<String>,
    pub runtime: Option<i32>,
}

#[tauri::command]
pub async fn get_watch_statistics(app: AppHandle) -> Result<WatchStatistics, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    // Get total watch time from episodes
    let episode_time: i64 = sqlx::query(
        r#"SELECT COALESCE(SUM(runtime), 0) as total FROM episodes WHERE watched = 1"#
    )
    .fetch_one(&pool)
    .await
    .map(|row| row.get("total"))
    .unwrap_or(0);

    // Get total watch time from movies
    let movie_time: i64 = sqlx::query(
        r#"SELECT COALESCE(SUM(runtime), 0) as total FROM movies WHERE watched = 1"#
    )
    .fetch_one(&pool)
    .await
    .map(|row| row.get("total"))
    .unwrap_or(0);

    // Get episodes watched count
    let episodes_watched: i64 = sqlx::query(
        r#"SELECT COUNT(*) as count FROM episodes WHERE watched = 1"#
    )
    .fetch_one(&pool)
    .await
    .map(|row| row.get("count"))
    .unwrap_or(0);

    // Get movies watched count
    let movies_watched: i64 = sqlx::query(
        r#"SELECT COUNT(*) as count FROM movies WHERE watched = 1"#
    )
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
        "#
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
        "#
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
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let date_format = match group_by.as_deref() {
        Some("month") => "%Y-%m",
        Some("year") => "%Y",
        _ => "%Y-%m-%d", // day is default
    };

    // SQLite doesn't support FULL OUTER JOIN, so we emulate it with UNION
    let rows = sqlx::query(&format!(
        r#"
        WITH episode_stats AS (
            SELECT
                strftime('{}', watched_at) as period,
                COUNT(*) as ep_count,
                COALESCE(SUM(runtime), 0) as ep_runtime
            FROM episodes
            WHERE watched = 1
              AND watched_at IS NOT NULL
              AND watched_at >= ?
              AND watched_at <= ?
            GROUP BY period
        ),
        movie_stats AS (
            SELECT
                strftime('{}', watched_at) as period,
                COUNT(*) as mv_count,
                COALESCE(SUM(runtime), 0) as mv_runtime
            FROM movies
            WHERE watched = 1
              AND watched_at IS NOT NULL
              AND watched_at >= ?
              AND watched_at <= ?
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
    .fetch_all(&pool)
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
    let pool = connection::get_pool(&app).await
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
        "#
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
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let limit_val = limit.unwrap_or(50);

    let rows = sqlx::query(
        r#"
        SELECT
            'episode' as item_type,
            e.id,
            e.name,
            s.name as show_name,
            e.season_number,
            e.episode_number,
            e.watched_at,
            s.poster_url,
            e.runtime
        FROM episodes e
        JOIN shows s ON e.show_id = s.id
        WHERE e.watched = 1 AND e.watched_at IS NOT NULL
        UNION ALL
        SELECT
            'movie' as item_type,
            m.id,
            m.title as name,
            NULL as show_name,
            NULL as season_number,
            NULL as episode_number,
            m.watched_at,
            m.poster_url,
            m.runtime
        FROM movies m
        WHERE m.watched = 1 AND m.watched_at IS NOT NULL
        ORDER BY watched_at DESC
        LIMIT ?
        "#
    )
    .bind(limit_val)
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to get watch history: {}", e))?;

    let history: Vec<WatchHistoryItem> = rows
        .into_iter()
        .map(|row| WatchHistoryItem {
            item_type: row.get("item_type"),
            id: row.get("id"),
            name: row.get::<Option<String>, _>("name").unwrap_or_else(|| "Unknown".to_string()),
            show_name: row.get("show_name"),
            season_number: row.get("season_number"),
            episode_number: row.get("episode_number"),
            watched_at: row.get("watched_at"),
            poster_url: row.get("poster_url"),
            runtime: row.get("runtime"),
        })
        .collect();

    Ok(history)
}
