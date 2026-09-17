//! All fixtures are disposable. This module never resolves an installed app path.
use sqlx::{Row, SqlitePool};

pub(crate) async fn database() -> SqlitePool {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    for migration in [
        crate::MIGRATION_001,
        crate::MIGRATION_002,
        crate::MIGRATION_003,
        crate::MIGRATION_004,
        crate::MIGRATION_005,
        crate::MIGRATION_006,
        crate::MIGRATION_007,
        crate::MIGRATION_008,
        crate::MIGRATION_009,
        crate::MIGRATION_010,
        crate::MIGRATION_011,
        crate::MIGRATION_012,
        crate::MIGRATION_013,
        crate::MIGRATION_014,
        crate::MIGRATION_015,
        crate::MIGRATION_016,
        crate::MIGRATION_017,
        crate::MIGRATION_018,
        crate::MIGRATION_019,
    ] {
        sqlx::raw_sql(migration).execute(&pool).await.unwrap();
    }
    pool
}

#[test]
fn canonical_dates_reject_impossible_or_unpadded_values() {
    for date in [
        "2026-02-29",
        "2026-02-31",
        "2026-4-01",
        "2026-04-1",
        "2026-13-01",
        "2026-04-31",
        "0000-01-01",
        "2026-09-17T12:00:00Z",
    ] {
        assert!(
            crate::commands::validation::validate_date(date).is_err(),
            "{date}"
        );
    }
    for date in ["2024-02-29", "2026-09-17", "2100-12-31"] {
        assert!(crate::commands::validation::validate_date(date).is_ok());
    }
}

#[tokio::test]
async fn calendar_uses_visible_date_and_excludes_archived_and_tier_only() {
    let pool = database().await;
    sqlx::raw_sql(
        "INSERT INTO shows (id,name) VALUES (1,'Visible'),(2,'Archived'),(3,'Tier');
        UPDATE shows SET archived = 1 WHERE id = 2; UPDATE shows SET tier_only = 1 WHERE id = 3;
        INSERT INTO episodes (id,show_id,aired,scheduled_date) VALUES
        (11,1,'2026-09-01','2026-10-01'),(12,1,'2026-10-01','2026-09-17'),
        (13,1,'2026-09-18',NULL),(21,2,'2026-09-17',NULL),(31,3,'2026-09-17',NULL);",
    )
    .execute(&pool)
    .await
    .unwrap();
    let episodes = crate::commands::episodes::episodes_for_range(&pool, "2026-09-01", "2026-09-30")
        .await
        .unwrap();
    assert_eq!(
        episodes.iter().map(|e| e.id).collect::<Vec<_>>(),
        vec![12, 13]
    );
    sqlx::query("UPDATE shows SET archived=0 WHERE id=2")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        crate::commands::episodes::episodes_for_range(&pool, "2026-09-01", "2026-09-30")
            .await
            .unwrap()
            .len(),
        3
    );
}

#[tokio::test]
async fn watched_actions_and_repeated_scrobbles_preserve_first_watch_date() {
    use crate::watch_history::{episodes, Episodes};
    let pool = database().await;
    sqlx::raw_sql("INSERT INTO shows (id,name) VALUES (1,'Show');
        INSERT INTO episodes (id,show_id,season_number,episode_number,watched,watched_at) VALUES
        (1,1,1,1,1,'2026-01-01 20:00:00'),(2,1,1,2,0,NULL),(3,1,2,1,0,NULL);
        INSERT INTO movies (id,title,watched,watched_at) VALUES (1,'Movie',1,'2026-02-01 20:00:00');")
        .execute(&pool).await.unwrap();
    episodes(&pool, Episodes::Season(1, 1), true).await.unwrap();
    let dates: Vec<Option<String>> =
        sqlx::query_scalar("SELECT watched_at FROM episodes ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(dates[0].as_deref(), Some("2026-01-01 20:00:00"));
    assert!(dates[1].is_some());
    assert!(dates[2].is_none());
    episodes(&pool, Episodes::Show(1), true).await.unwrap();
    crate::plex::matcher::mark_episode_watched(&pool, 1)
        .await
        .unwrap();
    let date: String = sqlx::query_scalar("SELECT watched_at FROM episodes WHERE id=1")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(date, "2026-01-01 20:00:00");
    crate::plex::matcher::mark_movie_watched(&pool, 1)
        .await
        .unwrap();
    let date: String = sqlx::query_scalar("SELECT watched_at FROM movies WHERE id=1")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(date, "2026-02-01 20:00:00");
    episodes(&pool, Episodes::One(1), false).await.unwrap();
    let cleared: Option<String> = sqlx::query_scalar("SELECT watched_at FROM episodes WHERE id=1")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(cleared.is_none());
}

#[tokio::test]
async fn plex_rejects_remakes_substrings_and_ambiguous_titles() {
    use crate::plex::matcher::{match_episode, match_movie};
    let pool = database().await;
    sqlx::raw_sql("INSERT INTO movies(id,title,release_date) VALUES (1,'The Thing','1982-01-01'),(2,'The Thing','2011-01-01'),(3,'Amélie!','2001-01-01');
        INSERT INTO shows(id,name) VALUES (1,'The Office'),(2,'The Office'),(3,'The Office Australia');
        INSERT INTO episodes(id,show_id,season_number,episode_number) VALUES (1,1,1,1),(2,2,1,1),(3,3,1,1);")
        .execute(&pool).await.unwrap();
    assert_eq!(
        match_movie(&pool, "The Thing", Some(1982), None)
            .await
            .unwrap()
            .entity_id,
        1
    );
    assert!(match_movie(&pool, "The Thing", Some(1999), None)
        .await
        .is_none());
    assert!(match_movie(&pool, "The Thing", None, None).await.is_none());
    assert!(match_movie(&pool, "The Thing", Some(1982), Some(999))
        .await
        .is_none());
    assert_eq!(
        match_movie(&pool, "AMÉLIE", Some(2001), None)
            .await
            .unwrap()
            .entity_id,
        3
    );
    assert!(match_episode(&pool, "Office", 1, 1).await.is_none());
    assert!(match_episode(&pool, "The Office", 1, 1).await.is_none());
    sqlx::raw_sql("INSERT INTO title_mappings(plex_title,media_type,tvc_id,tvc_title) VALUES ('The Office','show',2,'The Office'),('Custom','movie',999,'Missing');")
        .execute(&pool).await.unwrap();
    assert_eq!(
        match_episode(&pool, "The Office", 1, 1)
            .await
            .unwrap()
            .entity_id,
        2
    );
    assert!(match_movie(&pool, "Custom", None, None).await.is_none());
}

#[tokio::test]
async fn imported_titles_delete_and_duplicate_merge_preserves_state() {
    let pool = database().await;
    sqlx::raw_sql("INSERT INTO shows(id,name) VALUES (1,'Kept'),(2,'Merged');
        INSERT INTO movies(id,title) VALUES (3,'Movie');
        INSERT INTO arr_servers(id,name,type,base_url,api_key) VALUES (1,'Test','sonarr','http://localhost','fixture');
        INSERT INTO sonarr_imports(show_id,arr_server_id) VALUES (2,1);
        INSERT INTO radarr_imports(movie_id,arr_server_id) VALUES (3,1);
        INSERT INTO episodes(id,show_id,season_number,episode_number,watched,watched_at,rating,tags) VALUES
        (11,1,1,1,0,NULL,NULL,'[\"keep\"]'),(21,2,1,1,1,'2026-01-01',4.5,'[\"merge\"]');
        INSERT INTO title_mappings(plex_title,media_type,tvc_id,tvc_title) VALUES ('Mapped','show',2,'Merged');")
        .execute(&pool).await.unwrap();
    assert!(crate::commands::duplicates::merge_in_pool(&pool, 1, 1)
        .await
        .is_err());
    assert!(crate::commands::duplicates::merge_in_pool(&pool, 999, 2)
        .await
        .is_err());
    let result = crate::commands::duplicates::merge_in_pool(&pool, 1, 2)
        .await
        .unwrap();
    assert_eq!(result.episodes_merged, 1);
    let row = sqlx::query("SELECT watched,watched_at,rating,tags FROM episodes WHERE id=11")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.get::<i32, _>("watched"), 1);
    assert_eq!(row.get::<f64, _>("rating"), 4.5);
    assert_eq!(
        serde_json::from_str::<Vec<String>>(&row.get::<String, _>("tags")).unwrap(),
        vec!["keep", "merge"]
    );
    let parent: i64 = sqlx::query_scalar("SELECT show_id FROM sonarr_imports")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(parent, 1);
    sqlx::query("DELETE FROM shows WHERE id=1")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM movies WHERE id=3")
        .execute(&pool)
        .await
        .unwrap();
    let count:i64=sqlx::query_scalar("SELECT (SELECT COUNT(*) FROM sonarr_imports)+(SELECT COUNT(*) FROM radarr_imports)+(SELECT COUNT(*) FROM title_mappings)").fetch_one(&pool).await.unwrap();
    assert_eq!(count, 0);
}

const ICS: &str = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:race-1\r\nSUMMARY:Montréal: Race\r\nDTSTART:20990917T230000Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";

#[tokio::test]
async fn duplicate_merge_rolls_back_earlier_changes_if_final_delete_fails() {
    let pool = database().await;
    sqlx::raw_sql("INSERT INTO shows(id,name,notes) VALUES (1,'Keep','Original'),(2,'Merge','Other');
        INSERT INTO episodes(id,show_id,season_number,episode_number,watched,tags) VALUES
        (11,1,1,1,0,'[\"keep\"]'),(21,2,1,1,1,'[\"merge\"]'),(22,2,1,2,1,'[]');
        INSERT INTO title_mappings(plex_title,media_type,tvc_id,tvc_title) VALUES ('Mapped','show',2,'Merge');
        CREATE TRIGGER reject_merge BEFORE DELETE ON shows WHEN OLD.id=2 BEGIN SELECT RAISE(ABORT,'fixture failure'); END;")
        .execute(&pool).await.unwrap();
    assert!(crate::commands::duplicates::merge_in_pool(&pool, 1, 2)
        .await
        .is_err());
    let episodes: Vec<(i64, i64, i64, String)> =
        sqlx::query_as("SELECT id,show_id,watched,tags FROM episodes ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        episodes,
        vec![
            (11, 1, 0, "[\"keep\"]".into()),
            (21, 2, 1, "[\"merge\"]".into()),
            (22, 2, 1, "[]".into())
        ]
    );
    let shows: Vec<(i64, String)> = sqlx::query_as("SELECT id,notes FROM shows ORDER BY id")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(shows, vec![(1, "Original".into()), (2, "Other".into())]);
    let mapping: i64 = sqlx::query_scalar("SELECT tvc_id FROM title_mappings")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(mapping, 2);
}

#[tokio::test]
async fn racing_validates_before_replacement_and_preserves_ids_reminders() {
    use crate::racing::{api::parse_ics, replace_events};
    let pool = database().await;
    let events = parse_ics(ICS, "f1").unwrap();
    replace_events(&pool, "f1", &events).await.unwrap();
    sqlx::query("UPDATE racing_events SET notified=1")
        .execute(&pool)
        .await
        .unwrap();
    let id: i64 = sqlx::query_scalar("SELECT id FROM racing_events")
        .fetch_one(&pool)
        .await
        .unwrap();
    replace_events(&pool, "f1", &events).await.unwrap();
    let row = sqlx::query("SELECT id,notified FROM racing_events")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.get::<i64, _>("id"), id);
    assert_eq!(row.get::<i32, _>("notified"), 1);
    for bad in [
        "<html>Unavailable</html>",
        "BEGIN:VCALENDAR\r\nEND:VCALENDAR",
        &ICS.replace("SUMMARY:Montréal: Race\r\n", ""),
        &ICS.replace("20990917T230000Z", "éééééééééT230000Z"),
        &ICS.replace("20990917", "20990231"),
    ] {
        assert!(parse_ics(bad, "f1").is_err());
    }
    sqlx::raw_sql("CREATE TRIGGER reject_event BEFORE UPDATE ON racing_events BEGIN SELECT RAISE(ABORT,'test failure'); END;").execute(&pool).await.unwrap();
    assert!(replace_events(
        &pool,
        "f1",
        &parse_ics(&ICS.replace("230000", "220000"), "f1").unwrap()
    )
    .await
    .is_err());
    let time: String = sqlx::query_scalar("SELECT start_time FROM racing_events")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(time, "2099-09-17T23:00:00Z");
}

#[tokio::test]
async fn racing_range_includes_last_day_and_excludes_next_midnight() {
    let pool = database().await;
    sqlx::raw_sql(
        "UPDATE racing_series SET enabled=1 WHERE slug='f1';
        INSERT INTO racing_events(series_slug,uid,event_title,start_time) VALUES
        ('f1','1','Before','2026-09-16T21:59:59Z'),('f1','2','Start','2026-09-16T22:00:00Z'),
        ('f1','3','End','2026-09-17T21:59:59Z'),('f1','4','After','2026-09-17T22:00:00Z');",
    )
    .execute(&pool)
    .await
    .unwrap();
    let events =
        crate::racing::get_events_for_range(&pool, "2026-09-16T22:00:00Z", "2026-09-17T22:00:00Z")
            .await
            .unwrap();
    assert_eq!(
        events.iter().map(|e| e.uid.as_str()).collect::<Vec<_>>(),
        vec!["2", "3"]
    );
}

#[tokio::test]
async fn predictions_validate_membership_and_lock_revealed_or_past_categories() {
    use crate::awards::db::{clear_prediction, set_prediction};
    let pool = database().await;
    sqlx::raw_sql("INSERT INTO award_ceremonies(id,award_type,edition,name,year,ceremony_date,status,wiki_title) VALUES (1,'oscars',1,'Future',2099,'2099-09-17','nominated','Future');
        INSERT INTO award_categories(id,ceremony_id,name) VALUES (1,1,'A'),(2,1,'B');
        INSERT INTO award_nominees(id,category_id,title,source_key) VALUES (1,1,'A','a'),(2,2,'B','b');")
        .execute(&pool).await.unwrap();
    assert!(set_prediction(&pool, 1, 2).await.is_err());
    set_prediction(&pool, 1, 1).await.unwrap();
    clear_prediction(&pool, 1).await.unwrap();
    set_prediction(&pool, 1, 1).await.unwrap();
    sqlx::query("UPDATE award_nominees SET is_winner=1 WHERE id=1")
        .execute(&pool)
        .await
        .unwrap();
    assert!(set_prediction(&pool, 1, 1).await.is_err());
    assert!(clear_prediction(&pool, 1).await.is_err());
    sqlx::raw_sql("UPDATE award_nominees SET is_winner=NULL; UPDATE award_ceremonies SET ceremony_date='2020-01-01';").execute(&pool).await.unwrap();
    assert!(set_prediction(&pool, 1, 1).await.is_err());
}

#[tokio::test]
async fn statistics_include_the_whole_local_last_day() {
    let pool = database().await;
    sqlx::raw_sql(
        "INSERT INTO movies(id,title,watched,watched_at) VALUES
        (1,'Early',1,datetime('2026-09-17 00:00:00','utc')),
        (2,'Late',1,datetime('2026-09-17 23:59:59','utc')),
        (3,'Next',1,datetime('2026-09-18 00:00:00','utc'));",
    )
    .execute(&pool)
    .await
    .unwrap();
    let stats =
        crate::commands::statistics::period_stats(&pool, "2026-09-17", "2026-09-17", Some("day"))
            .await
            .unwrap();
    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0].movies_count, 2);
    assert_eq!(stats[0].date, "2026-09-17");
}
