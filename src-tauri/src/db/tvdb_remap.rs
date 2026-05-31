// Live TVDB -> TMDB remap runner. Runs once at app startup (Phase 3 of the
// migration plan) and re-invoked by the resolver UI for manual fixes.
//
// Flow per run:
//   1. Bail if settings.tvdb_to_tmdb_migration_complete = '1'.
//   2. Crash-recovery pre-pass for rows rewritten previously but missing the
//      episode INSERT (process killed mid-tx). Re-run the episode portion.
//   3. Load all WHERE unmigrated = 1 AND id > 0, emit `tvdb_migration_started`.
//   4. Concurrency-bounded loop (Semaphore(8), 80 ms sleep, 429-friendly):
//        a. tmdb::find_tv_by_tvdb_id(legacy_tvdb_id)
//        b. If mapped -> remap_single_show
//        c. Emit `tvdb_migration_progress` after each.
//   5. Emit `tvdb_migration_finished`. Set complete=1 only if no transient
//      errors are left (quarantined rows are an acceptable terminal state).

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use serde::Serialize;
use sqlx::{Row, SqlitePool, Transaction, Sqlite};
use tauri::{AppHandle, Emitter};
use tokio::sync::{Mutex, Semaphore};

use crate::db::connection;
use crate::tmdb;

/// Per-show snapshot of user-edited episode state, keyed by (season, episode).
#[derive(Debug, Clone, Default)]
struct UserEpisodeState {
    watched: i64,
    watched_at: Option<String>,
    scheduled_date: Option<String>,
    rating: Option<i64>,
    tags: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemapOutcome {
    pub episodes_orphaned: usize,
    pub merged_with: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
struct PerShowResult {
    name: String,
    new_tmdb_id: i64,
    episodes_orphaned: usize,
    merged_with: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Default)]
struct FinishedEvent {
    mapped: usize,
    quarantined: usize,
    errors: Vec<String>,
    per_show: Vec<PerShowResult>,
}

#[derive(Debug, Clone, Serialize)]
struct StartedEvent {
    total: usize,
}

#[derive(Debug, Clone, Serialize)]
struct ProgressEvent {
    done: usize,
    total: usize,
    current_name: String,
}

/// Entry point called from `lib.rs` `setup`.
pub async fn run_migration_if_needed(app: AppHandle) {
    let pool = match connection::get_pool(&app).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[tvdb_remap] failed to acquire pool: {}", e);
            return;
        }
    };

    // tauri-plugin-sql applies migrations only when the JS side calls
    // Database.load(); this runner is spawned during Tauri setup and races
    // against that. Wait for the column migration 015 adds before doing
    // anything that depends on it.
    if !wait_for_schema(&pool).await {
        eprintln!("[tvdb_remap] migration 015 did not apply within timeout; will retry on next launch");
        return;
    }

    let complete: Option<String> = sqlx::query_scalar(
        "SELECT value FROM settings WHERE key = 'tvdb_to_tmdb_migration_complete'",
    )
    .fetch_optional(&pool)
    .await
    .unwrap_or(None);

    // Self-heal: a prior buggy run (before wait_for_schema existed) could have
    // set complete='1' without doing anything. If unmigrated rows are still
    // present, clear the flag so we re-run.
    if complete.as_deref() == Some("1") {
        let pending_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM shows WHERE unmigrated = 1 AND id > 0",
        )
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

        if pending_count == 0 {
            return;
        }

        eprintln!(
            "[tvdb_remap] clearing stale completion flag; {} unmigrated row(s) still pending",
            pending_count
        );
        let _ = sqlx::query(
            "INSERT OR REPLACE INTO settings (key, value) \
             VALUES ('tvdb_to_tmdb_migration_complete', '0')",
        )
        .execute(&pool)
        .await;
    }

    let pending: Vec<(i64, i64, String)> = match sqlx::query_as(
        "SELECT id, COALESCE(legacy_tvdb_id, id), name FROM shows \
         WHERE unmigrated = 1 AND id > 0",
    )
    .fetch_all(&pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("[tvdb_remap] failed to read pending shows: {}", e);
            return;
        }
    };

    let total = pending.len();

    if total == 0 {
        let _ = mark_complete(&pool).await;
        return;
    }

    let _ = app.emit("tvdb_migration_started", StartedEvent { total });

    let semaphore = Arc::new(Semaphore::new(8));
    let progress = Arc::new(AtomicUsize::new(0));
    let results: Arc<Mutex<FinishedEvent>> = Arc::new(Mutex::new(FinishedEvent::default()));

    let mut handles = Vec::with_capacity(total);
    for (current_id, tvdb_id, name) in pending {
        let semaphore = semaphore.clone();
        let pool = pool.clone();
        let app = app.clone();
        let progress = progress.clone();
        let results = results.clone();

        handles.push(tokio::spawn(async move {
            let _permit = match semaphore.acquire_owned().await {
                Ok(p) => p,
                Err(_) => return,
            };
            // Gentle pacing for TMDB.
            tokio::time::sleep(Duration::from_millis(80)).await;

            let outcome = process_one_show(&pool, current_id, tvdb_id, &name).await;

            {
                let mut r = results.lock().await;
                match &outcome {
                    ProcessResult::Mapped { new_tmdb_id, outcome } => {
                        r.mapped += 1;
                        r.per_show.push(PerShowResult {
                            name: name.clone(),
                            new_tmdb_id: *new_tmdb_id,
                            episodes_orphaned: outcome.episodes_orphaned,
                            merged_with: outcome.merged_with,
                        });
                    }
                    ProcessResult::Quarantined => {
                        r.quarantined += 1;
                    }
                    ProcessResult::Errored(msg) => {
                        r.errors.push(format!("{}: {}", name, msg));
                    }
                }
            }

            let done = progress.fetch_add(1, Ordering::SeqCst) + 1;
            let _ = app.emit(
                "tvdb_migration_progress",
                ProgressEvent {
                    done,
                    total,
                    current_name: name,
                },
            );
        }));
    }

    for h in handles {
        let _ = h.await;
    }

    let finished = {
        // Replace contents to avoid Arc::try_unwrap edge cases if a handle leaked.
        let mut guard = results.lock().await;
        std::mem::take(&mut *guard)
    };

    if finished.errors.is_empty() {
        if let Err(e) = mark_complete(&pool).await {
            eprintln!("[tvdb_remap] failed to mark complete: {}", e);
        }
    }

    let _ = app.emit("tvdb_migration_finished", finished);
}

/// Poll for `shows.legacy_tvdb_id` (added by migration 015). Returns true once
/// the column exists, false if it never appears within the timeout (~60s).
async fn wait_for_schema(pool: &SqlitePool) -> bool {
    for attempt in 0..120 {
        let exists: Option<(i64,)> = sqlx::query_as(
            "SELECT 1 FROM pragma_table_info('shows') WHERE name = 'legacy_tvdb_id'",
        )
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();
        if exists.is_some() {
            return true;
        }
        if attempt == 0 {
            eprintln!("[tvdb_remap] waiting for migration 015 to apply...");
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    false
}

enum ProcessResult {
    Mapped {
        new_tmdb_id: i64,
        outcome: RemapOutcome,
    },
    Quarantined,
    Errored(String),
}

async fn process_one_show(
    pool: &SqlitePool,
    current_id: i64,
    tvdb_id: i64,
    _name: &str,
) -> ProcessResult {
    let new_tmdb_id = match tmdb::find_tv_by_tvdb_id(tvdb_id).await {
        Ok(Some(id)) => id,
        Ok(None) => return ProcessResult::Quarantined,
        Err(e) => return ProcessResult::Errored(format!("tmdb /find: {}", e)),
    };

    match remap_single_show(pool, current_id, new_tmdb_id).await {
        Ok(outcome) => ProcessResult::Mapped {
            new_tmdb_id,
            outcome,
        },
        Err(e) => ProcessResult::Errored(format!("remap: {}", e)),
    }
}

/// Public so the Phase 4 resolver UI can re-run remap for manually picked IDs.
/// `old_id` is whatever currently lives in `shows.id` (the TVDB id for an
/// unmigrated row).
pub async fn remap_single_show(
    pool: &SqlitePool,
    old_id: i64,
    new_tmdb_id: i64,
) -> Result<RemapOutcome, String> {
    // Fetch all TMDB data OUTSIDE the transaction; network in a tx is bad.
    let details = tmdb::get_tv_details(new_tmdb_id)
        .await
        .map_err(|e| format!("get_tv_details: {}", e))?;
    let tmdb_eps = tmdb::get_tv_episodes(new_tmdb_id)
        .await
        .map_err(|e| format!("get_tv_episodes: {}", e))?;

    // Snapshot existing user state and episode IDs keyed by (season, ep).
    let (user_state, old_eps_by_key) = snapshot_user_state(pool, old_id)
        .await
        .map_err(|e| format!("snapshot: {}", e))?;

    let mut tx = pool.begin().await.map_err(|e| format!("begin: {}", e))?;

    // Defer FK validation to COMMIT — we'll temporarily orphan FK targets while
    // rewriting the show PK.
    sqlx::query("PRAGMA defer_foreign_keys = ON")
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("defer_foreign_keys: {}", e))?;

    // Detect a collision with a pre-existing TMDB row.
    let collision: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM shows WHERE id = ? AND legacy_tvdb_id IS NULL AND unmigrated = 0",
    )
    .bind(new_tmdb_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("collision check: {}", e))?;

    let outcome = if collision.is_some() && old_id != new_tmdb_id {
        merge_into_existing(&mut tx, old_id, new_tmdb_id, &user_state, &old_eps_by_key)
            .await
            .map_err(|e| format!("merge: {}", e))?
    } else {
        rewrite_pk_in_place(
            &mut tx,
            old_id,
            new_tmdb_id,
            &details,
            &tmdb_eps,
            &user_state,
            &old_eps_by_key,
        )
        .await
        .map_err(|e| format!("rewrite: {}", e))?
    };

    tx.commit().await.map_err(|e| format!("commit: {}", e))?;
    Ok(outcome)
}

async fn snapshot_user_state(
    pool: &SqlitePool,
    old_id: i64,
) -> Result<(HashMap<(i32, i32), UserEpisodeState>, HashMap<(i32, i32), i64>), sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, season_number, episode_number, watched, watched_at, \
         scheduled_date, rating, tags FROM episodes WHERE show_id = ?",
    )
    .bind(old_id)
    .fetch_all(pool)
    .await?;

    let mut user_state: HashMap<(i32, i32), UserEpisodeState> = HashMap::new();
    let mut old_eps_by_key: HashMap<(i32, i32), i64> = HashMap::new();

    for row in rows {
        let id: i64 = row.try_get("id")?;
        let season: Option<i32> = row.try_get("season_number")?;
        let episode: Option<i32> = row.try_get("episode_number")?;
        let (Some(season), Some(episode)) = (season, episode) else {
            continue;
        };
        let key = (season, episode);

        old_eps_by_key.insert(key, id);

        let watched: i64 = row.try_get("watched").unwrap_or(0);
        let watched_at: Option<String> = row.try_get("watched_at").ok().flatten();
        let scheduled_date: Option<String> = row.try_get("scheduled_date").ok().flatten();
        let rating: Option<i64> = row.try_get("rating").ok().flatten();
        let tags: Option<String> = row.try_get("tags").ok().flatten();

        // Only keep entries that carry actual state. Saves work in the merge
        // path; in the rewrite path, missing keys default to a fresh row.
        if watched != 0
            || watched_at.is_some()
            || scheduled_date.is_some()
            || rating.is_some()
            || tags.is_some()
        {
            user_state.insert(
                key,
                UserEpisodeState {
                    watched,
                    watched_at,
                    scheduled_date,
                    rating,
                    tags,
                },
            );
        }
    }

    Ok((user_state, old_eps_by_key))
}

#[allow(clippy::too_many_arguments)]
async fn rewrite_pk_in_place(
    tx: &mut Transaction<'_, Sqlite>,
    old_id: i64,
    new_id: i64,
    details: &tmdb::TvShowDetails,
    tmdb_eps: &[tmdb::TvEpisode],
    user_state: &HashMap<(i32, i32), UserEpisodeState>,
    old_eps_by_key: &HashMap<(i32, i32), i64>,
) -> Result<RemapOutcome, sqlx::Error> {
    let status_name = details.status.clone();
    let poster = details.poster_url();
    let first_aired = details.first_air_date.clone();
    let network = details.network_name();
    let overview = details.overview.clone();
    let runtime = details.runtime();

    // Rewrite the show PK + metadata. If old_id == new_id we still need to
    // update the metadata + flip the unmigrated flag.
    if old_id == new_id {
        // COALESCE so we don't clobber a real TVDB id (set by v1.0 backup
        // import) when refreshing a row whose TMDB id happens to equal the
        // TVDB id. Live-migration same-id rows have legacy_tvdb_id IS NULL,
        // so the COALESCE picks up `old_id`; v1.0-import same-id rows keep
        // the original TVDB id they were stored with.
        sqlx::query(
            "UPDATE shows SET legacy_tvdb_id = COALESCE(legacy_tvdb_id, ?), unmigrated = 0, \
             name = ?, poster_url = ?, status = ?, first_aired = ?, \
             network = ?, overview = ?, runtime = ?, \
             slug = NULL, airs_time = NULL, airs_days = NULL, \
             last_synced = datetime('now') WHERE id = ?",
        )
        .bind(old_id)
        .bind(&details.name)
        .bind(&poster)
        .bind(&status_name)
        .bind(&first_aired)
        .bind(&network)
        .bind(&overview)
        .bind(runtime)
        .bind(old_id)
        .execute(&mut **tx)
        .await?;
    } else {
        sqlx::query(
            "UPDATE shows SET id = ?, legacy_tvdb_id = ?, unmigrated = 0, \
             name = ?, poster_url = ?, status = ?, first_aired = ?, \
             network = ?, overview = ?, runtime = ?, \
             slug = NULL, airs_time = NULL, airs_days = NULL, \
             last_synced = datetime('now') WHERE id = ?",
        )
        .bind(new_id)
        .bind(old_id)
        .bind(&details.name)
        .bind(&poster)
        .bind(&status_name)
        .bind(&first_aired)
        .bind(&network)
        .bind(&overview)
        .bind(runtime)
        .bind(old_id)
        .execute(&mut **tx)
        .await?;

        // Cascade show_id rewrites on tables we keep.
        for sql in [
            "UPDATE episodes SET show_id = ? WHERE show_id = ?",
            "UPDATE sonarr_imports SET show_id = ? WHERE show_id = ?",
        ] {
            sqlx::query(sql)
                .bind(new_id)
                .bind(old_id)
                .execute(&mut **tx)
                .await?;
        }

        // Drop cached cast/crew — they were keyed to the old TVDB show and
        // carry stale person/character data. Next Fetch Cast pulls fresh TMDB.
        sqlx::query("DELETE FROM cast_members WHERE show_id = ?")
            .bind(old_id)
            .execute(&mut **tx)
            .await?;
        sqlx::query("DELETE FROM crew_members WHERE show_id = ?")
            .bind(old_id)
            .execute(&mut **tx)
            .await?;

        // Historically-overlooked tables that also carry show ids.
        sqlx::query(
            "UPDATE title_mappings SET tvc_id = ? \
             WHERE media_type = 'show' AND tvc_id = ?",
        )
        .bind(new_id)
        .bind(old_id)
        .execute(&mut **tx)
        .await?;

        sqlx::query(
            "UPDATE plex_scrobble_log SET matched_entity_id = ? \
             WHERE matched_entity_type = 'show' AND matched_entity_id = ?",
        )
        .bind(new_id)
        .bind(old_id)
        .execute(&mut **tx)
        .await?;

        sqlx::query(
            "UPDATE change_history SET entity_id = ? \
             WHERE entity_type = 'show' AND entity_id = ?",
        )
        .bind(new_id)
        .bind(old_id)
        .execute(&mut **tx)
        .await?;
    }

    // Replace episodes with the fresh TMDB list.
    sqlx::query("DELETE FROM episodes WHERE show_id = ?")
        .bind(new_id)
        .execute(&mut **tx)
        .await?;

    let mut matched: usize = 0;
    let mut new_eps_by_key: HashMap<(i32, i32), i64> = HashMap::new();

    for ep in tmdb_eps {
        let key = (ep.season_number, ep.episode_number);
        let state = user_state.get(&key);
        let legacy = old_eps_by_key.get(&key).copied();

        if state.is_some() {
            matched += 1;
        }

        sqlx::query(
            "INSERT INTO episodes (id, show_id, season_number, episode_number, \
             name, overview, aired, runtime, image_url, watched, watched_at, \
             scheduled_date, rating, tags, legacy_tvdb_id) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(ep.id)
        .bind(new_id)
        .bind(ep.season_number)
        .bind(ep.episode_number)
        .bind(&ep.name)
        .bind(&ep.overview)
        .bind(&ep.air_date)
        .bind(ep.runtime)
        .bind(ep.image_url())
        .bind(state.map(|s| s.watched).unwrap_or(0))
        .bind(state.and_then(|s| s.watched_at.clone()))
        .bind(state.and_then(|s| s.scheduled_date.clone()))
        .bind(state.and_then(|s| s.rating))
        .bind(state.and_then(|s| s.tags.clone()))
        .bind(legacy)
        .execute(&mut **tx)
        .await?;

        new_eps_by_key.insert(key, ep.id);
    }

    // Episode-id rewrite chain on history/scrobble tables.
    rewrite_episode_refs(tx, old_eps_by_key, &new_eps_by_key).await?;

    let episodes_orphaned = user_state.len().saturating_sub(matched);
    Ok(RemapOutcome {
        episodes_orphaned,
        merged_with: None,
    })
}

async fn merge_into_existing(
    tx: &mut Transaction<'_, Sqlite>,
    old_id: i64,
    target_id: i64,
    user_state: &HashMap<(i32, i32), UserEpisodeState>,
    old_eps_by_key: &HashMap<(i32, i32), i64>,
) -> Result<RemapOutcome, sqlx::Error> {
    // Build the merge target's existing episode map.
    let existing = sqlx::query(
        "SELECT id, season_number, episode_number FROM episodes WHERE show_id = ?",
    )
    .bind(target_id)
    .fetch_all(&mut **tx)
    .await?;

    let mut target_eps_by_key: HashMap<(i32, i32), i64> = HashMap::new();
    for row in existing {
        let id: i64 = row.try_get("id")?;
        let season: Option<i32> = row.try_get("season_number")?;
        let episode: Option<i32> = row.try_get("episode_number")?;
        if let (Some(s), Some(e)) = (season, episode) {
            target_eps_by_key.insert((s, e), id);
        }
    }

    let mut matched = 0usize;

    // Port user state onto the surviving TMDB episodes where (season, ep) match.
    for (key, state) in user_state {
        if let Some(&target_ep_id) = target_eps_by_key.get(key) {
            matched += 1;
            // COALESCE keeps any state the target already has (we don't want
            // to clobber a watched=1 with the old row's watched=0).
            sqlx::query(
                "UPDATE episodes SET \
                 watched = MAX(watched, ?), \
                 watched_at = COALESCE(watched_at, ?), \
                 scheduled_date = COALESCE(scheduled_date, ?), \
                 rating = COALESCE(rating, ?), \
                 tags = COALESCE(tags, ?) \
                 WHERE id = ?",
            )
            .bind(state.watched)
            .bind(&state.watched_at)
            .bind(&state.scheduled_date)
            .bind(state.rating)
            .bind(&state.tags)
            .bind(target_ep_id)
            .execute(&mut **tx)
            .await?;
        }
    }

    // Reattach foreign-key children of the losing row.
    for sql in [
        "UPDATE sonarr_imports SET show_id = ? WHERE show_id = ?",
    ] {
        sqlx::query(sql)
            .bind(target_id)
            .bind(old_id)
            .execute(&mut **tx)
            .await?;
    }

    // Cast/crew on the target stay; drop the losing row's cast/crew so the
    // FK cascade on DELETE shows doesn't double-up.
    for sql in [
        "DELETE FROM cast_members WHERE show_id = ?",
        "DELETE FROM crew_members WHERE show_id = ?",
    ] {
        sqlx::query(sql).bind(old_id).execute(&mut **tx).await?;
    }

    sqlx::query(
        "UPDATE title_mappings SET tvc_id = ? \
         WHERE media_type = 'show' AND tvc_id = ?",
    )
    .bind(target_id)
    .bind(old_id)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        "UPDATE plex_scrobble_log SET matched_entity_id = ? \
         WHERE matched_entity_type = 'show' AND matched_entity_id = ?",
    )
    .bind(target_id)
    .bind(old_id)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        "UPDATE change_history SET entity_id = ? \
         WHERE entity_type = 'show' AND entity_id = ?",
    )
    .bind(target_id)
    .bind(old_id)
    .execute(&mut **tx)
    .await?;

    // Rewrite episode refs from old episode ids to surviving target ids.
    rewrite_episode_refs(tx, old_eps_by_key, &target_eps_by_key).await?;

    // Drop the losing show row; CASCADE removes its (now-orphaned) episodes.
    sqlx::query("DELETE FROM shows WHERE id = ?")
        .bind(old_id)
        .execute(&mut **tx)
        .await?;

    Ok(RemapOutcome {
        episodes_orphaned: user_state.len().saturating_sub(matched),
        merged_with: Some(target_id),
    })
}

async fn rewrite_episode_refs(
    tx: &mut Transaction<'_, Sqlite>,
    old: &HashMap<(i32, i32), i64>,
    new: &HashMap<(i32, i32), i64>,
) -> Result<(), sqlx::Error> {
    for (key, &old_ep_id) in old {
        match new.get(key) {
            Some(&new_ep_id) if new_ep_id != old_ep_id => {
                sqlx::query(
                    "UPDATE change_history SET entity_id = ? \
                     WHERE entity_type = 'episode' AND entity_id = ?",
                )
                .bind(new_ep_id)
                .bind(old_ep_id)
                .execute(&mut **tx)
                .await?;

                sqlx::query(
                    "UPDATE plex_scrobble_log SET matched_entity_id = ? \
                     WHERE matched_entity_type = 'episode' AND matched_entity_id = ?",
                )
                .bind(new_ep_id)
                .bind(old_ep_id)
                .execute(&mut **tx)
                .await?;
            }
            Some(_) => {
                // ids already align, nothing to do.
            }
            None => {
                // Episode no longer exists in TMDB; drop dangling history.
                sqlx::query(
                    "DELETE FROM change_history \
                     WHERE entity_type = 'episode' AND entity_id = ?",
                )
                .bind(old_ep_id)
                .execute(&mut **tx)
                .await?;
            }
        }
    }
    Ok(())
}

async fn mark_complete(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT OR REPLACE INTO settings (key, value) \
         VALUES ('tvdb_to_tmdb_migration_complete', '1')",
    )
    .execute(pool)
    .await?;
    Ok(())
}
