use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use tokio::sync::RwLock;

const API_BASE: &str = "https://api4.thetvdb.com/v4";
const API_KEY: &str = "a3ceb063-8688-4916-9c1d-8f8039e87307";

static TOKEN: OnceLock<RwLock<Option<String>>> = OnceLock::new();

fn get_token_lock() -> &'static RwLock<Option<String>> {
    TOKEN.get_or_init(|| RwLock::new(None))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub tvdb_id: Option<String>,
    pub id: Option<String>,
    pub name: Option<String>,
    pub slug: Option<String>,
    pub image_url: Option<String>,
    pub status: Option<String>,
    pub first_air_time: Option<String>,
    pub overview: Option<String>,
    pub network: Option<String>,
    pub year: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesExtended {
    pub id: i64,
    pub name: String,
    pub slug: Option<String>,
    pub image: Option<String>,
    pub status: Option<Status>,
    pub first_aired: Option<String>,
    pub overview: Option<String>,
    pub airs_time: Option<String>,
    pub airs_days: Option<AirsDays>,
    pub average_runtime: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Status {
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AirsDays {
    pub sunday: Option<bool>,
    pub monday: Option<bool>,
    pub tuesday: Option<bool>,
    pub wednesday: Option<bool>,
    pub thursday: Option<bool>,
    pub friday: Option<bool>,
    pub saturday: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EpisodeBase {
    pub id: i64,
    #[serde(alias = "seriesId")]
    pub series_id: Option<i64>,
    pub name: Option<String>,
    pub aired: Option<String>,
    pub runtime: Option<i32>,
    pub image: Option<String>,
    #[serde(alias = "seasonNumber")]
    pub season_number: Option<i32>,
    #[serde(alias = "number", alias = "episodeNumber")]
    pub episode_number: Option<i32>,
    pub overview: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SeasonRecord {
    id: i64,
    number: Option<i32>,
    #[serde(rename = "type")]
    season_type: Option<SeasonType>,
}

#[derive(Debug, Deserialize)]
struct SeasonType {
    id: Option<i64>,
    name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LoginRequest {
    apikey: String,
}

#[derive(Debug, Deserialize)]
struct LoginResponse {
    data: LoginData,
}

#[derive(Debug, Deserialize)]
struct LoginData {
    token: String,
}

#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    data: T,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    data: Vec<SearchResult>,
}

async fn get_token() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    {
        let token_guard = get_token_lock().read().await;
        if let Some(ref token) = *token_guard {
            return Ok(token.clone());
        }
    }

    let client = Client::new();
    let login_req = LoginRequest {
        apikey: API_KEY.to_string(),
    };

    let response = client
        .post(format!("{}/login", API_BASE))
        .json(&login_req)
        .send()
        .await?;

    let login_response: LoginResponse = response.json().await?;
    let token = login_response.data.token;

    {
        let mut token_guard = get_token_lock().write().await;
        *token_guard = Some(token.clone());
    }

    Ok(token)
}

pub async fn search_series(
    query: &str,
) -> Result<Vec<SearchResult>, Box<dyn std::error::Error + Send + Sync>> {
    let token = get_token().await?;
    let client = Client::new();

    let response = client
        .get(format!("{}/search", API_BASE))
        .query(&[("query", query), ("type", "series")])
        .bearer_auth(&token)
        .send()
        .await?;

    let search_response: SearchResponse = response.json().await?;
    Ok(search_response.data)
}

pub async fn get_series_extended(
    id: i64,
) -> Result<SeriesExtended, Box<dyn std::error::Error + Send + Sync>> {
    let token = get_token().await?;
    let client = Client::new();

    let response = client
        .get(format!("{}/series/{}/extended", API_BASE, id))
        .bearer_auth(&token)
        .send()
        .await?;

    let api_response: ApiResponse<SeriesExtended> = response.json().await?;
    Ok(api_response.data)
}

pub async fn get_series_episodes(
    id: i64,
) -> Result<Vec<EpisodeBase>, Box<dyn std::error::Error + Send + Sync>> {
    let token = get_token().await?;
    let client = Client::new();

    // Get all seasons for this series
    let url = format!("{}/series/{}/extended", API_BASE, id);
    let seasons_response = client
        .get(&url)
        .query(&[("meta", "episodes"), ("short", "true")])
        .bearer_auth(&token)
        .send()
        .await?;

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct SeriesWithSeasons {
        seasons: Option<Vec<SeasonRecord>>,
    }

    #[derive(Deserialize)]
    struct SeasonsApiResponse {
        data: SeriesWithSeasons,
    }

    let seasons_data: SeasonsApiResponse = seasons_response.json().await?;

    // Filter to only "official" seasons (type id 1)
    let official_seasons: Vec<&SeasonRecord> = seasons_data
        .data
        .seasons
        .as_ref()
        .map(|s| {
            s.iter()
                .filter(|season| {
                    season.season_type.as_ref().map(|t| t.id == Some(1)).unwrap_or(false)
                })
                .collect()
        })
        .unwrap_or_default();

    let mut all_episodes = Vec::new();

    // Fetch episodes for each official season
    for season in official_seasons {
        let season_num = season.number.unwrap_or(0);
        let season_url = format!("{}/seasons/{}/extended", API_BASE, season.id);

        let response = client
            .get(&season_url)
            .bearer_auth(&token)
            .send()
            .await?;

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct SeasonExtended {
            episodes: Option<Vec<EpisodeBase>>,
        }

        #[derive(Deserialize)]
        struct SeasonApiResponse {
            data: SeasonExtended,
        }

        if let Ok(season_response) = response.json::<SeasonApiResponse>().await {
            if let Some(mut episodes) = season_response.data.episodes {
                // Set season/episode number if missing
                for (idx, ep) in episodes.iter_mut().enumerate() {
                    if ep.season_number.is_none() {
                        ep.season_number = Some(season_num);
                    }
                    if ep.episode_number.is_none() {
                        ep.episode_number = Some((idx + 1) as i32);
                    }
                }
                all_episodes.extend(episodes);
            }
        }
    }

    // Sort by season, then episode number
    all_episodes.sort_by(|a, b| {
        let season_cmp = a.season_number.unwrap_or(0).cmp(&b.season_number.unwrap_or(0));
        if season_cmp == std::cmp::Ordering::Equal {
            a.episode_number.unwrap_or(0).cmp(&b.episode_number.unwrap_or(0))
        } else {
            season_cmp
        }
    });

    Ok(all_episodes)
}
