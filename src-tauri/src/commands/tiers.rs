use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row};
use tauri::AppHandle;
use crate::db::connection;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Tier {
    pub id: i64,
    pub position: i64,
    pub name: String,
    pub color: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteTierResult {
    pub affected_shows: i64,
    pub affected_movies: i64,
}

#[derive(Debug, Deserialize)]
pub struct TierDefinition {
    pub name: String,
    pub color: String,
}

#[derive(Debug, Serialize)]
pub struct ApplyTierConfigurationResult {
    pub tiers: Vec<Tier>,
    pub cleared_shows: u64,
    pub cleared_movies: u64,
}

#[tauri::command]
pub async fn apply_tier_configuration(
    app: AppHandle,
    preset: String,
    tiers: Vec<TierDefinition>,
) -> Result<ApplyTierConfigurationResult, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;
    apply_configuration(&pool, &preset, tiers).await
}

async fn apply_configuration(
    pool: &sqlx::SqlitePool,
    preset: &str,
    mut definitions: Vec<TierDefinition>,
) -> Result<ApplyTierConfigurationResult, String> {
    if !matches!(preset, "10-star" | "5-star" | "custom") {
        return Err("Choose a valid tier preset".into());
    }
    if definitions.is_empty() || definitions.len() > 100 {
        return Err("Use between 1 and 100 tiers".into());
    }
    for tier in &mut definitions {
        tier.name = tier.name.trim().to_string();
        if tier.name.is_empty() || tier.name.chars().count() > 120 {
            return Err("Tier names must contain 1 to 120 characters".into());
        }
        if !tier.color.is_empty()
            && !(tier.color.len() == 7
                && tier.color.starts_with('#')
                && tier.color[1..].bytes().all(|byte| byte.is_ascii_hexdigit()))
        {
            return Err("Choose a valid tier colour".into());
        }
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Could not start tier update: {}", e))?;
    let cleared_shows = sqlx::query("UPDATE shows SET tier_id = NULL, rank_order = NULL WHERE tier_id IS NOT NULL OR rank_order IS NOT NULL")
        .execute(&mut *tx).await.map_err(|e| format!("Could not clear show tiers: {}", e))?.rows_affected();
    let cleared_movies = sqlx::query("UPDATE movies SET tier_id = NULL, rank_order = NULL WHERE tier_id IS NOT NULL OR rank_order IS NOT NULL")
        .execute(&mut *tx).await.map_err(|e| format!("Could not clear movie tiers: {}", e))?.rows_affected();
    sqlx::query("DELETE FROM tiers")
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Could not replace tiers: {}", e))?;
    let count = definitions.len() as i64;
    let mut saved = Vec::with_capacity(definitions.len());
    for (index, definition) in definitions.into_iter().enumerate() {
        let tier = sqlx::query_as::<_, Tier>("INSERT INTO tiers (position, name, color) VALUES (?, ?, ?) RETURNING id, position, name, color, created_at")
            .bind(count - index as i64).bind(definition.name).bind(definition.color)
            .fetch_one(&mut *tx).await.map_err(|e| format!("Could not save tier: {}", e))?;
        saved.push(tier);
    }
    sqlx::query("INSERT OR REPLACE INTO settings (key, value) VALUES ('tier_preset', ?)")
        .bind(preset)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Could not save tier preset: {}", e))?;
    tx.commit()
        .await
        .map_err(|e| format!("Could not finish tier update: {}", e))?;
    Ok(ApplyTierConfigurationResult {
        tiers: saved,
        cleared_shows,
        cleared_movies,
    })
}

#[cfg(test)]
mod configuration_tests {
    use super::{apply_configuration, TierDefinition};
    use sqlx::{sqlite::SqlitePoolOptions, Row};

    async fn fixture() -> sqlx::SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::raw_sql("PRAGMA foreign_keys=ON;
            CREATE TABLE tiers(id INTEGER PRIMARY KEY AUTOINCREMENT, position INTEGER, name TEXT, color TEXT, created_at TEXT DEFAULT CURRENT_TIMESTAMP);
            CREATE TABLE settings(key TEXT PRIMARY KEY, value TEXT);
            CREATE TABLE shows(id INTEGER PRIMARY KEY, name TEXT, tier_id INTEGER REFERENCES tiers(id) ON DELETE SET NULL, rank_order INTEGER, tier_only INTEGER, archived INTEGER, notes TEXT);
            CREATE TABLE movies(id INTEGER PRIMARY KEY, title TEXT, tier_id INTEGER REFERENCES tiers(id) ON DELETE SET NULL, rank_order INTEGER, tier_only INTEGER, archived INTEGER, watched INTEGER, watched_at TEXT, scheduled_date TEXT);
            INSERT INTO tiers(id,position,name,color) VALUES(1,2,'Old best',''),(2,1,'Old good','');
            INSERT INTO settings VALUES('tier_preset','10-star');
            INSERT INTO shows VALUES(11,'Tracked',1,0,0,0,'Keep notes'),(12,'Tier only',2,1,1,0,'Keep history'),(13,'Archived',1,2,1,1,'Keep archived');
            INSERT INTO movies VALUES(21,'Tracked movie',1,0,0,0,1,'2026-10-01','2026-10-30'),(22,'Tier movie',2,1,1,0,0,NULL,NULL),(23,'Archived movie',1,2,1,1,1,'2025-01-01',NULL);").execute(&pool).await.unwrap();
        pool
    }

    fn definitions() -> Vec<TierDefinition> {
        vec![
            TierDefinition {
                name: "  Favourite  ".into(),
                color: "#8b5cf6".into(),
            },
            TierDefinition {
                name: "Good".into(),
                color: "".into(),
            },
        ]
    }

    #[tokio::test]
    async fn confirmed_configuration_clears_both_lists_but_keeps_personal_data() {
        let pool = fixture().await;
        let result = apply_configuration(&pool, "custom", definitions())
            .await
            .unwrap();
        assert_eq!((result.cleared_shows, result.cleared_movies), (3, 3));
        assert_eq!(result.tiers[0].name, "Favourite");
        assert_eq!((result.tiers[0].position, result.tiers[1].position), (2, 1));
        for table in ["shows", "movies"] {
            let remaining: i64 = sqlx::query_scalar(&format!(
                "SELECT COUNT(*) FROM {table} WHERE tier_id IS NOT NULL OR rank_order IS NOT NULL"
            ))
            .fetch_one(&pool)
            .await
            .unwrap();
            assert_eq!(remaining, 0);
            let titles: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!(titles, 3);
        }
        let movie = sqlx::query(
            "SELECT watched,watched_at,scheduled_date,tier_only,archived FROM movies WHERE id=21",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(movie.get::<i64, _>("watched"), 1);
        assert_eq!(movie.get::<String, _>("watched_at"), "2026-10-01");
        assert_eq!(movie.get::<String, _>("scheduled_date"), "2026-10-30");
        assert_eq!(
            sqlx::query_scalar::<_, String>("SELECT notes FROM shows WHERE id=12")
                .fetch_one(&pool)
                .await
                .unwrap(),
            "Keep history"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT archived FROM movies WHERE id=23")
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT tier_only FROM movies WHERE id=22")
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key='tier_preset'")
                .fetch_one(&pool)
                .await
                .unwrap(),
            "custom"
        );
    }

    #[tokio::test]
    async fn failed_configuration_rolls_back_tiers_assignments_and_preset() {
        let pool = fixture().await;
        sqlx::raw_sql("CREATE TRIGGER reject_new_tier BEFORE INSERT ON tiers WHEN NEW.name='Good' BEGIN SELECT RAISE(ABORT,'fixture failure'); END;").execute(&pool).await.unwrap();
        assert!(apply_configuration(&pool, "custom", definitions())
            .await
            .is_err());
        assert_eq!(
            sqlx::query_scalar::<_, String>("SELECT name FROM tiers WHERE id=1")
                .fetch_one(&pool)
                .await
                .unwrap(),
            "Old best"
        );
        for table in ["shows", "movies"] {
            assert_eq!(sqlx::query_scalar::<_,i64>(&format!("SELECT COUNT(*) FROM {table} WHERE tier_id IS NOT NULL AND rank_order IS NOT NULL")).fetch_one(&pool).await.unwrap(), 3);
        }
        assert_eq!(
            sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key='tier_preset'")
                .fetch_one(&pool)
                .await
                .unwrap(),
            "10-star"
        );
    }

    #[tokio::test]
    async fn invalid_configuration_never_changes_the_existing_layout() {
        let pool = fixture().await;
        assert!(apply_configuration(&pool, "unknown", definitions())
            .await
            .is_err());
        assert!(apply_configuration(&pool, "custom", vec![]).await.is_err());
        assert!(apply_configuration(
            &pool,
            "custom",
            vec![TierDefinition {
                name: " ".into(),
                color: "".into()
            }]
        )
        .await
        .is_err());
        assert!(apply_configuration(
            &pool,
            "custom",
            vec![TierDefinition {
                name: "Good".into(),
                color: "red; background:url(x)".into()
            }]
        )
        .await
        .is_err());
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT tier_id FROM shows WHERE id=11")
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tiers")
                .fetch_one(&pool)
                .await
                .unwrap(),
            2
        );
    }
}

// ============================================
// Tier CRUD commands
// ============================================

#[tauri::command]
pub async fn get_tiers(app: AppHandle) -> Result<Vec<Tier>, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let tiers = sqlx::query_as::<_, Tier>(
        "SELECT id, position, name, color, created_at FROM tiers ORDER BY position DESC"
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to get tiers: {}", e))?;

    Ok(tiers)
}

#[tauri::command]
pub async fn create_tier(
    app: AppHandle,
    name: String,
    color: String,
) -> Result<Tier, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    // New tier gets position 0 (bottom). User reorders after.
    let result = sqlx::query(
        "INSERT INTO tiers (position, name, color) VALUES (0, ?, ?) RETURNING id, position, name, color, created_at"
    )
    .bind(&name)
    .bind(&color)
    .fetch_one(&pool)
    .await
    .map_err(|e| format!("Failed to create tier: {}", e))?;

    Ok(Tier {
        id: result.get("id"),
        position: result.get("position"),
        name: result.get("name"),
        color: result.get("color"),
        created_at: result.get("created_at"),
    })
}

#[tauri::command]
pub async fn update_tier(
    app: AppHandle,
    id: i64,
    name: Option<String>,
    color: Option<String>,
) -> Result<(), String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    if let Some(name) = name {
        sqlx::query("UPDATE tiers SET name = ? WHERE id = ?")
            .bind(&name)
            .bind(id)
            .execute(&pool)
            .await
            .map_err(|e| format!("Failed to update tier name: {}", e))?;
    }

    if let Some(color) = color {
        sqlx::query("UPDATE tiers SET color = ? WHERE id = ?")
            .bind(&color)
            .bind(id)
            .execute(&pool)
            .await
            .map_err(|e| format!("Failed to update tier color: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub async fn delete_tier(
    app: AppHandle,
    id: i64,
) -> Result<DeleteTierResult, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let mut tx = pool.begin().await
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    // Count affected items and delete within the same transaction
    let affected_shows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM shows WHERE tier_id = ?"
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| format!("Failed to count affected shows: {}", e))?;

    let affected_movies: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM movies WHERE tier_id = ?"
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| format!("Failed to count affected movies: {}", e))?;

    // Delete the tier (ON DELETE SET NULL will unrate items)
    sqlx::query("DELETE FROM tiers WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to delete tier: {}", e))?;

    tx.commit().await
        .map_err(|e| format!("Failed to commit tier deletion: {}", e))?;

    Ok(DeleteTierResult {
        affected_shows,
        affected_movies,
    })
}

#[tauri::command]
pub async fn reorder_tiers(
    app: AppHandle,
    tier_ids: Vec<i64>,
) -> Result<(), String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let mut tx = pool.begin().await
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    // Assign positions: first in vec = highest position (best tier)
    let total = tier_ids.len() as i64;
    for (i, tier_id) in tier_ids.iter().enumerate() {
        let position = total - i as i64;
        sqlx::query("UPDATE tiers SET position = ? WHERE id = ?")
            .bind(position)
            .bind(tier_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to reorder tier: {}", e))?;
    }

    tx.commit().await
        .map_err(|e| format!("Failed to commit tier reorder: {}", e))?;

    Ok(())
}

// ============================================
// Tier preset commands
// ============================================

#[tauri::command]
pub async fn get_tier_preset(app: AppHandle) -> Result<String, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let preset: Option<String> = sqlx::query_scalar(
        "SELECT value FROM settings WHERE key = 'tier_preset'"
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| format!("Failed to get tier preset: {}", e))?;

    Ok(preset.unwrap_or_else(|| "10-star".to_string()))
}

#[tauri::command]
pub async fn apply_tier_preset(
    app: AppHandle,
    preset: String,
) -> Result<(), String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let tiers_data: Vec<(&str, i64)> = match preset.as_str() {
        "10-star" => vec![
            ("Masterpiece", 10), ("Excellent", 9), ("Great", 8),
            ("Good", 7), ("Solid", 6), ("Average", 5),
            ("Below Average", 4), ("Poor", 3), ("Bad", 2), ("Terrible", 1),
        ],
        "5-star" => vec![
            ("Excellent", 5), ("Great", 4), ("Good", 3),
            ("Average", 2), ("Poor", 1),
        ],
        _ => return Err(format!("Unknown preset: {}", preset)),
    };

    let mut tx = pool.begin().await
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    // Unrate all items first
    sqlx::query("UPDATE shows SET tier_id = NULL WHERE tier_id IS NOT NULL")
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to unrate shows: {}", e))?;

    sqlx::query("UPDATE movies SET tier_id = NULL WHERE tier_id IS NOT NULL")
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to unrate movies: {}", e))?;

    // Clear existing tiers
    sqlx::query("DELETE FROM tiers")
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to clear tiers: {}", e))?;

    // Insert new tiers
    for (name, position) in tiers_data {
        sqlx::query("INSERT INTO tiers (position, name, color) VALUES (?, ?, '')")
            .bind(position)
            .bind(name)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to insert tier: {}", e))?;
    }

    // Save preset preference
    sqlx::query("INSERT OR REPLACE INTO settings (key, value) VALUES ('tier_preset', ?)")
        .bind(&preset)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to save preset: {}", e))?;

    tx.commit().await
        .map_err(|e| format!("Failed to commit preset change: {}", e))?;

    Ok(())
}

// ============================================
// Tier list item commands
// ============================================

#[derive(Debug, Serialize, Deserialize)]
pub struct TierListShow {
    pub id: i64,
    pub name: String,
    pub first_aired: Option<String>,
    pub poster_url: Option<String>,
    pub tier_id: Option<i64>,
    pub rank_order: Option<i32>,
    pub tier_only: bool,
    pub episode_count: i64,
    pub watched_episodes: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TierListMovie {
    pub id: i64,
    pub title: String,
    pub release_date: Option<String>,
    pub poster_url: Option<String>,
    pub tier_id: Option<i64>,
    pub rank_order: Option<i32>,
    pub tier_only: bool,
    pub watched: bool,
}

#[tauri::command]
pub async fn get_tier_list_shows(app: AppHandle) -> Result<Vec<TierListShow>, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    tier_list_shows(&pool).await
}

pub(crate) async fn tier_list_shows(pool: &sqlx::SqlitePool) -> Result<Vec<TierListShow>, String> {
    let rows = sqlx::query(
        r#"
        SELECT id, name, first_aired, poster_url, tier_id, rank_order, tier_only,
               (SELECT COUNT(*) FROM episodes WHERE show_id = shows.id) AS episode_count,
               (SELECT COUNT(*) FROM episodes WHERE show_id = shows.id AND watched = 1) AS watched_episodes
        FROM shows
        WHERE tier_only = 1 OR tier_id IS NOT NULL
        ORDER BY name
        LIMIT 10000
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to get tier list shows: {}", e))?;

    let shows: Vec<TierListShow> = rows
        .into_iter()
        .map(|row| TierListShow {
            id: row.get("id"),
            name: row.get("name"),
            first_aired: row.get("first_aired"),
            poster_url: row.get("poster_url"),
            tier_id: row.get("tier_id"),
            rank_order: row.get("rank_order"),
            tier_only: row.get::<i32, _>("tier_only") == 1,
            episode_count: row.get("episode_count"),
            watched_episodes: row.get("watched_episodes"),
        })
        .collect();

    eprintln!("[tier-add] get_tier_list_shows returned {} rows", shows.len());

    Ok(shows)
}

#[tauri::command]
pub async fn get_tier_list_movies(app: AppHandle) -> Result<Vec<TierListMovie>, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    tier_list_movies(&pool).await
}

pub(crate) async fn tier_list_movies(pool: &sqlx::SqlitePool) -> Result<Vec<TierListMovie>, String> {
    let rows = sqlx::query(
        r#"
        SELECT id, title, release_date, poster_url, tier_id, rank_order, tier_only, watched
        FROM movies
        WHERE tier_only = 1 OR tier_id IS NOT NULL
        ORDER BY title
        LIMIT 10000
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to get tier list movies: {}", e))?;

    let movies: Vec<TierListMovie> = rows
        .into_iter()
        .map(|row| TierListMovie {
            id: row.get("id"),
            title: row.get("title"),
            release_date: row.get("release_date"),
            poster_url: row.get("poster_url"),
            tier_id: row.get("tier_id"),
            rank_order: row.get("rank_order"),
            tier_only: row.get::<i32, _>("tier_only") == 1,
            watched: row.get::<i32, _>("watched") == 1,
        })
        .collect();

    eprintln!("[tier-add] get_tier_list_movies returned {} rows", movies.len());

    Ok(movies)
}

#[cfg(test)]
mod watched_summary_tests {
    use super::*;

    #[tokio::test]
    async fn show_summaries_count_all_saved_episodes_and_keep_empty_shows_unwatched() {
        let pool = crate::audit_tests::database().await;
        sqlx::raw_sql(
            "INSERT INTO tiers (id,position,name,color) VALUES (1001,1001,'Test','#ffffff');
            INSERT INTO shows (id,name,tier_id,rank_order,tier_only) VALUES
            (1,'Ranked show',1001,3,0),(2,'Empty tier show',NULL,NULL,1),
            (-1,'Manual tier show',1001,4,1),(3,'Outside tiers',NULL,NULL,0);
            INSERT INTO episodes (id,show_id,season_number,episode_number,aired,watched) VALUES
            (10,1,1,1,'2020-01-01',1),(11,1,1,2,'2099-01-01',0),
            (12,1,0,1,NULL,1),(-10,-1,1,1,NULL,1),(30,3,1,1,NULL,1);",
        )
        .execute(&pool)
        .await
        .unwrap();

        let shows = tier_list_shows(&pool).await.unwrap();
        assert_eq!(shows.len(), 3);
        let ranked = shows.iter().find(|show| show.id == 1).unwrap();
        assert_eq!((ranked.episode_count, ranked.watched_episodes), (3, 2));
        assert_eq!((ranked.tier_id, ranked.rank_order), (Some(1001), Some(3)));
        let empty = shows.iter().find(|show| show.id == 2).unwrap();
        assert_eq!((empty.episode_count, empty.watched_episodes), (0, 0));
        let manual = shows.iter().find(|show| show.id == -1).unwrap();
        assert_eq!((manual.episode_count, manual.watched_episodes), (1, 1));
        assert!(manual.tier_only);

        crate::watch_history::episodes(&pool, crate::watch_history::Episodes::Show(1), true)
            .await
            .unwrap();
        let refreshed = tier_list_shows(&pool).await.unwrap();
        let ranked = refreshed.iter().find(|show| show.id == 1).unwrap();
        assert_eq!((ranked.episode_count, ranked.watched_episodes), (3, 3));
    }

    #[tokio::test]
    async fn movie_summaries_expose_saved_watched_state_for_tracked_and_tier_only_titles() {
        let pool = crate::audit_tests::database().await;
        sqlx::raw_sql(
            "INSERT INTO tiers (id,position,name,color) VALUES (1001,1001,'Test','#ffffff');
            INSERT INTO movies (id,title,tier_id,rank_order,tier_only,watched) VALUES
            (1,'Watched ranked',1001,3,0,1),(2,'Watched tier only',NULL,NULL,1,1),
            (3,'Unwatched tier only',NULL,NULL,1,0),(4,'Outside tiers',NULL,NULL,0,1);",
        )
        .execute(&pool)
        .await
        .unwrap();

        let movies = tier_list_movies(&pool).await.unwrap();
        assert_eq!(movies.len(), 3);
        assert!(movies.iter().find(|movie| movie.id == 1).unwrap().watched);
        assert!(movies.iter().find(|movie| movie.id == 2).unwrap().watched);
        assert!(!movies.iter().find(|movie| movie.id == 3).unwrap().watched);
        let ranked = movies.iter().find(|movie| movie.id == 1).unwrap();
        assert_eq!((ranked.tier_id, ranked.rank_order), (Some(1001), Some(3)));
        assert!(!ranked.tier_only);
    }
}

// ============================================
// Tier-only add/promote/demote commands
// ============================================

#[tauri::command]
pub async fn add_show_tier_only(
    app: AppHandle,
    id: i64,
    tier_id: Option<i64>,
) -> Result<(), String> {
    eprintln!("[tier-add] add_show_tier_only id={} tier_id={:?}", id, tier_id);
    crate::commands::validation::validate_id(id)?;

    let show_details = crate::tmdb::get_tv_details(id)
        .await
        .map_err(|e| {
            eprintln!("[tier-add] TMDB fetch failed for show id={}: {}", id, e);
            format!("Failed to fetch show details: {}", e)
        })?;
    eprintln!("[tier-add] TMDB ok: id={} name={:?}", show_details.id, show_details.name);

    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let result = sqlx::query(
        r#"
        INSERT INTO shows
        (id, name, status, poster_url, first_aired, overview, network, runtime, added_at, tier_only, tier_id)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, datetime('now'), 1, ?)
        ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            status = excluded.status,
            poster_url = excluded.poster_url,
            first_aired = excluded.first_aired,
            overview = excluded.overview,
            network = excluded.network,
            runtime = excluded.runtime,
            tier_id = excluded.tier_id
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
    .bind(tier_id)
    .execute(&pool)
    .await
    .map_err(|e| {
        eprintln!("[tier-add] INSERT failed for show id={}: {}", id, e);
        format!("Failed to add show to tier list: {}", e)
    })?;
    eprintln!(
        "[tier-add] INSERT ok: id={} rows_affected={}",
        id,
        result.rows_affected()
    );

    // Echo the row back so we can confirm what landed in the DB.
    if let Ok(row) = sqlx::query(
        "SELECT tier_only, tier_id, archived FROM shows WHERE id = ?",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    {
        let tier_only: i32 = row.get("tier_only");
        let resolved_tier_id: Option<i64> = row.get("tier_id");
        let archived: Option<i32> = row.try_get("archived").ok();
        eprintln!(
            "[tier-add] row state: id={} tier_only={} tier_id={:?} archived={:?}",
            id, tier_only, resolved_tier_id, archived
        );
    }

    Ok(())
}

#[tauri::command]
pub async fn add_movie_tier_only(
    app: AppHandle,
    id: i64,
    tier_id: Option<i64>,
) -> Result<(), String> {
    eprintln!("[tier-add] add_movie_tier_only id={} tier_id={:?}", id, tier_id);
    crate::commands::validation::validate_id(id)?;

    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;
    let country = crate::commands::preferences::release_country(&pool).await?;
    let (movie_details, release_dates) = crate::tmdb::get_movie_with_release_dates(id, &country)
        .await
        .map_err(|e| {
            eprintln!("[tier-add] TMDB fetch failed for movie id={}: {}", id, e);
            format!("Failed to fetch movie details: {}", e)
        })?;
    eprintln!("[tier-add] TMDB ok: id={} title={:?}", movie_details.id, movie_details.title);

    let genres_json = movie_details.genres_string();

    sqlx::query(
        r#"
        INSERT INTO movies
        (id, title, tagline, overview, poster_url, backdrop_url, release_date,
         digital_release_date, physical_release_date, runtime, status, genres,
         vote_average, added_at, last_synced, tier_only, tier_id)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'), 1, ?)
        ON CONFLICT(id) DO UPDATE SET
            title = excluded.title,
            tagline = excluded.tagline,
            overview = excluded.overview,
            poster_url = excluded.poster_url,
            backdrop_url = excluded.backdrop_url,
            release_date = excluded.release_date,
            digital_release_date = excluded.digital_release_date,
            physical_release_date = excluded.physical_release_date,
            runtime = excluded.runtime,
            status = excluded.status,
            genres = excluded.genres,
            vote_average = excluded.vote_average,
            last_synced = datetime('now'),
            tier_id = excluded.tier_id
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
    .bind(tier_id)
    .execute(&pool)
    .await
    .map_err(|e| {
        eprintln!("[tier-add] INSERT failed for movie id={}: {}", id, e);
        format!("Failed to add movie to tier list: {}", e)
    })?;
    eprintln!("[tier-add] INSERT ok for movie id={}", id);

    Ok(())
}

// Manual entries use negative IDs (positive IDs collide with TMDB).
// Microsecond timestamp gives ~10^6 IDs/sec headroom; on collision we walk
// down by 1 until INSERT succeeds. Caps retries to avoid pathological loops.
async fn insert_manual_show(
    pool: &sqlx::SqlitePool,
    title: &str,
    poster_url: Option<&str>,
    tier_id: Option<i64>,
) -> Result<i64, String> {
    let mut candidate = -(chrono::Utc::now().timestamp_micros());
    for _ in 0..1000 {
        let result = sqlx::query(
            r#"
            INSERT INTO shows (id, name, poster_url, added_at, tier_only, tier_id)
            VALUES (?, ?, ?, datetime('now'), 1, ?)
            "#,
        )
        .bind(candidate)
        .bind(title)
        .bind(poster_url)
        .bind(tier_id)
        .execute(pool)
        .await;

        match result {
            Ok(_) => return Ok(candidate),
            Err(sqlx::Error::Database(db_err)) if db_err.message().contains("UNIQUE") => {
                candidate -= 1;
            }
            Err(e) => return Err(format!("Failed to add manual show: {}", e)),
        }
    }
    Err("Failed to generate unique manual show ID after 1000 attempts".to_string())
}

async fn insert_manual_movie(
    pool: &sqlx::SqlitePool,
    title: &str,
    poster_url: Option<&str>,
    tier_id: Option<i64>,
) -> Result<i64, String> {
    let mut candidate = -(chrono::Utc::now().timestamp_micros());
    for _ in 0..1000 {
        let result = sqlx::query(
            r#"
            INSERT INTO movies (id, title, poster_url, added_at, tier_only, tier_id)
            VALUES (?, ?, ?, datetime('now'), 1, ?)
            "#,
        )
        .bind(candidate)
        .bind(title)
        .bind(poster_url)
        .bind(tier_id)
        .execute(pool)
        .await;

        match result {
            Ok(_) => return Ok(candidate),
            Err(sqlx::Error::Database(db_err)) if db_err.message().contains("UNIQUE") => {
                candidate -= 1;
            }
            Err(e) => return Err(format!("Failed to add manual movie: {}", e)),
        }
    }
    Err("Failed to generate unique manual movie ID after 1000 attempts".to_string())
}

#[tauri::command]
pub async fn add_manual_show(
    app: AppHandle,
    title: String,
    poster_url: Option<String>,
    tier_id: Option<i64>,
) -> Result<i64, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    insert_manual_show(&pool, &title, poster_url.as_deref(), tier_id).await
}

#[tauri::command]
pub async fn add_manual_movie(
    app: AppHandle,
    title: String,
    poster_url: Option<String>,
    tier_id: Option<i64>,
) -> Result<i64, String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    insert_manual_movie(&pool, &title, poster_url.as_deref(), tier_id).await
}

#[tauri::command]
pub async fn promote_show_to_tracked(app: AppHandle, id: i64) -> Result<(), String> {
    if id < 0 {
        return Err("Cannot promote manual entries to tracked (no external data source)".to_string());
    }

    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    sqlx::query("UPDATE shows SET tier_only = 0 WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to promote show: {}", e))?;

    // Sync episodes for the newly tracked show
    crate::commands::episodes::sync_show_episodes(app, id).await?;

    Ok(())
}

#[tauri::command]
pub async fn promote_movie_to_tracked(app: AppHandle, id: i64) -> Result<(), String> {
    if id < 0 {
        return Err("Cannot promote manual entries to tracked (no external data source)".to_string());
    }

    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    sqlx::query("UPDATE movies SET tier_only = 0 WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to promote movie: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn demote_show_to_tier_only(app: AppHandle, id: i64) -> Result<(), String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    // Set tier_only flag
    sqlx::query("UPDATE shows SET tier_only = 1 WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to demote show: {}", e))?;

    // Clear scheduled episodes for this show
    sqlx::query("UPDATE episodes SET scheduled_date = NULL WHERE show_id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to clear episode schedules: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn demote_movie_to_tier_only(app: AppHandle, id: i64) -> Result<(), String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    sqlx::query("UPDATE movies SET tier_only = 1, scheduled_date = NULL WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to demote movie: {}", e))?;

    Ok(())
}

// Inverse of add_show_tier_only / add_movie_tier_only: undoes a tier-list add
// from the search modal. Semantics depend on whether the row was tracked:
//   tier_only = 1 → delete the row (it only existed for tiering)
//   tier_only = 0 → keep the row, clear tier_id/rank_order (stays tracked)
#[tauri::command]
pub async fn remove_show_from_tier_list(app: AppHandle, id: i64) -> Result<(), String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let tier_only: Option<i32> = sqlx::query_scalar("SELECT tier_only FROM shows WHERE id = ?")
        .bind(id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| format!("Failed to look up show: {}", e))?;

    match tier_only {
        Some(1) => {
            sqlx::query("DELETE FROM episodes WHERE show_id = ?")
                .bind(id)
                .execute(&pool)
                .await
                .map_err(|e| format!("Failed to delete episodes for tier-only show: {}", e))?;

            sqlx::query("DELETE FROM shows WHERE id = ?")
                .bind(id)
                .execute(&pool)
                .await
                .map_err(|e| format!("Failed to delete tier-only show: {}", e))?;
        }
        Some(0) => {
            sqlx::query("UPDATE shows SET tier_id = NULL, rank_order = NULL WHERE id = ?")
                .bind(id)
                .execute(&pool)
                .await
                .map_err(|e| format!("Failed to clear show tier: {}", e))?;
        }
        _ => {}
    }

    Ok(())
}

#[tauri::command]
pub async fn remove_movie_from_tier_list(app: AppHandle, id: i64) -> Result<(), String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let tier_only: Option<i32> = sqlx::query_scalar("SELECT tier_only FROM movies WHERE id = ?")
        .bind(id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| format!("Failed to look up movie: {}", e))?;

    match tier_only {
        Some(1) => {
            sqlx::query("DELETE FROM movies WHERE id = ?")
                .bind(id)
                .execute(&pool)
                .await
                .map_err(|e| format!("Failed to delete tier-only movie: {}", e))?;
        }
        Some(0) => {
            sqlx::query("UPDATE movies SET tier_id = NULL, rank_order = NULL WHERE id = ?")
                .bind(id)
                .execute(&pool)
                .await
                .map_err(|e| format!("Failed to clear movie tier: {}", e))?;
        }
        _ => {}
    }

    Ok(())
}

// ============================================
// Update tier assignment (replaces old rating commands)
// ============================================

#[tauri::command]
pub async fn update_show_tier(
    app: AppHandle,
    id: i64,
    tier_id: Option<i64>,
) -> Result<(), String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    sqlx::query("UPDATE shows SET tier_id = ?, rank_order = NULL WHERE id = ?")
        .bind(tier_id)
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to update show tier: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn update_movie_tier(
    app: AppHandle,
    id: i64,
    tier_id: Option<i64>,
) -> Result<(), String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    sqlx::query("UPDATE movies SET tier_id = ?, rank_order = NULL WHERE id = ?")
        .bind(tier_id)
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to update movie tier: {}", e))?;

    Ok(())
}

// ============================================
// Bulk reorder / tier reassignment commands for drag-and-drop
// ============================================

// Called per drop zone after a drag finalizes. Sets tier_id + rank_order for
// every item in ordered_ids to match its index. tier_id = None means the
// untiered section (clears both tier_id and rank_order).
#[tauri::command]
pub async fn set_tier_show_positions(
    app: AppHandle,
    tier_id: Option<i64>,
    ordered_ids: Vec<i64>,
) -> Result<(), String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let mut tx = pool.begin().await
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    for (idx, show_id) in ordered_ids.iter().enumerate() {
        let rank = idx as i32;
        match tier_id {
            Some(tid) => {
                sqlx::query("UPDATE shows SET tier_id = ?, rank_order = ? WHERE id = ?")
                    .bind(tid)
                    .bind(rank)
                    .bind(show_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| format!("Failed to set show position: {}", e))?;
            }
            None => {
                sqlx::query("UPDATE shows SET tier_id = NULL, rank_order = NULL WHERE id = ?")
                    .bind(show_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| format!("Failed to clear show position: {}", e))?;
            }
        }
    }

    tx.commit().await
        .map_err(|e| format!("Failed to commit show positions: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn set_tier_movie_positions(
    app: AppHandle,
    tier_id: Option<i64>,
    ordered_ids: Vec<i64>,
) -> Result<(), String> {
    let pool = connection::get_pool(&app).await
        .map_err(|e| format!("Database error: {}", e))?;

    let mut tx = pool.begin().await
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    for (idx, movie_id) in ordered_ids.iter().enumerate() {
        let rank = idx as i32;
        match tier_id {
            Some(tid) => {
                sqlx::query("UPDATE movies SET tier_id = ?, rank_order = ? WHERE id = ?")
                    .bind(tid)
                    .bind(rank)
                    .bind(movie_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| format!("Failed to set movie position: {}", e))?;
            }
            None => {
                sqlx::query("UPDATE movies SET tier_id = NULL, rank_order = NULL WHERE id = ?")
                    .bind(movie_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| format!("Failed to clear movie position: {}", e))?;
            }
        }
    }

    tx.commit().await
        .map_err(|e| format!("Failed to commit movie positions: {}", e))?;

    Ok(())
}

