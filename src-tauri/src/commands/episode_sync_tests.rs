//! Episode reconciliation regressions use only disposable in-memory databases.
use super::*;
use serde_json::json;

async fn database() -> sqlx::SqlitePool {
    let pool = crate::audit_tests::database().await;
    sqlx::query("INSERT INTO shows (id,name) VALUES (1,'Top Gear'),(2,'Other show')")
        .execute(&pool)
        .await
        .unwrap();
    pool
}

fn details() -> tmdb::TvShowDetails {
    serde_json::from_value(json!({"id":1,"name":"Updated Top Gear"})).unwrap()
}

fn episode(id: i64, season: i32, number: i32) -> tmdb::TvEpisode {
    serde_json::from_value(json!({
        "id":id,"season_number":season,"episode_number":number,
        "name":format!("Episode {id}"),"air_date":"2026-10-24"
    }))
    .unwrap()
}

async fn seed_personal(pool: &sqlx::SqlitePool, id: i64, season: i32, number: i32) {
    sqlx::query(
        "INSERT INTO episodes (id,show_id,season_number,episode_number,name,watched,
         watched_at,scheduled_date,rating,tags,legacy_tvdb_id)
         VALUES (?,1,?,?,'Saved episode',1,'2026-09-15T20:00:00Z','2026-09-14',8.5,'[\"favorite\"]',?)",
    )
    .bind(id)
    .bind(season)
    .bind(number)
    .bind(id + 9000)
    .execute(pool)
    .await
    .unwrap();
}

async fn assert_personal(pool: &sqlx::SqlitePool, id: i64, original_id: i64) {
    let row = sqlx::query("SELECT * FROM episodes WHERE id=?")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(row.get::<i32, _>("watched"), 1);
    assert_eq!(row.get::<String, _>("watched_at"), "2026-09-15T20:00:00Z");
    assert_eq!(row.get::<String, _>("scheduled_date"), "2026-09-14");
    assert_eq!(row.get::<f64, _>("rating"), 8.5);
    assert_eq!(row.get::<String, _>("tags"), "[\"favorite\"]");
    assert_eq!(row.get::<i64, _>("legacy_tvdb_id"), original_id + 9000);
}

async fn rows(pool: &sqlx::SqlitePool) -> Vec<(i64, i64, i32, i32)> {
    sqlx::query_as("SELECT id,show_id,season_number,episode_number FROM episodes ORDER BY id")
        .fetch_all(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn swapped_episode_numbers_keep_personal_state_with_provider_identity() {
    let pool = database().await;
    seed_personal(&pool, 10, 0, 1).await;
    seed_personal(&pool, 11, 0, 2).await;
    sqlx::query("UPDATE episodes SET watched=0,watched_at=NULL,tags='[\"second\"]' WHERE id=11")
        .execute(&pool)
        .await
        .unwrap();

    apply_show_refresh(
        &pool,
        &details(),
        vec![episode(11, 0, 1), episode(10, 0, 2)],
    )
    .await
    .unwrap();

    assert_eq!(rows(&pool).await, vec![(10, 1, 0, 2), (11, 1, 0, 1)]);
    assert_personal(&pool, 10, 10).await;
    let second =
        sqlx::query("SELECT watched,watched_at,tags,legacy_tvdb_id FROM episodes WHERE id=11")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(second.get::<i32, _>("watched"), 0);
    assert!(second.get::<Option<String>, _>("watched_at").is_none());
    assert_eq!(second.get::<String, _>("tags"), "[\"second\"]");
    assert_eq!(second.get::<i64, _>("legacy_tvdb_id"), 9011);
}

#[tokio::test]
async fn renumbered_special_keeps_its_history_and_unmatched_personal_episodes() {
    let pool = database().await;
    seed_personal(&pool, 10, 0, 1).await;
    seed_personal(&pool, 20, 1, 1).await;
    sqlx::query("INSERT INTO episodes (id,show_id,season_number,episode_number) VALUES (30,1,1,2)")
        .execute(&pool)
        .await
        .unwrap();

    apply_show_refresh(&pool, &details(), vec![episode(10, 0, 7)])
        .await
        .unwrap();

    assert_eq!(rows(&pool).await, vec![(10, 1, 0, 7), (20, 1, 1, 1)]);
    assert_personal(&pool, 10, 10).await;
    assert_personal(&pool, 20, 20).await;
}

#[tokio::test]
async fn mixed_ids_keep_state_legacy_ids_and_reference_owners_after_replacement() {
    let pool = database().await;
    seed_personal(&pool, 10, 1, 1).await;
    seed_personal(&pool, 11, 1, 2).await;
    seed_personal(&pool, 20, 0, 1).await;
    sqlx::raw_sql(
        "INSERT INTO change_history (id,entity_type,entity_id,change_type) VALUES
         (1,'episode',10,'watched'),(2,'episode',11,'watched'),(3,'movie',10,'watched');
         INSERT INTO plex_scrobble_log (id,event_type,media_type,raw_title,matched_entity_type,matched_entity_id)
         VALUES (1,'media.scrobble','episode','First','episode',10),
                (2,'media.scrobble','episode','Second','episode',11),
                (3,'media.scrobble','movie','Movie','movie',10);",
    )
    .execute(&pool)
    .await
    .unwrap();

    apply_show_refresh(
        &pool,
        &details(),
        vec![episode(100, 1, 1), episode(11, 1, 3), episode(101, 2, 1)],
    )
    .await
    .unwrap();

    assert_eq!(
        rows(&pool).await,
        vec![(11, 1, 1, 3), (20, 1, 0, 1), (100, 1, 1, 1), (101, 1, 2, 1)]
    );
    for (id, original) in [(100, 10), (11, 11), (20, 20)] {
        assert_personal(&pool, id, original).await;
    }
    let fresh = sqlx::query("SELECT watched,watched_at,scheduled_date,rating,tags,legacy_tvdb_id FROM episodes WHERE id=101")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(fresh.get::<i32, _>("watched"), 0);
    for column in ["watched_at", "scheduled_date", "tags"] {
        assert!(fresh.get::<Option<String>, _>(column).is_none());
    }
    assert!(fresh.get::<Option<f64>, _>("rating").is_none());
    assert!(fresh.get::<Option<i64>, _>("legacy_tvdb_id").is_none());
    for query in [
        "SELECT entity_id FROM change_history ORDER BY id",
        "SELECT matched_entity_id FROM plex_scrobble_log ORDER BY id",
    ] {
        assert_eq!(
            sqlx::query_scalar::<_, i64>(query)
                .fetch_all(&pool)
                .await
                .unwrap(),
            vec![100, 11, 10]
        );
    }
}

#[tokio::test]
async fn incoming_identity_cannot_also_donate_state_to_a_replacement_at_its_old_number() {
    let pool = database().await;
    seed_personal(&pool, 10, 0, 1).await;
    apply_show_refresh(
        &pool,
        &details(),
        vec![episode(100, 0, 1), episode(10, 0, 7)],
    )
    .await
    .unwrap();
    assert_personal(&pool, 10, 10).await;
    assert_eq!(rows(&pool).await, vec![(10, 1, 0, 7), (100, 1, 0, 1)]);
    let replacement: (i32, Option<String>, Option<f64>) =
        sqlx::query_as("SELECT watched,watched_at,rating FROM episodes WHERE id=100")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(replacement, (0, None, None));
}

#[tokio::test]
async fn conflicting_other_show_owner_rejects_sync_without_changing_either_show() {
    let pool = database().await;
    seed_personal(&pool, 10, 1, 1).await;
    sqlx::query("INSERT INTO episodes (id,show_id,season_number,episode_number,watched) VALUES (100,2,3,4,1)")
        .execute(&pool)
        .await
        .unwrap();
    let before = rows(&pool).await;
    let error = apply_show_refresh(
        &pool,
        &details(),
        vec![episode(101, 1, 1), episode(100, 1, 2)],
    )
    .await
    .unwrap_err();
    assert!(error.contains("100"), "{error}");
    assert!(error.contains("Other show"), "{error}");
    assert_eq!(rows(&pool).await, before);
    assert_personal(&pool, 10, 10).await;
    let name: String = sqlx::query_scalar("SELECT name FROM shows WHERE id=1")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(name, "Top Gear");
}

#[tokio::test]
async fn duplicate_or_invalid_provider_response_preserves_saved_data() {
    for episodes in [
        vec![episode(100, 1, 1), episode(100, 1, 2)],
        vec![episode(100, 1, 1), episode(101, 1, 1)],
        vec![episode(0, 1, 1)],
        vec![episode(-100, 1, 1)],
    ] {
        let pool = database().await;
        seed_personal(&pool, 10, 1, 1).await;
        let before = rows(&pool).await;
        assert!(apply_show_refresh(&pool, &details(), episodes)
            .await
            .is_err());
        assert_eq!(rows(&pool).await, before);
        assert_personal(&pool, 10, 10).await;
        let name: String = sqlx::query_scalar("SELECT name FROM shows WHERE id=1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(name, "Top Gear");
    }
}

#[tokio::test]
async fn failed_replacement_rolls_back_metadata_personal_state_and_history() {
    let pool = database().await;
    seed_personal(&pool, 10, 1, 1).await;
    sqlx::raw_sql(
        "INSERT INTO change_history (entity_type,entity_id,change_type) VALUES ('episode',10,'watched');
         INSERT INTO plex_scrobble_log (event_type,media_type,raw_title,matched_entity_type,matched_entity_id)
         VALUES ('media.scrobble','episode','Saved episode','episode',10);
         CREATE TRIGGER reject_replacement BEFORE INSERT ON episodes WHEN NEW.id=100
         BEGIN SELECT RAISE(ABORT,'test write failure'); END;",
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        apply_show_refresh(&pool, &details(), vec![episode(100, 1, 1)])
            .await
            .is_err()
    );
    assert_eq!(rows(&pool).await, vec![(10, 1, 1, 1)]);
    assert_personal(&pool, 10, 10).await;
    for query in [
        "SELECT entity_id FROM change_history",
        "SELECT matched_entity_id FROM plex_scrobble_log",
    ] {
        assert_eq!(
            sqlx::query_scalar::<_, i64>(query)
                .fetch_one(&pool)
                .await
                .unwrap(),
            10
        );
    }
    let metadata: (String, Option<String>) =
        sqlx::query_as("SELECT name,last_synced FROM shows WHERE id=1")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(metadata, ("Top Gear".into(), None));
}

#[tokio::test]
async fn unmatched_episodes_referenced_only_by_history_or_plex_are_retained() {
    let pool = database().await;
    sqlx::raw_sql(
        "INSERT INTO episodes (id,show_id,season_number,episode_number)
         VALUES (10,1,0,1),(11,1,0,2),(12,1,0,3);
         INSERT INTO change_history (entity_type,entity_id,change_type) VALUES ('episode',10,'watched');
         INSERT INTO plex_scrobble_log (event_type,media_type,raw_title,matched_entity_type,matched_entity_id)
         VALUES ('media.scrobble','episode','Saved episode','episode',11);",
    )
    .execute(&pool)
    .await
    .unwrap();
    apply_show_refresh(&pool, &details(), vec![episode(100, 1, 1)])
        .await
        .unwrap();
    assert_eq!(
        rows(&pool).await,
        vec![(10, 1, 0, 1), (11, 1, 0, 2), (100, 1, 1, 1)]
    );
    let history: i64 = sqlx::query_scalar("SELECT entity_id FROM change_history")
        .fetch_one(&pool)
        .await
        .unwrap();
    let plex: i64 = sqlx::query_scalar("SELECT matched_entity_id FROM plex_scrobble_log")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!((history, plex), (10, 11));
}

#[tokio::test]
async fn unmatched_episodes_referenced_by_legacy_ids_are_retained() {
    let pool = database().await;
    sqlx::raw_sql(
        "INSERT INTO episodes (id,show_id,season_number,episode_number,legacy_tvdb_id)
         VALUES (10,1,0,1,9010),(11,1,0,2,9011),(12,1,0,3,9012),(13,1,0,4,NULL);
         INSERT INTO change_history (entity_type,entity_id,change_type) VALUES
         ('episode',9010,'watched'),('movie',9012,'watched');
         INSERT INTO plex_scrobble_log (event_type,media_type,raw_title,matched_entity_type,matched_entity_id)
         VALUES ('media.scrobble','episode','Legacy episode','episode',9011),
                ('media.scrobble','movie','Unrelated movie','movie',9012);",
    )
    .execute(&pool)
    .await
    .unwrap();

    apply_show_refresh(&pool, &details(), vec![episode(100, 1, 1)])
        .await
        .unwrap();

    assert_eq!(
        rows(&pool).await,
        vec![(10, 1, 0, 1), (11, 1, 0, 2), (100, 1, 1, 1)]
    );
    let retained: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT id,legacy_tvdb_id FROM episodes WHERE legacy_tvdb_id IS NOT NULL ORDER BY id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(retained, vec![(10, 9010), (11, 9011)]);
    let history: i64 =
        sqlx::query_scalar("SELECT entity_id FROM change_history WHERE entity_type='episode'")
            .fetch_one(&pool)
            .await
            .unwrap();
    let plex: i64 = sqlx::query_scalar(
        "SELECT matched_entity_id FROM plex_scrobble_log WHERE matched_entity_type='episode'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((history, plex), (9010, 9011));
}
