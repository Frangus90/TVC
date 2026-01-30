use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::AppHandle;
use crate::db::connection;

#[derive(Debug, Serialize, Deserialize)]
pub struct ChangeHistoryItem {
    pub id: i64,
    pub entity_type: String,
    pub entity_id: i64,
    pub change_type: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub changed_at: String,
    pub user_action: Option<String>,
    // Joined data for display
    pub entity_name: Option<String>,
    pub show_name: Option<String>,
    pub poster_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChangeHistoryStats {
    pub total_changes: i64,
    pub watched_changes: i64,
    pub schedule_changes: i64,
    pub rating_changes: i64,
}

/// Log a change to the history table
#[allow(dead_code)]
pub async fn log_change(
    app: &AppHandle,
    entity_type: &str,
    entity_id: i64,
    change_type: &str,
    old_value: Option<&str>,
    new_value: Option<&str>,
    user_action: &str,
) -> Result<(), String> {
    let pool = connection::get_pool(app).await
        .map_err(|e| format!("Database error: {}", e))?;

    sqlx::query(
        r#"INSERT INTO change_history (entity_type, entity_id, change_type, old_value, new_value, user_action)
           VALUES (?, ?, ?, ?, ?, ?)"#
    )
    .bind(entity_type)
    .bind(entity_id)
    .bind(change_type)
    .bind(old_value)
    .bind(new_value)
    .bind(user_action)
    .execute(&pool)
    .await
    .map_err(|e| format!("Failed to log change: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn get_change_history(
    app: AppHandle,
    entity_type: Option<String>,
    change_type: Option<String>,
    limit: Option<i32>,
) -> Result<Vec<ChangeHistoryItem>, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let limit = limit.unwrap_or(100);

    // Build query using sqlx query builder for safety (all parameters are bound, not interpolated)
    let base_query = r#"SELECT
        ch.id, ch.entity_type, ch.entity_id, ch.change_type,
        ch.old_value, ch.new_value, ch.changed_at, ch.user_action,
        CASE
            WHEN ch.entity_type = 'episode' THEN e.name
            WHEN ch.entity_type = 'movie' THEN m.title
            WHEN ch.entity_type = 'show' THEN s.name
        END as entity_name,
        CASE
            WHEN ch.entity_type = 'episode' THEN s2.name
            ELSE NULL
        END as show_name,
        CASE
            WHEN ch.entity_type = 'episode' THEN s2.poster_url
            WHEN ch.entity_type = 'movie' THEN m.poster_url
            WHEN ch.entity_type = 'show' THEN s.poster_url
        END as poster_url
    FROM change_history ch
    LEFT JOIN episodes e ON ch.entity_type = 'episode' AND ch.entity_id = e.id
    LEFT JOIN shows s2 ON e.show_id = s2.id
    LEFT JOIN movies m ON ch.entity_type = 'movie' AND ch.entity_id = m.id
    LEFT JOIN shows s ON ch.entity_type = 'show' AND ch.entity_id = s.id
    WHERE 1=1"#;

    // Build query with proper parameterization (all dynamic parts use .bind())
    // Construct WHERE clause conditionally but all values are parameterized
    let where_clause = match (entity_type.as_ref(), change_type.as_ref()) {
        (Some(_), Some(_)) => " AND ch.entity_type = ? AND ch.change_type = ?",
        (Some(_), None) => " AND ch.entity_type = ?",
        (None, Some(_)) => " AND ch.change_type = ?",
        (None, None) => "",
    };
    
    let query = format!("{} {} ORDER BY ch.changed_at DESC LIMIT ?", base_query, where_clause);
    let mut q = sqlx::query(&query);

    if let Some(ref et) = entity_type {
        q = q.bind(et);
    }
    if let Some(ref ct) = change_type {
        q = q.bind(ct);
    }
    q = q.bind(limit);

    let rows = q.fetch_all(&pool)
        .await
        .map_err(|e| format!("Failed to fetch change history: {}", e))?;

    let items: Vec<ChangeHistoryItem> = rows.iter().map(|row| {
        ChangeHistoryItem {
            id: row.get("id"),
            entity_type: row.get("entity_type"),
            entity_id: row.get("entity_id"),
            change_type: row.get("change_type"),
            old_value: row.get("old_value"),
            new_value: row.get("new_value"),
            changed_at: row.get("changed_at"),
            user_action: row.get("user_action"),
            entity_name: row.get("entity_name"),
            show_name: row.get("show_name"),
            poster_url: row.get("poster_url"),
        }
    }).collect();

    Ok(items)
}

#[tauri::command]
pub async fn get_change_history_stats(app: AppHandle) -> Result<ChangeHistoryStats, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let total: i64 = sqlx::query(r#"SELECT COUNT(*) as count FROM change_history"#)
        .fetch_one(&pool)
        .await
        .map(|row| row.get("count"))
        .unwrap_or(0);

    let watched: i64 = sqlx::query(r#"SELECT COUNT(*) as count FROM change_history WHERE change_type = 'watched'"#)
        .fetch_one(&pool)
        .await
        .map(|row| row.get("count"))
        .unwrap_or(0);

    let schedule: i64 = sqlx::query(r#"SELECT COUNT(*) as count FROM change_history WHERE change_type IN ('scheduled', 'unscheduled')"#)
        .fetch_one(&pool)
        .await
        .map(|row| row.get("count"))
        .unwrap_or(0);

    let rating: i64 = sqlx::query(r#"SELECT COUNT(*) as count FROM change_history WHERE change_type = 'rating'"#)
        .fetch_one(&pool)
        .await
        .map(|row| row.get("count"))
        .unwrap_or(0);

    Ok(ChangeHistoryStats {
        total_changes: total,
        watched_changes: watched,
        schedule_changes: schedule,
        rating_changes: rating,
    })
}

#[tauri::command]
pub async fn clear_change_history(app: AppHandle) -> Result<i64, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let result = sqlx::query(r#"DELETE FROM change_history"#)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to clear history: {}", e))?;

    Ok(result.rows_affected() as i64)
}
