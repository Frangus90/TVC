//! SQLite persistence for awards data: idempotent upserts used by the sync
//! orchestrator, and read queries used by the Tauri commands. Mirrors the
//! row-mapping style in `racing/mod.rs`.

use serde::Serialize;
use sqlx::{Row, SqlitePool};

#[derive(Debug, Serialize)]
pub struct CeremonySummary {
    pub id: i64,
    pub award_type: String,
    pub edition: i64,
    pub name: String,
    pub year: i64,
    pub ceremony_date: Option<String>,
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
    status: &str,
    wiki_title: &str,
) -> Result<i64, String> {
    sqlx::query(
        "INSERT INTO award_ceremonies (award_type, edition, name, year, status, wiki_title, last_synced)
         VALUES (?, ?, ?, ?, ?, ?, datetime('now'))
         ON CONFLICT(award_type, edition) DO UPDATE SET
             name = excluded.name, year = excluded.year, status = excluded.status,
             wiki_title = excluded.wiki_title, last_synced = excluded.last_synced",
    )
    .bind(award_type)
    .bind(edition)
    .bind(name)
    .bind(year)
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

pub async fn get_ceremonies(
    pool: &SqlitePool,
    award_type: &str,
) -> Result<Vec<CeremonySummary>, String> {
    let rows = sqlx::query(
        "SELECT id, award_type, edition, name, year, ceremony_date, status
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
            status: r.get("status"),
        })
        .collect())
}

pub async fn get_ceremony_detail(
    pool: &SqlitePool,
    ceremony_id: i64,
) -> Result<CeremonyDetail, String> {
    let cr = sqlx::query(
        "SELECT id, award_type, edition, name, year, ceremony_date, status
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
