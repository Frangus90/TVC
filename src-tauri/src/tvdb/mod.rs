pub mod cache;
pub mod rate_limit;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::RwLock;

/// Create an HTTP client with proper timeout configuration
fn create_http_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| Client::new()) // Fallback to default if builder fails
}

/// Invalidate cache for a specific show (call before sync to force fresh data)
pub async fn invalidate_show_cache(id: i64) {
    get_cache().invalidate_show(id).await;
}

/// Clear all caches (call before full resync)
pub async fn clear_all_caches() {
    get_cache().clear_all().await;
}

const API_BASE: &str = "https://api4.thetvdb.com/v4";
const API_KEY: &str = "a3ceb063-8688-4916-9c1d-8f8039e87307";

static TOKEN: OnceLock<RwLock<Option<String>>> = OnceLock::new();
static CACHE: OnceLock<cache::ApiCache> = OnceLock::new();
static RATE_LIMITER: OnceLock<rate_limit::RateLimiter> = OnceLock::new();

fn get_cache() -> &'static cache::ApiCache {
    CACHE.get_or_init(|| cache::ApiCache::new())
}

fn get_rate_limiter() -> &'static rate_limit::RateLimiter {
    RATE_LIMITER.get_or_init(|| rate_limit::RateLimiter::default())
}

async fn retry_with_backoff<F, Fut, T>(
    mut f: F,
    max_retries: u32,
) -> Result<T, Box<dyn std::error::Error + Send + Sync>>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, Box<dyn std::error::Error + Send + Sync>>>,
{
    let mut last_error = None;
    for attempt in 0..=max_retries {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_error = Some(e);
                if attempt < max_retries {
                    let delay = Duration::from_millis(100 * 2_u64.pow(attempt));
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }
    Err(last_error.expect("retry_with_backoff: last_error should always be Some after loop"))
}

fn get_token_lock() -> &'static RwLock<Option<String>> {
    TOKEN.get_or_init(|| RwLock::new(None))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub original_network: Option<NetworkInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInfo {
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirsDays {
    pub sunday: Option<bool>,
    pub monday: Option<bool>,
    pub tuesday: Option<bool>,
    pub wednesday: Option<bool>,
    pub thursday: Option<bool>,
    pub friday: Option<bool>,
    pub saturday: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

    let client = create_http_client();
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
    // Check cache first
    let cache = get_cache();
    if let Some(cached) = cache.get_search(query).await {
        return Ok(cached);
    }

    // Rate limiting
    get_rate_limiter().wait_if_needed().await;

    let token = get_token().await?;
    let query_str = query.to_string();

    let response = retry_with_backoff(
        || {
            let token = token.clone();
            let query = query_str.clone();
            async move {
                let client = create_http_client();
                let response = client
                    .get(format!("{}/search", API_BASE))
                    .query(&[("query", query.as_str()), ("type", "series")])
                    .bearer_auth(&token)
                    .send()
                    .await?;

                if !response.status().is_success() {
                    return Err(format!("API error: {}", response.status()).into());
                }

                Ok(response)
            }
        },
        3,
    )
    .await?;

    let search_response: SearchResponse = response.json().await?;
    let results = search_response.data;

    // Cache the results
    cache.set_search(query.to_string(), results.clone()).await;

    Ok(results)
}

pub async fn get_series_extended(
    id: i64,
) -> Result<SeriesExtended, Box<dyn std::error::Error + Send + Sync>> {
    // Check cache first
    let cache = get_cache();
    if let Some(cached) = cache.get_show(id).await {
        return Ok(cached);
    }

    // Rate limiting
    get_rate_limiter().wait_if_needed().await;

    let token = get_token().await?;

    let response = retry_with_backoff(
        || {
            let token = token.clone();
            let id = id;
            async move {
                let client = create_http_client();
                let response = client
                    .get(format!("{}/series/{}/extended", API_BASE, id))
                    .bearer_auth(&token)
                    .send()
                    .await?;

                if !response.status().is_success() {
                    return Err(format!("API error: {}", response.status()).into());
                }

                Ok(response)
            }
        },
        3,
    )
    .await?;

    let api_response: ApiResponse<SeriesExtended> = response.json().await?;
    let show = api_response.data;

    // Cache the result
    cache.set_show(id, show.clone()).await;

    Ok(show)
}

pub async fn get_series_episodes(
    id: i64,
) -> Result<Vec<EpisodeBase>, Box<dyn std::error::Error + Send + Sync>> {
    // Check cache first
    let cache = get_cache();
    if let Some(cached) = cache.get_episodes(id).await {
        return Ok(cached);
    }

    // Rate limiting
    get_rate_limiter().wait_if_needed().await;

    let token = get_token().await?;
    let client = create_http_client();

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

    // Cache the results
    cache.set_episodes(id, all_episodes.clone()).await;

    Ok(all_episodes)
}

// Character/cast member from TVDB
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Character {
    pub id: i64,
    pub name: Option<String>,
    pub people_id: Option<i64>,
    pub person_name: Option<String>,
    pub image: Option<String>,
    pub sort: Option<i32>,
}

// Show characters response
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowCharacters {
    pub characters: Vec<Character>,
}

/// Get characters/cast for a show
pub async fn get_series_characters(
    id: i64,
) -> Result<Vec<Character>, Box<dyn std::error::Error + Send + Sync>> {
    // Rate limiting
    get_rate_limiter().wait_if_needed().await;

    let token = get_token().await?;
    let client = create_http_client();

    // Get series extended with characters
    let response = client
        .get(format!("{}/series/{}/extended", API_BASE, id))
        .query(&[("meta", "characters")])
        .bearer_auth(&token)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("TVDB API error: {}", response.status()).into());
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct SeriesWithCharacters {
        characters: Option<Vec<Character>>,
    }

    #[derive(Deserialize)]
    struct CharactersApiResponse {
        data: SeriesWithCharacters,
    }

    let api_response: CharactersApiResponse = response.json().await?;
    let mut characters = api_response.data.characters.unwrap_or_default();

    // Sort by sort order
    characters.sort_by_key(|c| c.sort.unwrap_or(999));

    Ok(characters)
}
