// Database operations module
// Will be implemented using tauri-plugin-sql

// The actual database operations are performed through the SQL plugin
// from the frontend. This module provides types and helper functions.

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DbShow {
    pub id: i64,
    pub name: String,
    pub slug: Option<String>,
    pub status: Option<String>,
    pub poster_url: Option<String>,
    pub first_aired: Option<String>,
    pub network: Option<String>,
    pub overview: Option<String>,
    pub airs_time: Option<String>,
    pub airs_days: Option<String>,
    pub runtime: Option<i32>,
    pub added_at: Option<String>,
    pub last_synced: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DbEpisode {
    pub id: i64,
    pub show_id: i64,
    pub season_number: Option<i32>,
    pub episode_number: Option<i32>,
    pub name: Option<String>,
    pub overview: Option<String>,
    pub aired: Option<String>,
    pub runtime: Option<i32>,
    pub image_url: Option<String>,
    pub watched: bool,
    pub watched_at: Option<String>,
}
