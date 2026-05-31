use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::{AppHandle, Emitter};
use tokio::sync::{Mutex, Semaphore};

use crate::db::connection;
use crate::tmdb;

/// Export data structure
#[derive(Debug, Serialize, Deserialize)]
pub struct BackupData {
    #[serde(default = "default_version")]
    pub version: String,
    pub exported_at: String,
    pub shows: Vec<ShowBackup>,
    pub episodes: Vec<EpisodeBackup>,
    pub movies: Vec<MovieBackup>,
    #[serde(default)]
    pub tiers: Vec<TierBackup>,
}

fn default_version() -> String {
    "1.0".to_string()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TierBackup {
    pub id: i64,
    pub position: i64,
    pub name: String,
    pub color: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShowBackup {
    pub id: i64,
    pub name: String,
    pub status: Option<String>,
    pub poster_url: Option<String>,
    pub first_aired: Option<String>,
    pub network: Option<String>,
    pub overview: Option<String>,
    pub runtime: Option<i32>,
    pub added_at: Option<String>,
    pub last_synced: Option<String>,
    pub color: Option<String>,
    pub notes: Option<String>,
    pub tags: Option<String>,
    pub archived: i32,
    pub rating: Option<f64>,
    #[serde(default)]
    pub tier_id: Option<i64>,
    #[serde(default)]
    pub tier_only: i32,
    #[serde(default)]
    pub rank_order: Option<i32>,
    #[serde(default)]
    pub legacy_tvdb_id: Option<i64>,
    #[serde(default)]
    pub tmdb_id: Option<i64>,
    #[serde(default)]
    pub unmigrated: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EpisodeBackup {
    pub id: i64,
    pub show_id: i64,
    pub season_number: Option<i32>,
    pub episode_number: Option<i32>,
    pub name: Option<String>,
    pub overview: Option<String>,
    pub aired: Option<String>,
    pub runtime: Option<i32>,
    pub image_url: Option<String>,
    pub watched: i32,
    pub watched_at: Option<String>,
    pub scheduled_date: Option<String>,
    pub rating: Option<f64>,
    pub tags: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MovieBackup {
    pub id: i64,
    pub title: String,
    pub tagline: Option<String>,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub release_date: Option<String>,
    pub digital_release_date: Option<String>,
    pub physical_release_date: Option<String>,
    pub runtime: Option<i32>,
    pub status: Option<String>,
    pub genres: Option<String>,
    pub vote_average: Option<f64>,
    pub scheduled_date: Option<String>,
    pub watched: i32,
    pub watched_at: Option<String>,
    pub rating: Option<f64>,
    pub notes: Option<String>,
    pub color: Option<String>,
    pub tags: Option<String>,
    pub archived: i32,
    pub added_at: Option<String>,
    pub last_synced: Option<String>,
    #[serde(default)]
    pub tier_id: Option<i64>,
    #[serde(default)]
    pub tier_only: i32,
    #[serde(default)]
    pub rank_order: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportResult {
    pub shows_imported: u32,
    pub episodes_imported: u32,
    pub movies_imported: u32,
    #[serde(default)]
    pub quarantined: u32,
    #[serde(default)]
    pub remapped: u32,
    #[serde(default)]
    pub episodes_orphaned: u32,
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

/// Export all user data to JSON (v2.0 format)
#[tauri::command]
pub async fn export_database(app: AppHandle) -> Result<BackupData, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let tier_rows = sqlx::query(
        r#"SELECT id, position, name, color, created_at FROM tiers ORDER BY position DESC"#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to export tiers: {}", e))?;

    let tiers: Vec<TierBackup> = tier_rows
        .into_iter()
        .map(|row| TierBackup {
            id: row.get("id"),
            position: row.get("position"),
            name: row.get("name"),
            color: row.get("color"),
            created_at: row.get("created_at"),
        })
        .collect();

    let show_rows = sqlx::query(
        r#"SELECT id, name, status, poster_url, first_aired, network, overview,
                  runtime, added_at, last_synced, color, notes, tags,
                  COALESCE(archived, 0) as archived, rating, tier_id,
                  COALESCE(tier_only, 0) as tier_only, rank_order,
                  legacy_tvdb_id, COALESCE(unmigrated, 0) as unmigrated
           FROM shows"#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to export shows: {}", e))?;

    let shows: Vec<ShowBackup> = show_rows
        .into_iter()
        .map(|row| {
            let id: i64 = row.get("id");
            let unmigrated: i32 = row.get("unmigrated");
            let tmdb_id = if unmigrated == 0 && id > 0 {
                Some(id)
            } else {
                None
            };
            ShowBackup {
                id,
                name: row.get("name"),
                status: row.get("status"),
                poster_url: row.get("poster_url"),
                first_aired: row.get("first_aired"),
                network: row.get("network"),
                overview: row.get("overview"),
                runtime: row.get("runtime"),
                added_at: row.get("added_at"),
                last_synced: row.get("last_synced"),
                color: row.get("color"),
                notes: row.get("notes"),
                tags: row.get("tags"),
                archived: row.get("archived"),
                rating: row.get::<Option<f64>, _>("rating"),
                tier_id: row.get("tier_id"),
                tier_only: row.get("tier_only"),
                rank_order: row.get("rank_order"),
                legacy_tvdb_id: row.get("legacy_tvdb_id"),
                tmdb_id,
                unmigrated,
            }
        })
        .collect();

    let episode_rows = sqlx::query(
        r#"SELECT id, show_id, season_number, episode_number, name, overview, aired,
                  runtime, image_url, watched, watched_at, scheduled_date, rating, tags
           FROM episodes"#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to export episodes: {}", e))?;

    let episodes: Vec<EpisodeBackup> = episode_rows
        .into_iter()
        .map(|row| EpisodeBackup {
            id: row.get("id"),
            show_id: row.get("show_id"),
            season_number: row.get("season_number"),
            episode_number: row.get("episode_number"),
            name: row.get("name"),
            overview: row.get("overview"),
            aired: row.get("aired"),
            runtime: row.get("runtime"),
            image_url: row.get("image_url"),
            watched: row.get("watched"),
            watched_at: row.get("watched_at"),
            scheduled_date: row.get("scheduled_date"),
            rating: row.get::<Option<f64>, _>("rating"),
            tags: row.get("tags"),
        })
        .collect();

    let movie_rows = sqlx::query(
        r#"SELECT id, title, tagline, overview, poster_url, backdrop_url, release_date,
                  digital_release_date, physical_release_date, runtime, status, genres,
                  vote_average, scheduled_date, watched, watched_at, rating, notes, color,
                  tags, COALESCE(archived, 0) as archived, added_at, last_synced,
                  tier_id, COALESCE(tier_only, 0) as tier_only, rank_order
           FROM movies"#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("Failed to export movies: {}", e))?;

    let movies: Vec<MovieBackup> = movie_rows
        .into_iter()
        .map(|row| MovieBackup {
            id: row.get("id"),
            title: row.get("title"),
            tagline: row.get("tagline"),
            overview: row.get("overview"),
            poster_url: row.get("poster_url"),
            backdrop_url: row.get("backdrop_url"),
            release_date: row.get("release_date"),
            digital_release_date: row.get("digital_release_date"),
            physical_release_date: row.get("physical_release_date"),
            runtime: row.get("runtime"),
            status: row.get("status"),
            genres: row.get("genres"),
            vote_average: row.get("vote_average"),
            scheduled_date: row.get("scheduled_date"),
            watched: row.get("watched"),
            watched_at: row.get("watched_at"),
            rating: row.get::<Option<f64>, _>("rating"),
            notes: row.get("notes"),
            color: row.get("color"),
            tags: row.get("tags"),
            archived: row.get("archived"),
            added_at: row.get("added_at"),
            last_synced: row.get("last_synced"),
            tier_id: row.get("tier_id"),
            tier_only: row.get("tier_only"),
            rank_order: row.get("rank_order"),
        })
        .collect();

    Ok(BackupData {
        version: "2.0".to_string(),
        exported_at: chrono::Utc::now().to_rfc3339(),
        shows,
        episodes,
        movies,
        tiers,
    })
}

/// Import data from JSON backup (replaces existing data).
/// v2.0 backups insert as-is. v1.0 (or unset) backups are treated as TVDB-keyed:
///   1. Pre-pass: TMDB `/find` to remap TVDB ids -> TMDB ids (silent).
///   2. Insert under new ids, carrying backup metadata as a placeholder.
///   3. Post-commit refresh pass: re-fetch details + episodes from TMDB in en-US
///      via remap_single_show for each remapped show, preserving user state.
///      Emits the same `tvdb_migration_started/progress/finished` events the
///      live runner uses so the existing MigrationProgress modal lights up.
#[tauri::command]
pub async fn import_database(app: AppHandle, data: BackupData) -> Result<ImportResult, String> {
    let pool = connection::get_pool(&app)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let is_legacy = data.version != "2.0";

    // Build a TVDB -> Option<TMDB> remap for legacy backups. Pre-pass uses the
    // same Semaphore/pacing/event names as the live migration runner so the
    // existing MigrationProgress overlay surfaces restore progress unchanged.
    let mut remap: HashMap<i64, Option<i64>> = HashMap::new();
    let mut errors: Vec<String> = Vec::new();
    let mut per_show: Vec<PerShowResult> = Vec::new();

    if is_legacy {
        // Only positive ids are TVDB-keyed; manual tier-only entries use
        // negative ids and skip the lookup. This pre-pass is just the /find
        // lookup so we can pick the right id for the insert; the heavier
        // metadata refresh runs post-commit (see the refresh pass below)
        // where it can be parallelized over remap_single_show. No progress
        // events here — /find is fast (~6 s for 500 shows with concurrency 8).
        let candidates: Vec<(i64, String)> = data
            .shows
            .iter()
            .filter(|s| s.id > 0)
            .map(|s| (s.id, s.name.clone()))
            .collect();
        let total = candidates.len();

        if total > 0 {
            type MapEntry = (i64, Option<i64>);
            let semaphore = Arc::new(Semaphore::new(8));
            let collected: Arc<Mutex<Vec<MapEntry>>> =
                Arc::new(Mutex::new(Vec::with_capacity(total)));

            let mut handles = Vec::with_capacity(total);
            for (tvdb_id, _name) in candidates {
                let semaphore = semaphore.clone();
                let collected = collected.clone();

                handles.push(tokio::spawn(async move {
                    let _permit = match semaphore.acquire_owned().await {
                        Ok(p) => p,
                        Err(_) => return,
                    };
                    tokio::time::sleep(Duration::from_millis(80)).await;

                    let mapped = tmdb::find_tv_by_tvdb_id(tvdb_id).await.unwrap_or_default();

                    let mut c = collected.lock().await;
                    c.push((tvdb_id, mapped));
                }));
            }

            for h in handles {
                let _ = h.await;
            }

            let collected = {
                let guard = collected.lock().await;
                guard.clone()
            };
            for (tvdb_id, mapped) in collected {
                remap.insert(tvdb_id, mapped);
            }
        }
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    sqlx::query("PRAGMA defer_foreign_keys = ON")
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to defer foreign keys: {}", e))?;

    // Clear auxiliary tables before the parents. cast_members/crew_members
    // have ON DELETE CASCADE but sonarr_imports/radarr_imports do not — those
    // would dangle and fail FK check at commit. change_history/title_mappings/
    // plex_scrobble_log carry show/episode/movie ids without FK constraints,
    // but the new shows reuse old TVDB ids as TMDB ids, so leaving stale rows
    // would silently mis-attribute history. Wipe them too.
    for stmt in [
        "DELETE FROM cast_members",
        "DELETE FROM crew_members",
        "DELETE FROM sonarr_imports",
        "DELETE FROM radarr_imports",
        "DELETE FROM change_history",
        "DELETE FROM title_mappings",
        "DELETE FROM plex_scrobble_log",
        "DELETE FROM episodes",
        "DELETE FROM shows",
        "DELETE FROM movies",
        "DELETE FROM tiers",
    ] {
        if let Err(e) = sqlx::query(stmt).execute(&mut *tx).await {
            let _ = tx.rollback().await;
            return Err(format!("Failed to clear data ({}): {}", stmt, e));
        }
    }

    // Tiers first — shows/movies reference them via tier_id FK.
    for tier in &data.tiers {
        if let Err(e) = sqlx::query(
            r#"INSERT INTO tiers (id, position, name, color, created_at)
               VALUES (?, ?, ?, ?, ?)"#,
        )
        .bind(tier.id)
        .bind(tier.position)
        .bind(&tier.name)
        .bind(&tier.color)
        .bind(&tier.created_at)
        .execute(&mut *tx)
        .await
        {
            let _ = tx.rollback().await;
            return Err(format!("Failed to import tier {}: {}", tier.name, e));
        }
    }

    // Insert shows. For legacy backups, route through remap; for v2.0 insert as-is.
    // Track final ids assigned to each backup row so episodes can be rewritten.
    let mut show_id_remap: HashMap<i64, i64> = HashMap::new();
    let mut final_ids: HashSet<i64> = HashSet::new();
    let mut quarantined: u32 = 0;
    let mut remapped: u32 = 0;
    let mut shows_imported: u32 = 0;
    // Shows that need a TMDB metadata refresh after commit (v1.0 backups only).
    // The backup stored TVDB-sourced fields (name/poster/overview/episode names)
    // which may be in a non-English language for anime / regional content.
    // remap_single_show re-fetches in en-US (with original-language fallback).
    let mut to_refresh: Vec<(i64, String)> = Vec::new();

    for show in &data.shows {
        let (final_id, legacy_tvdb_id, unmigrated) = if !is_legacy {
            (
                show.id,
                show.legacy_tvdb_id,
                if show.unmigrated == 1 { 1 } else { 0 },
            )
        } else if show.id <= 0 {
            // Manual tier-only entry — keep id as-is, not subject to remap.
            (show.id, None, 0)
        } else {
            match remap.get(&show.id).copied().flatten() {
                Some(new_id) if !final_ids.contains(&new_id) => (new_id, Some(show.id), 0),
                Some(_collision) => {
                    // Another backup show already claimed this TMDB id — quarantine
                    // this one under its original TVDB id (also check collision).
                    if final_ids.contains(&show.id) {
                        errors.push(format!(
                            "Skipped {}: TMDB collision and TVDB id already used",
                            show.name
                        ));
                        continue;
                    }
                    (show.id, Some(show.id), 1)
                }
                None => {
                    if final_ids.contains(&show.id) {
                        errors.push(format!(
                            "Skipped {}: duplicate TVDB id in backup",
                            show.name
                        ));
                        continue;
                    }
                    (show.id, Some(show.id), 1)
                }
            }
        };

        if let Err(e) = sqlx::query(
            r#"INSERT INTO shows (id, name, status, poster_url, first_aired, network,
                                  overview, runtime, added_at, last_synced,
                                  color, notes, tags, archived, rating, tier_id, tier_only, rank_order,
                                  legacy_tvdb_id, unmigrated)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(final_id)
        .bind(&show.name)
        .bind(&show.status)
        .bind(&show.poster_url)
        .bind(&show.first_aired)
        .bind(&show.network)
        .bind(&show.overview)
        .bind(show.runtime)
        .bind(&show.added_at)
        .bind(&show.last_synced)
        .bind(&show.color)
        .bind(&show.notes)
        .bind(&show.tags)
        .bind(show.archived)
        .bind(show.rating)
        .bind(show.tier_id)
        .bind(show.tier_only)
        .bind(show.rank_order)
        .bind(legacy_tvdb_id)
        .bind(unmigrated)
        .execute(&mut *tx)
        .await
        {
            errors.push(format!("Failed to import show {}: {}", show.name, e));
            continue;
        }

        show_id_remap.insert(show.id, final_id);
        final_ids.insert(final_id);
        shows_imported += 1;
        if unmigrated == 1 {
            quarantined += 1;
        } else if is_legacy && show.id > 0 && legacy_tvdb_id.is_some() {
            // Successfully mapped via /find — queue for English refresh.
            // (Includes the rare same-id case where TVDB id == TMDB id.)
            if final_id != show.id {
                remapped += 1;
            }
            per_show.push(PerShowResult {
                name: show.name.clone(),
                new_tmdb_id: final_id,
                episodes_orphaned: 0,
                merged_with: None,
            });
            to_refresh.push((final_id, show.name.clone()));
        }
    }

    // Episodes — rewrite show_id through the remap. Skip orphaned episodes
    // (parent show was dropped above).
    let mut episodes_imported: u32 = 0;
    let mut episodes_orphaned: u32 = 0;

    for episode in &data.episodes {
        let Some(&new_show_id) = show_id_remap.get(&episode.show_id) else {
            episodes_orphaned += 1;
            continue;
        };

        if let Err(e) = sqlx::query(
            r#"INSERT INTO episodes (id, show_id, season_number, episode_number, name, overview,
                                     aired, runtime, image_url, watched, watched_at, scheduled_date,
                                     rating, tags)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(episode.id)
        .bind(new_show_id)
        .bind(episode.season_number)
        .bind(episode.episode_number)
        .bind(&episode.name)
        .bind(&episode.overview)
        .bind(&episode.aired)
        .bind(episode.runtime)
        .bind(&episode.image_url)
        .bind(episode.watched)
        .bind(&episode.watched_at)
        .bind(&episode.scheduled_date)
        .bind(episode.rating)
        .bind(&episode.tags)
        .execute(&mut *tx)
        .await
        {
            errors.push(format!("Failed to import episode {}: {}", episode.id, e));
            continue;
        }
        episodes_imported += 1;
    }

    // Movies are unaffected by the TVDB → TMDB transition.
    let mut movies_imported: u32 = 0;
    for movie in &data.movies {
        if let Err(e) = sqlx::query(
            r#"INSERT INTO movies (id, title, tagline, overview, poster_url, backdrop_url,
                                   release_date, digital_release_date, physical_release_date,
                                   runtime, status, genres, vote_average, scheduled_date, watched,
                                   watched_at, rating, notes, color, tags, archived, added_at, last_synced,
                                   tier_id, tier_only, rank_order)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(movie.id)
        .bind(&movie.title)
        .bind(&movie.tagline)
        .bind(&movie.overview)
        .bind(&movie.poster_url)
        .bind(&movie.backdrop_url)
        .bind(&movie.release_date)
        .bind(&movie.digital_release_date)
        .bind(&movie.physical_release_date)
        .bind(movie.runtime)
        .bind(&movie.status)
        .bind(&movie.genres)
        .bind(movie.vote_average)
        .bind(&movie.scheduled_date)
        .bind(movie.watched)
        .bind(&movie.watched_at)
        .bind(movie.rating)
        .bind(&movie.notes)
        .bind(&movie.color)
        .bind(&movie.tags)
        .bind(movie.archived)
        .bind(&movie.added_at)
        .bind(&movie.last_synced)
        .bind(movie.tier_id)
        .bind(movie.tier_only)
        .bind(movie.rank_order)
        .execute(&mut *tx)
        .await
        {
            let _ = tx.rollback().await;
            return Err(format!("Failed to import movie {}: {}", movie.title, e));
        }
        movies_imported += 1;
    }

    tx.commit()
        .await
        .map_err(|e| format!("Failed to commit transaction: {}", e))?;

    if is_legacy {
        // Refresh pass: re-fetch metadata + episodes from TMDB in en-US for
        // each successfully-remapped show. User state survives via (season,
        // ep) matching inside remap_single_show. This is what makes anime /
        // regional shows come out in English on first launch instead of
        // displaying the TVDB-stored Japanese/Korean/etc. titles.
        let refresh_total = to_refresh.len();
        let mut orphan_index: HashMap<i64, usize> = HashMap::new();

        if refresh_total > 0 {
            let _ = app.emit(
                "tvdb_migration_started",
                StartedEvent {
                    total: refresh_total,
                },
            );

            let semaphore = Arc::new(Semaphore::new(8));
            let progress = Arc::new(AtomicUsize::new(0));
            // Collect (tmdb_id, orphan_count, optional_error) per show.
            type RefreshEntry = (i64, usize, Option<String>);
            let collected: Arc<Mutex<Vec<RefreshEntry>>> =
                Arc::new(Mutex::new(Vec::with_capacity(refresh_total)));

            let mut handles = Vec::with_capacity(refresh_total);
            for (new_id, name) in to_refresh.iter().cloned() {
                let semaphore = semaphore.clone();
                let pool = pool.clone();
                let app = app.clone();
                let progress = progress.clone();
                let collected = collected.clone();

                handles.push(tokio::spawn(async move {
                    let _permit = match semaphore.acquire_owned().await {
                        Ok(p) => p,
                        Err(_) => return,
                    };
                    tokio::time::sleep(Duration::from_millis(80)).await;

                    let entry: RefreshEntry =
                        match crate::db::tvdb_remap::remap_single_show(&pool, new_id, new_id)
                            .await
                        {
                            Ok(outcome) => (new_id, outcome.episodes_orphaned, None),
                            Err(e) => (new_id, 0, Some(e)),
                        };

                    {
                        let mut c = collected.lock().await;
                        c.push(entry);
                    }

                    let done = progress.fetch_add(1, Ordering::SeqCst) + 1;
                    let _ = app.emit(
                        "tvdb_migration_progress",
                        ProgressEvent {
                            done,
                            total: refresh_total,
                            current_name: name,
                        },
                    );
                }));
            }

            for h in handles {
                let _ = h.await;
            }

            let collected = {
                let guard = collected.lock().await;
                guard.clone()
            };

            // Pre-index per_show by tmdb id so we can fill in orphan counts.
            for (i, ps) in per_show.iter().enumerate() {
                orphan_index.insert(ps.new_tmdb_id, i);
            }

            for (new_id, orphans, err) in collected {
                if let Some(msg) = err {
                    errors.push(format!("Refresh tmdb_id={}: {}", new_id, msg));
                    // The /find call succeeded but the metadata/episode
                    // refresh didn't. Without this flag the row would sit
                    // unmigrated=0 with stale TVDB-language data, invisible
                    // to the live runner on next launch (it only picks up
                    // unmigrated=1). Flip it back to 1 so we retry then,
                    // and bump the quarantined counter so the completion
                    // flag isn't set prematurely.
                    let _ = sqlx::query(
                        "UPDATE shows SET unmigrated = 1, last_synced = NULL WHERE id = ?",
                    )
                    .bind(new_id)
                    .execute(&pool)
                    .await;
                    quarantined += 1;
                    if remapped > 0 {
                        remapped -= 1;
                    }
                    continue;
                }
                if let Some(&i) = orphan_index.get(&new_id) {
                    per_show[i].episodes_orphaned = orphans;
                }
                episodes_orphaned += orphans as u32;
            }
        }

        let _ = app.emit(
            "tvdb_migration_finished",
            FinishedEvent {
                mapped: remapped as usize,
                quarantined: quarantined as usize,
                errors: errors.clone(),
                per_show,
            },
        );

        // If anything ended in quarantine we leave the global flag alone; the
        // resolver UI can finish the job. If the restore was 100% clean, mark
        // migration complete so the runner doesn't fire on next launch.
        if quarantined == 0 && errors.is_empty() {
            let _ = sqlx::query(
                "INSERT OR REPLACE INTO settings (key, value) \
                 VALUES ('tvdb_to_tmdb_migration_complete', '1')",
            )
            .execute(&pool)
            .await;
        }
    }

    Ok(ImportResult {
        shows_imported,
        episodes_imported,
        movies_imported,
        quarantined,
        remapped,
        episodes_orphaned,
    })
}
