//! SQLite persistence for awards data: idempotent upserts used by the sync
//! orchestrator, and read queries used by the Tauri commands. Mirrors the
//! row-mapping style in `racing/mod.rs`.

use serde::Serialize;
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;

use crate::awards::scoring::score_predictions;

#[derive(Debug, Serialize)]
pub struct CeremonySummary {
    pub id: i64,
    pub award_type: String,
    pub edition: i64,
    pub name: String,
    pub year: i64,
    pub ceremony_date: Option<String>,
    pub nominations_date: Option<String>,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct NomineeRow {
    pub id: i64,
    pub title: String,
    pub detail: Option<String>,
    /// `Some(true)` won, `Some(false)` lost, `None` = not yet announced.
    pub is_winner: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct CategoryDetail {
    pub id: i64,
    pub name: String,
    pub nominees: Vec<NomineeRow>,
}

#[derive(Debug, Serialize)]
pub struct CeremonyDetail {
    #[serde(flatten)]
    pub ceremony: CeremonySummary,
    pub categories: Vec<CategoryDetail>,
}

fn to_opt_bool(v: Option<i64>) -> Option<bool> {
    v.map(|x| x != 0)
}

pub async fn upsert_ceremony(
    pool: &SqlitePool,
    award_type: &str,
    edition: i32,
    name: &str,
    year: i32,
    ceremony_date: Option<&str>,
    nominations_date: Option<&str>,
    status: &str,
    wiki_title: &str,
) -> Result<i64, String> {
    sqlx::query(
        "INSERT INTO award_ceremonies (award_type, edition, name, year, ceremony_date, nominations_date, status, wiki_title, last_synced)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))
         ON CONFLICT(award_type, edition) DO UPDATE SET
             name = excluded.name, year = excluded.year, ceremony_date = excluded.ceremony_date,
             nominations_date = excluded.nominations_date, status = excluded.status,
             wiki_title = excluded.wiki_title, last_synced = excluded.last_synced",
    )
    .bind(award_type)
    .bind(edition)
    .bind(name)
    .bind(year)
    .bind(ceremony_date)
    .bind(nominations_date)
    .bind(status)
    .bind(wiki_title)
    .execute(pool)
    .await
    .map_err(|e| format!("upsert ceremony: {e}"))?;

    sqlx::query_scalar("SELECT id FROM award_ceremonies WHERE award_type = ? AND edition = ?")
        .bind(award_type)
        .bind(edition)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("fetch ceremony id: {e}"))
}

pub async fn upsert_category(
    pool: &SqlitePool,
    ceremony_id: i64,
    name: &str,
    display_order: i64,
) -> Result<i64, String> {
    sqlx::query(
        "INSERT INTO award_categories (ceremony_id, name, display_order)
         VALUES (?, ?, ?)
         ON CONFLICT(ceremony_id, name) DO UPDATE SET display_order = excluded.display_order",
    )
    .bind(ceremony_id)
    .bind(name)
    .bind(display_order)
    .execute(pool)
    .await
    .map_err(|e| format!("upsert category: {e}"))?;

    sqlx::query_scalar("SELECT id FROM award_categories WHERE ceremony_id = ? AND name = ?")
        .bind(ceremony_id)
        .bind(name)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("fetch category id: {e}"))
}

pub async fn upsert_nominee(
    pool: &SqlitePool,
    category_id: i64,
    title: &str,
    detail: Option<&str>,
    is_winner: Option<i64>,
    source_key: &str,
) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO award_nominees (category_id, title, detail, is_winner, source_key)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(category_id, source_key) DO UPDATE SET
             title = excluded.title, detail = excluded.detail, is_winner = excluded.is_winner",
    )
    .bind(category_id)
    .bind(title)
    .bind(detail)
    .bind(is_winner)
    .bind(source_key)
    .execute(pool)
    .await
    .map_err(|e| format!("upsert nominee: {e}"))?;
    Ok(())
}

/// Remove a category's nominees whose `source_key` is not in `keep` — i.e. rows
/// left over from a previous sync (stale/renamed/old-format entries). Rows that
/// match are left in place by the caller's upsert, so their ids (and any
/// predictions pointing at them) survive.
pub async fn delete_nominees_not_in(
    pool: &SqlitePool,
    category_id: i64,
    keep: &[String],
) -> Result<(), String> {
    if keep.is_empty() {
        sqlx::query("DELETE FROM award_nominees WHERE category_id = ?")
            .bind(category_id)
            .execute(pool)
            .await
            .map_err(|e| format!("prune nominees: {e}"))?;
        return Ok(());
    }
    let placeholders = std::iter::repeat("?").take(keep.len()).collect::<Vec<_>>().join(",");
    let sql = format!(
        "DELETE FROM award_nominees WHERE category_id = ? AND source_key NOT IN ({placeholders})"
    );
    let mut q = sqlx::query(&sql).bind(category_id);
    for k in keep {
        q = q.bind(k);
    }
    q.execute(pool)
        .await
        .map_err(|e| format!("prune nominees: {e}"))?;
    Ok(())
}

pub async fn get_ceremonies(
    pool: &SqlitePool,
    award_type: &str,
) -> Result<Vec<CeremonySummary>, String> {
    let rows = sqlx::query(
        "SELECT id, award_type, edition, name, year, ceremony_date, nominations_date, status
         FROM award_ceremonies WHERE award_type = ? ORDER BY edition DESC",
    )
    .bind(award_type)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("get ceremonies: {e}"))?;

    Ok(rows
        .into_iter()
        .map(|r| CeremonySummary {
            id: r.get("id"),
            award_type: r.get("award_type"),
            edition: r.get("edition"),
            name: r.get("name"),
            year: r.get("year"),
            ceremony_date: r.get("ceremony_date"),
            nominations_date: r.get("nominations_date"),
            status: r.get("status"),
        })
        .collect())
}

pub async fn get_ceremony_detail(
    pool: &SqlitePool,
    ceremony_id: i64,
) -> Result<CeremonyDetail, String> {
    let cr = sqlx::query(
        "SELECT id, award_type, edition, name, year, ceremony_date, nominations_date, status
         FROM award_ceremonies WHERE id = ?",
    )
    .bind(ceremony_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("get ceremony: {e}"))?
    .ok_or_else(|| format!("ceremony {ceremony_id} not found"))?;

    let ceremony = CeremonySummary {
        id: cr.get("id"),
        award_type: cr.get("award_type"),
        edition: cr.get("edition"),
        name: cr.get("name"),
        year: cr.get("year"),
        ceremony_date: cr.get("ceremony_date"),
        nominations_date: cr.get("nominations_date"),
        status: cr.get("status"),
    };

    let cat_rows = sqlx::query(
        "SELECT id, name FROM award_categories WHERE ceremony_id = ? ORDER BY display_order, id",
    )
    .bind(ceremony_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("get categories: {e}"))?;

    let mut categories = Vec::with_capacity(cat_rows.len());
    for cat in cat_rows {
        let cat_id: i64 = cat.get("id");
        let nom_rows = sqlx::query(
            "SELECT id, title, detail, is_winner FROM award_nominees
             WHERE category_id = ? ORDER BY is_winner DESC, id",
        )
        .bind(cat_id)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("get nominees: {e}"))?;

        let nominees = nom_rows
            .into_iter()
            .map(|n| NomineeRow {
                id: n.get("id"),
                title: n.get("title"),
                detail: n.get("detail"),
                is_winner: to_opt_bool(n.get::<Option<i64>, _>("is_winner")),
            })
            .collect();

        categories.push(CategoryDetail {
            id: cat_id,
            name: cat.get("name"),
            nominees,
        });
    }

    Ok(CeremonyDetail {
        ceremony,
        categories,
    })
}

pub async fn count_ceremonies(pool: &SqlitePool) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM award_ceremonies")
        .fetch_one(pool)
        .await
        .unwrap_or(0)
}

pub async fn set_setting(pool: &SqlitePool, key: &str, value: &str) -> Result<(), String> {
    sqlx::query("INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)")
        .bind(key)
        .bind(value)
        .execute(pool)
        .await
        .map_err(|e| format!("set setting: {e}"))?;
    Ok(())
}

pub async fn get_setting(pool: &SqlitePool, key: &str) -> Option<String> {
    sqlx::query_scalar("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
}

#[derive(Debug, Serialize)]
pub struct CategoryPick {
    pub category_id: i64,
    pub nominee_id: i64,
}

#[derive(Debug, Serialize)]
pub struct PredictionResults {
    /// The user's pick per category (only categories they've picked).
    pub picks: Vec<CategoryPick>,
    /// Correct picks among categories whose winner is known.
    pub correct: u32,
    /// Categories that had both a pick and a revealed winner (i.e. scored).
    pub total: u32,
}

/// Set (or replace) the user's pick for a category.
pub async fn set_prediction(
    pool: &SqlitePool,
    category_id: i64,
    nominee_id: i64,
) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO award_predictions (category_id, nominee_id, created_at, updated_at)
         VALUES (?, ?, datetime('now'), datetime('now'))
         ON CONFLICT(category_id) DO UPDATE SET
             nominee_id = excluded.nominee_id, updated_at = datetime('now')",
    )
    .bind(category_id)
    .bind(nominee_id)
    .execute(pool)
    .await
    .map_err(|e| format!("set prediction: {e}"))?;
    Ok(())
}

/// Remove the user's pick for a category.
pub async fn clear_prediction(pool: &SqlitePool, category_id: i64) -> Result<(), String> {
    sqlx::query("DELETE FROM award_predictions WHERE category_id = ?")
        .bind(category_id)
        .execute(pool)
        .await
        .map_err(|e| format!("clear prediction: {e}"))?;
    Ok(())
}

/// The user's picks for one ceremony plus their score against revealed winners.
pub async fn get_prediction_results(
    pool: &SqlitePool,
    ceremony_id: i64,
) -> Result<PredictionResults, String> {
    let pick_rows = sqlx::query(
        "SELECT p.category_id AS category_id, p.nominee_id AS nominee_id
         FROM award_predictions p
         JOIN award_categories c ON c.id = p.category_id
         WHERE c.ceremony_id = ?",
    )
    .bind(ceremony_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("get predictions: {e}"))?;

    let picks: Vec<CategoryPick> = pick_rows
        .into_iter()
        .map(|r| CategoryPick {
            category_id: r.get("category_id"),
            nominee_id: r.get("nominee_id"),
        })
        .collect();

    let winner_rows = sqlx::query(
        "SELECT n.category_id AS category_id, n.id AS nominee_id
         FROM award_nominees n
         JOIN award_categories c ON c.id = n.category_id
         WHERE c.ceremony_id = ? AND n.is_winner = 1",
    )
    .bind(ceremony_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("get winners: {e}"))?;

    let winners: HashMap<i64, i64> = winner_rows
        .into_iter()
        .map(|r| (r.get::<i64, _>("category_id"), r.get::<i64, _>("nominee_id")))
        .collect();

    let pick_pairs: Vec<(i64, i64)> = picks.iter().map(|p| (p.category_id, p.nominee_id)).collect();
    let (correct, total) = score_predictions(&pick_pairs, &winners);

    Ok(PredictionResults {
        picks,
        correct,
        total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    const SCHEMA: &str = "
        CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT);
        CREATE TABLE award_ceremonies (id INTEGER PRIMARY KEY AUTOINCREMENT, award_type TEXT NOT NULL,
            edition INTEGER NOT NULL, name TEXT NOT NULL, year INTEGER NOT NULL, ceremony_date TEXT,
            nominations_date TEXT, status TEXT NOT NULL, wiki_title TEXT NOT NULL, last_synced TEXT, UNIQUE(award_type, edition));
        CREATE TABLE award_categories (id INTEGER PRIMARY KEY AUTOINCREMENT, ceremony_id INTEGER NOT NULL,
            name TEXT NOT NULL, display_order INTEGER, UNIQUE(ceremony_id, name));
        CREATE TABLE award_nominees (id INTEGER PRIMARY KEY AUTOINCREMENT, category_id INTEGER NOT NULL,
            title TEXT NOT NULL, detail TEXT, is_winner INTEGER, source_key TEXT NOT NULL,
            UNIQUE(category_id, source_key));
        CREATE TABLE award_predictions (id INTEGER PRIMARY KEY AUTOINCREMENT, category_id INTEGER NOT NULL,
            nominee_id INTEGER NOT NULL, created_at TEXT, updated_at TEXT, UNIQUE(category_id));
    ";

    async fn setup() -> (SqlitePool, i64, i64, i64, i64) {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        for stmt in SCHEMA.split(';').filter(|s| !s.trim().is_empty()) {
            sqlx::query(stmt).execute(&pool).await.unwrap();
        }
        let cer = upsert_ceremony(&pool, "oscars", 97, "97th Academy Awards", 2025, None, None, "past", "97th Academy Awards")
            .await
            .unwrap();
        let cat = upsert_category(&pool, cer, "Best Picture", 0).await.unwrap();
        upsert_nominee(&pool, cat, "Anora", None, Some(1), "anora").await.unwrap();
        upsert_nominee(&pool, cat, "Conclave", None, Some(0), "conclave").await.unwrap();
        let winner: i64 = sqlx::query_scalar("SELECT id FROM award_nominees WHERE source_key='anora'")
            .fetch_one(&pool).await.unwrap();
        let loser: i64 = sqlx::query_scalar("SELECT id FROM award_nominees WHERE source_key='conclave'")
            .fetch_one(&pool).await.unwrap();
        (pool, cer, cat, winner, loser)
    }

    #[tokio::test]
    async fn prediction_scoring_roundtrip() {
        let (pool, cer, cat, winner, loser) = setup().await;

        // No picks yet.
        let r = get_prediction_results(&pool, cer).await.unwrap();
        assert_eq!((r.correct, r.total, r.picks.len()), (0, 0, 0));

        // Pick the eventual winner.
        set_prediction(&pool, cat, winner).await.unwrap();
        let r = get_prediction_results(&pool, cer).await.unwrap();
        assert_eq!((r.correct, r.total, r.picks.len()), (1, 1, 1));

        // Change the pick to a loser (upsert replaces).
        set_prediction(&pool, cat, loser).await.unwrap();
        let r = get_prediction_results(&pool, cer).await.unwrap();
        assert_eq!((r.correct, r.total), (0, 1));

        // Clear the pick.
        clear_prediction(&pool, cat).await.unwrap();
        let r = get_prediction_results(&pool, cer).await.unwrap();
        assert_eq!((r.correct, r.total, r.picks.len()), (0, 0, 0));

        // Detail read still works and orders the winner first.
        let detail = get_ceremony_detail(&pool, cer).await.unwrap();
        assert_eq!(detail.categories.len(), 1);
        assert_eq!(detail.categories[0].nominees[0].is_winner, Some(true));
    }

    #[tokio::test]
    async fn resync_prunes_stale_nominees_but_keeps_predictions() {
        let (pool, cer, cat, winner, _loser) = setup().await;
        // A leftover row from an earlier parse (different, "dirty" source_key).
        upsert_nominee(&pool, cat, "Anora stale", None, Some(0), "anora-stale")
            .await
            .unwrap();
        // The user's pick points at the good winner row.
        set_prediction(&pool, cat, winner).await.unwrap();

        // Re-sync keeps only the current source_keys and drops the stale one.
        delete_nominees_not_in(&pool, cat, &["anora".into(), "conclave".into()])
            .await
            .unwrap();

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM award_nominees WHERE category_id = ?")
            .bind(cat)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 2, "stale row pruned, current rows kept");

        // Prediction on the surviving winner row is intact.
        let r = get_prediction_results(&pool, cer).await.unwrap();
        assert_eq!((r.correct, r.total), (1, 1));
    }
}
