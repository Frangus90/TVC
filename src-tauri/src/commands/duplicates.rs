use crate::db::connection;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::AppHandle;

#[derive(Debug, Serialize, Deserialize)]
pub struct DuplicatePair {
    pub show1_id: i64,
    pub show1_name: String,
    pub show1_episode_count: i64,
    pub show1_watched_count: i64,
    pub show1_poster_url: Option<String>,
    pub show2_id: i64,
    pub show2_name: String,
    pub show2_episode_count: i64,
    pub show2_watched_count: i64,
    pub show2_poster_url: Option<String>,
    pub similarity_reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MergeResult {
    pub episodes_moved: i64,
    pub episodes_merged: i64,
    pub deleted_show_id: i64,
}

/// Find potential duplicate shows based on name similarity.
/// shows.id is the TMDB id (primary key), so same-id duplicates are impossible.
#[tauri::command]
pub async fn find_duplicates(app: AppHandle) -> Result<Vec<DuplicatePair>, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let mut duplicates = Vec::new();

    // Find shows with exact same name (case-insensitive)
    let name_dups = sqlx::query(
        r#"SELECT s1.id as id1, s1.name as name1, s1.poster_url as poster1,
                  s2.id as id2, s2.name as name2, s2.poster_url as poster2
           FROM shows s1
           JOIN shows s2 ON LOWER(s1.name) = LOWER(s2.name) AND s1.id < s2.id"#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to find name duplicates: {}", e))?;

    for row in name_dups {
        let show1_id: i64 = row.get("id1");
        let show2_id: i64 = row.get("id2");

        let (ep1_count, watched1) = get_episode_counts(&pool, show1_id).await?;
        let (ep2_count, watched2) = get_episode_counts(&pool, show2_id).await?;

        duplicates.push(DuplicatePair {
            show1_id,
            show1_name: row.get("name1"),
            show1_episode_count: ep1_count,
            show1_watched_count: watched1,
            show1_poster_url: row.get("poster1"),
            show2_id,
            show2_name: row.get("name2"),
            show2_episode_count: ep2_count,
            show2_watched_count: watched2,
            show2_poster_url: row.get("poster2"),
            similarity_reason: "Same name".to_string(),
        });
    }

    Ok(duplicates)
}

async fn get_episode_counts(pool: &sqlx::SqlitePool, show_id: i64) -> Result<(i64, i64), String> {
    let row = sqlx::query(
        r#"SELECT COUNT(*) as total, SUM(CASE WHEN watched = 1 THEN 1 ELSE 0 END) as watched
           FROM episodes WHERE show_id = ?"#,
    )
    .bind(show_id)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Failed to get episode counts: {}", e))?;

    let total: i64 = row.get("total");
    let watched: i64 = row.try_get("watched").unwrap_or(0);

    Ok((total, watched))
}

/// Merge two duplicate shows - keeps the show with keep_id, deletes merge_id
#[tauri::command]
pub async fn merge_duplicates(
    app: AppHandle,
    keep_id: i64,
    merge_id: i64,
) -> Result<MergeResult, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    merge_in_pool(&pool, keep_id, merge_id).await
}

pub(crate) async fn merge_in_pool(
    pool: &sqlx::SqlitePool,
    keep_id: i64,
    merge_id: i64,
) -> Result<MergeResult, String> {
    if keep_id == merge_id {
        return Err("Choose two different shows".into());
    }
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM shows WHERE id IN (?, ?)")
        .bind(keep_id)
        .bind(merge_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    if count != 2 {
        return Err("Both shows must still exist before merging".into());
    }

    // Get episodes from both shows
    let keep_episodes: Vec<(i32, i32)> =
        sqlx::query(r#"SELECT season_number, episode_number FROM episodes WHERE show_id = ?"#)
            .bind(keep_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| format!("Failed to get keep episodes: {}", e))?
            .iter()
            .map(|row| (row.get("season_number"), row.get("episode_number")))
            .collect();

    let merge_episodes = sqlx::query(
        r#"SELECT id, season_number, episode_number, watched, scheduled_date, watched_at, rating, tags
           FROM episodes WHERE show_id = ?"#
    )
    .bind(merge_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| format!("Failed to get merge episodes: {}", e))?;

    let mut moved = 0i64;
    let mut merged = 0i64;

    for ep in merge_episodes {
        let season: i32 = ep.get("season_number");
        let episode: i32 = ep.get("episode_number");
        let ep_id: i64 = ep.get("id");
        let watched: bool = ep.try_get::<i32, _>("watched").unwrap_or(0) == 1;
        let scheduled_date: Option<String> = ep.get("scheduled_date");
        let watched_at: Option<String> = ep.get("watched_at");

        if keep_episodes.contains(&(season, episode)) {
            // Episode exists in both - merge watched status
            {
                sqlx::query(
                    r#"UPDATE episodes SET
                        watched = CASE WHEN watched = 1 THEN 1 ELSE ? END,
                        watched_at = CASE WHEN watched_at IS NOT NULL THEN watched_at ELSE ? END,
                        scheduled_date = CASE WHEN scheduled_date IS NOT NULL THEN scheduled_date ELSE ? END,
                        rating = COALESCE(rating, ?),
                        tags = (SELECT json_group_array(value) FROM (SELECT value FROM json_each(COALESCE(tags, '[]')) UNION SELECT value FROM json_each(COALESCE(?, '[]'))))
                       WHERE show_id = ? AND season_number = ? AND episode_number = ?"#
                )
                .bind(if watched { 1 } else { 0 })
                .bind(&watched_at)
                .bind(&scheduled_date)
                .bind(ep.get::<Option<f64>, _>("rating"))
                .bind(ep.get::<Option<String>, _>("tags"))
                .bind(keep_id)
                .bind(season)
                .bind(episode)
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("Failed to merge episode: {}", e))?;
                merged += 1;
            }
            let target_id: i64 = sqlx::query_scalar("SELECT id FROM episodes WHERE show_id = ? AND season_number = ? AND episode_number = ?")
                .bind(keep_id).bind(season).bind(episode).fetch_one(&mut *tx).await.map_err(|e| e.to_string())?;
            sqlx::query("UPDATE plex_scrobble_log SET matched_entity_id = ? WHERE matched_entity_type = 'episode' AND matched_entity_id = ?")
                .bind(target_id).bind(ep_id).execute(&mut *tx).await.map_err(|e| e.to_string())?;
            sqlx::query("UPDATE change_history SET entity_id = ? WHERE entity_type = 'episode' AND entity_id = ?")
                .bind(target_id).bind(ep_id).execute(&mut *tx).await.map_err(|e| e.to_string())?;
            // Delete the duplicate episode
            sqlx::query(r#"DELETE FROM episodes WHERE id = ?"#)
                .bind(ep_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("Failed to delete duplicate episode: {}", e))?;
        } else {
            // Episode doesn't exist in keep show - move it
            sqlx::query(r#"UPDATE episodes SET show_id = ? WHERE id = ?"#)
                .bind(keep_id)
                .bind(ep_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("Failed to move episode: {}", e))?;
            moved += 1;
        }
    }

    sqlx::query("UPDATE sonarr_imports SET show_id = ? WHERE show_id = ?")
        .bind(keep_id)
        .bind(merge_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("UPDATE title_mappings SET tvc_id = ?, tvc_title = (SELECT name FROM shows WHERE id = ?) WHERE media_type = 'show' AND tvc_id = ?")
        .bind(keep_id).bind(keep_id).bind(merge_id).execute(&mut *tx).await.map_err(|e| e.to_string())?;
    sqlx::query("UPDATE shows SET rating = COALESCE(rating, (SELECT rating FROM shows WHERE id = ?)) WHERE id = ?")
        .bind(merge_id).bind(keep_id).execute(&mut *tx).await.map_err(|e| e.to_string())?;
    sqlx::query(
        "UPDATE change_history SET entity_id = ? WHERE entity_type = 'show' AND entity_id = ?",
    )
    .bind(keep_id)
    .bind(merge_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    sqlx::query("UPDATE shows SET tags = (SELECT json_group_array(value) FROM (SELECT value FROM json_each(COALESCE(tags, '[]')) UNION SELECT value FROM json_each(COALESCE((SELECT tags FROM shows WHERE id = ?), '[]')))), notes = CASE WHEN notes IS NULL OR notes = '' THEN (SELECT notes FROM shows WHERE id = ?) WHEN COALESCE((SELECT notes FROM shows WHERE id = ?), '') = '' OR notes = (SELECT notes FROM shows WHERE id = ?) THEN notes ELSE notes || char(10) || char(10) || (SELECT notes FROM shows WHERE id = ?) END, tier_id = COALESCE(tier_id, (SELECT tier_id FROM shows WHERE id = ?)) WHERE id = ?")
        .bind(merge_id).bind(merge_id).bind(merge_id).bind(merge_id).bind(merge_id).bind(merge_id).bind(keep_id)
        .execute(&mut *tx).await.map_err(|e| e.to_string())?;
    // Delete the merged show
    sqlx::query(r#"DELETE FROM shows WHERE id = ?"#)
        .bind(merge_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to delete merged show: {}", e))?;

    tx.commit()
        .await
        .map_err(|e| format!("Failed to commit merge: {}", e))?;

    Ok(MergeResult {
        episodes_moved: moved,
        episodes_merged: merged,
        deleted_show_id: merge_id,
    })
}
