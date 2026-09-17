//! User-owned records added in backup v3.0. Credentials and caches are excluded.
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqliteConnection};

use super::unique_ids;

#[derive(Debug, Serialize, Deserialize)]
pub struct PersonalData {
    pub ceremonies: Vec<Ceremony>,
    pub categories: Vec<Category>,
    pub nominees: Vec<Nominee>,
    pub predictions: Vec<Prediction>,
    pub title_mappings: Vec<TitleMapping>,
    pub scrobble_history: Vec<Scrobble>,
    pub change_history: Vec<Change>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Ceremony {
    pub id: i64,
    pub award_type: String,
    pub edition: i64,
    pub name: String,
    pub year: i64,
    pub ceremony_date: Option<String>,
    pub nominations_date: Option<String>,
    pub status: String,
    pub wiki_title: String,
    pub last_synced: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Category {
    pub id: i64,
    pub ceremony_id: i64,
    pub name: String,
    pub display_order: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Nominee {
    pub id: i64,
    pub category_id: i64,
    pub title: String,
    pub detail: Option<String>,
    pub is_winner: Option<i32>,
    pub source_key: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Prediction {
    pub id: i64,
    pub category_id: i64,
    pub nominee_id: i64,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct TitleMapping {
    pub id: i64,
    pub plex_title: String,
    pub media_type: String,
    pub tvc_id: i64,
    pub tvc_title: String,
    pub created_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Scrobble {
    pub id: i64,
    pub event_type: String,
    pub media_type: String,
    pub raw_title: String,
    pub show_name: Option<String>,
    pub season_number: Option<i32>,
    pub episode_number: Option<i32>,
    pub year: Option<i32>,
    pub matched_entity_type: Option<String>,
    pub matched_entity_id: Option<i64>,
    pub match_method: Option<String>,
    pub scrobbled_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Change {
    pub id: i64,
    pub entity_type: String,
    pub entity_id: i64,
    pub change_type: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub changed_at: Option<String>,
    pub user_action: Option<String>,
}

impl PersonalData {
    pub fn validate(&self) -> Result<(), String> {
        let ceremonies = unique_ids("ceremony", self.ceremonies.iter().map(|r| r.id))?;
        let categories = unique_ids("category", self.categories.iter().map(|r| r.id))?;
        unique_ids("nominee", self.nominees.iter().map(|r| r.id))?;
        unique_ids("prediction", self.predictions.iter().map(|r| r.id))?;
        unique_ids(
            "predicted category",
            self.predictions.iter().map(|r| r.category_id),
        )?;
        unique_ids("title mapping", self.title_mappings.iter().map(|r| r.id))?;
        unique_ids("scrobble", self.scrobble_history.iter().map(|r| r.id))?;
        unique_ids("change history", self.change_history.iter().map(|r| r.id))?;

        let nominees: std::collections::HashMap<_, _> = self
            .nominees
            .iter()
            .map(|n| (n.id, n.category_id))
            .collect();
        for category in &self.categories {
            if !ceremonies.contains(&category.ceremony_id) {
                return Err(format!(
                    "Category {} references a missing ceremony",
                    category.id
                ));
            }
        }
        for nominee in &self.nominees {
            if !categories.contains(&nominee.category_id) {
                return Err(format!(
                    "Nominee {} references a missing category",
                    nominee.id
                ));
            }
            if !matches!(nominee.is_winner, None | Some(0) | Some(1)) {
                return Err(format!("Nominee {} has an invalid result", nominee.id));
            }
        }
        for prediction in &self.predictions {
            if nominees.get(&prediction.nominee_id) != Some(&prediction.category_id) {
                return Err(format!(
                    "Prediction {} does not reference a nominee in its category",
                    prediction.id
                ));
            }
        }
        // History and title corrections can outlive deleted titles; preserve those
        // snapshots even when their target is no longer present in the library.
        Ok(())
    }

    pub async fn export(connection: &mut SqliteConnection) -> Result<Self, sqlx::Error> {
        Ok(Self {
            ceremonies: sqlx::query_as("SELECT * FROM award_ceremonies ORDER BY id")
                .fetch_all(&mut *connection)
                .await?,
            categories: sqlx::query_as("SELECT * FROM award_categories ORDER BY id")
                .fetch_all(&mut *connection)
                .await?,
            nominees: sqlx::query_as("SELECT * FROM award_nominees ORDER BY id")
                .fetch_all(&mut *connection)
                .await?,
            predictions: sqlx::query_as("SELECT * FROM award_predictions ORDER BY id")
                .fetch_all(&mut *connection)
                .await?,
            title_mappings: sqlx::query_as("SELECT * FROM title_mappings ORDER BY id")
                .fetch_all(&mut *connection)
                .await?,
            scrobble_history: sqlx::query_as("SELECT * FROM plex_scrobble_log ORDER BY id")
                .fetch_all(&mut *connection)
                .await?,
            change_history: sqlx::query_as("SELECT * FROM change_history ORDER BY id")
                .fetch_all(&mut *connection)
                .await?,
        })
    }

    pub async fn restore(&self, connection: &mut SqliteConnection) -> Result<(), sqlx::Error> {
        // Library-related auxiliary tables have already been cleared by the caller.
        // Replace awards only for v3 backups, which contain the complete snapshot.
        sqlx::query("DELETE FROM award_predictions")
            .execute(&mut *connection)
            .await?;
        sqlx::query("DELETE FROM award_nominees")
            .execute(&mut *connection)
            .await?;
        sqlx::query("DELETE FROM award_categories")
            .execute(&mut *connection)
            .await?;
        sqlx::query("DELETE FROM award_ceremonies")
            .execute(&mut *connection)
            .await?;

        for r in &self.ceremonies {
            sqlx::query("INSERT INTO award_ceremonies (id, award_type, edition, name, year, ceremony_date, nominations_date, status, wiki_title, last_synced) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
                .bind(r.id).bind(&r.award_type).bind(r.edition).bind(&r.name).bind(r.year)
                .bind(&r.ceremony_date).bind(&r.nominations_date).bind(&r.status).bind(&r.wiki_title).bind(&r.last_synced)
                .execute(&mut *connection).await?;
        }
        for r in &self.categories {
            sqlx::query("INSERT INTO award_categories (id, ceremony_id, name, display_order) VALUES (?, ?, ?, ?)")
                .bind(r.id).bind(r.ceremony_id).bind(&r.name).bind(r.display_order).execute(&mut *connection).await?;
        }
        for r in &self.nominees {
            sqlx::query("INSERT INTO award_nominees (id, category_id, title, detail, is_winner, source_key) VALUES (?, ?, ?, ?, ?, ?)")
                .bind(r.id).bind(r.category_id).bind(&r.title).bind(&r.detail).bind(r.is_winner).bind(&r.source_key)
                .execute(&mut *connection).await?;
        }
        for r in &self.predictions {
            sqlx::query("INSERT INTO award_predictions (id, category_id, nominee_id, created_at, updated_at) VALUES (?, ?, ?, ?, ?)")
                .bind(r.id).bind(r.category_id).bind(r.nominee_id).bind(&r.created_at).bind(&r.updated_at)
                .execute(&mut *connection).await?;
        }
        for r in &self.title_mappings {
            sqlx::query("INSERT INTO title_mappings (id, plex_title, media_type, tvc_id, tvc_title, created_at) VALUES (?, ?, ?, ?, ?, ?)")
                .bind(r.id).bind(&r.plex_title).bind(&r.media_type).bind(r.tvc_id).bind(&r.tvc_title).bind(&r.created_at)
                .execute(&mut *connection).await?;
        }
        for r in &self.scrobble_history {
            sqlx::query("INSERT INTO plex_scrobble_log (id, event_type, media_type, raw_title, show_name, season_number, episode_number, year, matched_entity_type, matched_entity_id, match_method, scrobbled_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
                .bind(r.id).bind(&r.event_type).bind(&r.media_type).bind(&r.raw_title).bind(&r.show_name)
                .bind(r.season_number).bind(r.episode_number).bind(r.year).bind(&r.matched_entity_type)
                .bind(r.matched_entity_id).bind(&r.match_method).bind(&r.scrobbled_at).execute(&mut *connection).await?;
        }
        for r in &self.change_history {
            sqlx::query("INSERT INTO change_history (id, entity_type, entity_id, change_type, old_value, new_value, changed_at, user_action) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
                .bind(r.id).bind(&r.entity_type).bind(r.entity_id).bind(&r.change_type).bind(&r.old_value)
                .bind(&r.new_value).bind(&r.changed_at).bind(&r.user_action).execute(&mut *connection).await?;
        }
        Ok(())
    }
}
