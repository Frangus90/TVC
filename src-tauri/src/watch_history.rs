use sqlx::SqlitePool;

pub enum Episodes {
    One(i64),
    Season(i64, i32),
    Show(i64),
}

pub async fn episodes(pool: &SqlitePool, scope: Episodes, watched: bool) -> Result<(), String> {
    let (filter, id, season) = match scope {
        Episodes::One(id) => ("id = ?", id, None),
        Episodes::Season(id, season) => ("show_id = ? AND season_number = ?", id, Some(season)),
        Episodes::Show(id) => ("show_id = ?", id, None),
    };
    let sql = format!(
        "UPDATE episodes SET watched = ?, watched_at =
        CASE WHEN ? = 0 THEN NULL WHEN watched = 1 THEN COALESCE(watched_at, datetime('now'))
        ELSE datetime('now') END WHERE {filter}"
    );
    let mut query = sqlx::query(&sql).bind(watched).bind(watched).bind(id);
    if let Some(season) = season {
        query = query.bind(season);
    }
    query
        .execute(pool)
        .await
        .map_err(|e| format!("Could not save watched state: {e}"))?;
    Ok(())
}

pub async fn movie(pool: &SqlitePool, id: i64, watched: bool) -> Result<(), String> {
    sqlx::query(
        "UPDATE movies SET watched = ?, watched_at =
        CASE WHEN ? = 0 THEN NULL WHEN watched = 1 THEN COALESCE(watched_at, datetime('now'))
        ELSE datetime('now') END WHERE id = ?",
    )
    .bind(watched)
    .bind(watched)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("Could not save watched state: {e}"))?;
    Ok(())
}
