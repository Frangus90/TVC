//! Sync orchestration: derive ceremony page titles at runtime, fetch + parse each
//! from Wikipedia, and upsert into SQLite. Resilient — a missing or failing
//! ceremony is recorded and skipped while the rest proceed.

use chrono::{DateTime, Datelike, Utc};
use serde::Serialize;
use sqlx::SqlitePool;
use tauri::AppHandle;

use crate::awards::db;
use crate::awards::models::ParsedCeremony;
use crate::awards::source::AwardType;
use crate::awards::wikipedia::{parse_wikitext, WikipediaAwardSource};
use crate::db::connection;

const FULL_YEARS: i32 = 20;
const RECENT_YEARS: i32 = 5;
const LAST_SYNC_KEY: &str = "awards_last_sync";
const AWARDS: [AwardType; 2] = [AwardType::Oscars, AwardType::Emmys];

/// Result of a sync, returned to the UI for a toast. Counts are of rows
/// *processed* this run (past ceremonies are re-processed but rarely change).
#[derive(Debug, Default, Serialize)]
pub struct SyncSummary {
    pub ceremonies: u32,
    pub categories: u32,
    pub nominees: u32,
    pub winners: u32,
    pub errors: Vec<String>,
}

/// Run a sync. `full` = re-pull 20 years of history per award; otherwise just the
/// newest few ceremonies (past ones are immutable) plus a probe for the next one.
pub async fn sync(pool: &SqlitePool, full: bool) -> SyncSummary {
    let source = WikipediaAwardSource::new();
    let mut summary = SyncSummary::default();
    let year = Utc::now().year();

    for award in AWARDS {
        let base = award.edition_for_year(year);
        // Full: 20 years of history. Otherwise: the last 5 years. Both probe one
        // edition ahead so a just-announced ceremony is picked up.
        let span = if full { FULL_YEARS } else { RECENT_YEARS };
        let editions: Vec<i32> = ((base - (span - 1))..=(base + 1)).rev().collect();

        for edition in editions {
            if edition < 1 {
                continue;
            }
            let title = award.page_title(edition);
            match source.fetch_wikitext(&title).await {
                Ok(None) => {} // page not created yet — skip silently
                Err(e) => summary.errors.push(e),
                Ok(Some(wikitext)) => match parse_wikitext(&wikitext) {
                    None => {} // no "Winners and nominees" section — skip
                    Some(parsed) => {
                        if let Err(e) =
                            persist(pool, award, edition, &title, &parsed, &mut summary).await
                        {
                            summary.errors.push(format!("{title}: {e}"));
                        }
                    }
                },
            }
            // Be polite to the MediaWiki API.
            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        }
    }

    let _ = db::set_setting(pool, LAST_SYNC_KEY, &Utc::now().to_rfc3339()).await;
    summary
}

async fn persist(
    pool: &SqlitePool,
    award: AwardType,
    edition: i32,
    title: &str,
    parsed: &ParsedCeremony,
    summary: &mut SyncSummary,
) -> Result<(), String> {
    let year = award.year_for_edition(edition);
    // Predictable only if nominations are out (no winners yet) AND the ceremony is
    // still in the future. A ceremony that's already happened is "past" even if
    // Wikipedia hasn't filled in its winners yet. ISO date strings compare
    // chronologically, so a lexical compare is correct.
    let today = Utc::now().date_naive().to_string();
    let current_year = Utc::now().year();
    let status = if parsed.has_winners {
        "past"
    } else if parsed.categories.is_empty() || year < current_year {
        // Nothing parseable to predict, or an edition from a past year — never
        // predictable, regardless of missing winner markers.
        "past"
    } else if let Some(ref d) = parsed.ceremony_date {
        if d.as_str() >= today.as_str() {
            "nominated"
        } else {
            "past"
        }
    } else {
        "nominated"
    };
    // `name` and `wiki_title` are both the page title (e.g. "97th Academy Awards").
    let ceremony_id = db::upsert_ceremony(
        pool,
        award.as_str(),
        edition,
        title,
        year,
        parsed.ceremony_date.as_deref(),
        status,
        title,
    )
    .await?;
    summary.ceremonies += 1;

    for cat in &parsed.categories {
        let cat_id = db::upsert_category(pool, ceremony_id, &cat.name, cat.display_order).await?;
        summary.categories += 1;
        let mut keys = Vec::with_capacity(cat.nominees.len());
        for nom in &cat.nominees {
            let is_winner = nom.is_winner.map(|w| if w { 1_i64 } else { 0_i64 });
            db::upsert_nominee(
                pool,
                cat_id,
                &nom.title,
                nom.detail.as_deref(),
                is_winner,
                &nom.source_key,
            )
            .await?;
            keys.push(nom.source_key.clone());
            summary.nominees += 1;
            if nom.is_winner == Some(true) {
                summary.winners += 1;
            }
        }
        // Drop rows from a previous sync that this parse no longer produces
        // (old-format/uncleaned entries) so re-syncing can't leave duplicates.
        db::delete_nominees_not_in(pool, cat_id, &keys).await?;
    }
    Ok(())
}

/// Background sync on startup: full backfill if the DB is empty, otherwise a light
/// incremental refresh, throttled to at most once per 12h.
pub async fn auto_sync_on_startup(app: AppHandle) {
    let pool = match connection::get_pool(&app).await {
        Ok(p) => p,
        Err(_) => return,
    };
    let have = db::count_ceremonies(&pool).await;
    let recently_synced = db::get_setting(&pool, LAST_SYNC_KEY)
        .await
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
        .map(|t| Utc::now().signed_duration_since(t.with_timezone(&Utc)).num_hours() < 12)
        .unwrap_or(false);

    if have > 0 && recently_synced {
        return;
    }
    // Startup always pulls the recent window (last 5 years); a full 20-year
    // backfill is only done on an explicit "Full refresh".
    let _ = sync(&pool, false).await;
}

#[cfg(test)]
mod it_tests {
    //! Live integration test (network). Run explicitly:
    //!   cargo test --lib awards::sync::it_tests -- --ignored --nocapture
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    const SCHEMA: &str = "
        CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT);
        CREATE TABLE award_ceremonies (id INTEGER PRIMARY KEY AUTOINCREMENT, award_type TEXT NOT NULL,
            edition INTEGER NOT NULL, name TEXT NOT NULL, year INTEGER NOT NULL, ceremony_date TEXT,
            status TEXT NOT NULL, wiki_title TEXT NOT NULL, last_synced TEXT, UNIQUE(award_type, edition));
        CREATE TABLE award_categories (id INTEGER PRIMARY KEY AUTOINCREMENT,
            ceremony_id INTEGER NOT NULL, name TEXT NOT NULL, display_order INTEGER, UNIQUE(ceremony_id, name));
        CREATE TABLE award_nominees (id INTEGER PRIMARY KEY AUTOINCREMENT, category_id INTEGER NOT NULL,
            title TEXT NOT NULL, detail TEXT, is_winner INTEGER, source_key TEXT NOT NULL,
            UNIQUE(category_id, source_key));
    ";

    async fn mem_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        for stmt in SCHEMA.split(';').filter(|s| !s.trim().is_empty()) {
            sqlx::query(stmt).execute(&pool).await.unwrap();
        }
        pool
    }

    #[tokio::test]
    #[ignore = "hits the live Wikipedia API"]
    async fn incremental_sync_populates_db() {
        let pool = mem_pool().await;
        let summary = sync(&pool, false).await;
        println!(
            "sync: ceremonies={} categories={} nominees={} winners={} errors={:?}",
            summary.ceremonies, summary.categories, summary.nominees, summary.winners, summary.errors
        );
        assert!(summary.ceremonies >= 2, "expected recent ceremonies");
        assert!(summary.winners > 0, "recent past ceremonies should have winners");

        let oscars = db::get_ceremonies(&pool, "oscars").await.unwrap();
        assert!(!oscars.is_empty(), "oscars ceremonies stored");
        let detail = db::get_ceremony_detail(&pool, oscars[0].id).await.unwrap();
        assert!(!detail.categories.is_empty(), "ceremony has categories");

        // Date-aware status: a ceremony is only predictable ("nominated") when its
        // nominations are out and the ceremony is still in the future.
        let emmys = db::get_ceremonies(&pool, "emmys").await.unwrap();
        let open = |cs: &[db::CeremonySummary]| {
            cs.iter()
                .filter(|c| c.status != "past")
                .map(|c| format!("{} ({})", c.name, c.ceremony_date.clone().unwrap_or_default()))
                .collect::<Vec<_>>()
        };
        println!("predictable oscars: {:?}", open(&oscars));
        println!("predictable emmys: {:?}", open(&emmys));
    }
}
