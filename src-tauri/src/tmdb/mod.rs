use moka::future::Cache;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use std::time::Duration;

/// Create an HTTP client with proper timeout configuration
fn create_http_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| Client::new()) // Fallback to default if builder fails
}

const API_BASE: &str = "https://api.themoviedb.org/3";
const IMAGE_BASE: &str = "https://image.tmdb.org/t/p/w500";
const READ_ACCESS_TOKEN: &str = "eyJhbGciOiJIUzI1NiJ9.eyJhdWQiOiI4MzU5NzQ2YTgyMTY5ZDQ1NjZjMDJiM2Q3NWYyMDMwYiIsIm5iZiI6MTc2NTcwNzM3Mi45Miwic3ViIjoiNjkzZThlNmMyY2M2NDVlOWUxNWUwMTk4Iiwic2NvcGVzIjpbImFwaV9yZWFkIl0sInZlcnNpb24iOjF9.NsOWeumDwHhAxu2KHDXY6_HhLAPX5JEa3tVbZHVTWtU";

// Cache for API responses
static CACHE: OnceLock<TmdbCache> = OnceLock::new();

fn get_cache() -> &'static TmdbCache {
    CACHE.get_or_init(TmdbCache::new)
}

struct TmdbCache {
    search_cache: Cache<String, Vec<MovieSearchResult>>,
    movie_cache: Cache<i64, MovieDetails>,
    tv_search_cache: Cache<String, Vec<TvShowSearchResult>>,
    tv_details_cache: Cache<i64, TvShowDetails>,
    tv_season_cache: Cache<(i64, i32), TvSeasonDetails>,
    tv_credits_cache: Cache<i64, TvCredits>,
}

impl TmdbCache {
    fn new() -> Self {
        Self {
            // Search results: 1 hour TTL
            search_cache: Cache::builder()
                .time_to_live(Duration::from_secs(3600))
                .build(),
            // Movie details: 24 hours TTL
            movie_cache: Cache::builder()
                .time_to_live(Duration::from_secs(86400))
                .build(),
            // TV search: 1 hour TTL
            tv_search_cache: Cache::builder()
                .time_to_live(Duration::from_secs(3600))
                .build(),
            // TV details: 24 hours TTL
            tv_details_cache: Cache::builder()
                .time_to_live(Duration::from_secs(86400))
                .build(),
            // TV seasons: 6 hours TTL
            tv_season_cache: Cache::builder()
                .time_to_live(Duration::from_secs(21600))
                .build(),
            // TV credits: 24 hours TTL
            tv_credits_cache: Cache::builder()
                .time_to_live(Duration::from_secs(86400))
                .build(),
        }
    }
}

// Search result from TMDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovieSearchResult {
    pub id: i64,
    pub title: String,
    pub original_title: Option<String>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub release_date: Option<String>,
    pub vote_average: Option<f64>,
    pub vote_count: Option<i64>,
    pub popularity: Option<f64>,
    pub adult: Option<bool>,
    pub genre_ids: Option<Vec<i64>>,
}

impl MovieSearchResult {
    pub fn poster_url(&self) -> Option<String> {
        self.poster_path.as_ref().map(|p| format!("{}{}", IMAGE_BASE, p))
    }
}

// Full movie details from TMDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovieDetails {
    pub id: i64,
    pub title: String,
    pub original_title: Option<String>,
    pub tagline: Option<String>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub release_date: Option<String>,
    pub runtime: Option<i32>,
    pub status: Option<String>,
    pub vote_average: Option<f64>,
    pub vote_count: Option<i64>,
    pub budget: Option<i64>,
    pub revenue: Option<i64>,
    pub genres: Option<Vec<Genre>>,
    pub production_companies: Option<Vec<ProductionCompany>>,
    pub imdb_id: Option<String>,
}

impl MovieDetails {
    pub fn poster_url(&self) -> Option<String> {
        self.poster_path.as_ref().map(|p| format!("{}{}", IMAGE_BASE, p))
    }

    pub fn backdrop_url(&self) -> Option<String> {
        self.backdrop_path.as_ref().map(|p| format!("https://image.tmdb.org/t/p/w1280{}", p))
    }

    pub fn genres_string(&self) -> Option<String> {
        self.genres.as_ref().map(|g| {
            g.iter()
                .filter_map(|genre| genre.name.clone())
                .collect::<Vec<_>>()
                .join(", ")
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Genre {
    pub id: i64,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionCompany {
    pub id: i64,
    pub name: Option<String>,
    pub logo_path: Option<String>,
    pub origin_country: Option<String>,
}

// Release dates response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseDatesResponse {
    pub id: i64,
    pub results: Vec<CountryReleaseDates>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountryReleaseDates {
    pub iso_3166_1: String,
    pub release_dates: Vec<ReleaseDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseDate {
    pub certification: Option<String>,
    pub iso_639_1: Option<String>,
    pub note: Option<String>,
    pub release_date: String,
    #[serde(rename = "type")]
    pub release_type: i32, // 1=Premiere, 2=Theatrical limited, 3=Theatrical, 4=Digital, 5=Physical, 6=TV
}

// Parsed release dates for a movie
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MovieReleaseDates {
    pub theatrical: Option<String>,
    pub digital: Option<String>,
    pub physical: Option<String>,
}

// API response wrappers
#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[allow(dead_code)]
    page: i32,
    results: Vec<MovieSearchResult>,
    #[allow(dead_code)]
    total_pages: i32,
    #[allow(dead_code)]
    total_results: i32,
}

/// Search for movies by title
pub async fn search_movies(
    query: &str,
) -> Result<Vec<MovieSearchResult>, Box<dyn std::error::Error + Send + Sync>> {
    // Check cache first
    let cache = get_cache();
    if let Some(cached) = cache.search_cache.get(query).await {
        return Ok(cached);
    }

    let client = create_http_client();
    let response = client
        .get(format!("{}/search/movie", API_BASE))
        .query(&[("query", query), ("include_adult", "false")])
        .bearer_auth(READ_ACCESS_TOKEN)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("TMDB API error: {}", response.status()).into());
    }

    let search_response: SearchResponse = response.json().await?;
    let results = search_response.results;

    // Cache the results
    cache.search_cache.insert(query.to_string(), results.clone()).await;

    Ok(results)
}

/// Get detailed movie information
pub async fn get_movie_details(
    id: i64,
) -> Result<MovieDetails, Box<dyn std::error::Error + Send + Sync>> {
    // Check cache first
    let cache = get_cache();
    if let Some(cached) = cache.movie_cache.get(&id).await {
        return Ok(cached);
    }

    let client = create_http_client();
    let response = client
        .get(format!("{}/movie/{}", API_BASE, id))
        .bearer_auth(READ_ACCESS_TOKEN)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("TMDB API error: {}", response.status()).into());
    }

    let movie: MovieDetails = response.json().await?;

    // Cache the result
    cache.movie_cache.insert(id, movie.clone()).await;

    Ok(movie)
}

/// Get release dates for a movie (theatrical, digital, physical)
pub async fn get_movie_release_dates(
    id: i64,
    country: &str, // ISO 3166-1 country code, e.g., "US"
) -> Result<MovieReleaseDates, Box<dyn std::error::Error + Send + Sync>> {
    let client = create_http_client();
    let response = client
        .get(format!("{}/movie/{}/release_dates", API_BASE, id))
        .bearer_auth(READ_ACCESS_TOKEN)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("TMDB API error: {}", response.status()).into());
    }

    let release_data: ReleaseDatesResponse = response.json().await?;

    // Find release dates for the specified country
    let country_releases = release_data
        .results
        .iter()
        .find(|r| r.iso_3166_1 == country);

    let mut dates = MovieReleaseDates::default();

    if let Some(country_data) = country_releases {
        for release in &country_data.release_dates {
            // Extract just the date part (YYYY-MM-DD) from the ISO datetime
            let date = release.release_date.split('T').next().unwrap_or(&release.release_date);

            match release.release_type {
                1 | 2 | 3 => {
                    // Premiere, Theatrical limited, or Theatrical
                    if dates.theatrical.is_none() {
                        dates.theatrical = Some(date.to_string());
                    }
                }
                4 => {
                    // Digital
                    if dates.digital.is_none() {
                        dates.digital = Some(date.to_string());
                    }
                }
                5 => {
                    // Physical
                    if dates.physical.is_none() {
                        dates.physical = Some(date.to_string());
                    }
                }
                _ => {}
            }
        }
    }

    // If no country-specific data, try to use the primary release_date from movie details
    if dates.theatrical.is_none() {
        if let Ok(movie) = get_movie_details(id).await {
            dates.theatrical = movie.release_date;
        }
    }

    Ok(dates)
}

/// Get movie details with release dates combined
pub async fn get_movie_with_release_dates(
    id: i64,
    country: &str,
) -> Result<(MovieDetails, MovieReleaseDates), Box<dyn std::error::Error + Send + Sync>> {
    let (movie, release_dates) = tokio::join!(
        get_movie_details(id),
        get_movie_release_dates(id, country)
    );

    Ok((movie?, release_dates?))
}

// Cast member from TMDB credits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CastMember {
    pub id: i64,
    pub name: String,
    pub character: Option<String>,
    pub profile_path: Option<String>,
    pub order: Option<i32>,
}

impl CastMember {
    pub fn image_url(&self) -> Option<String> {
        self.profile_path.as_ref().map(|p| format!("{}{}", IMAGE_BASE, p))
    }
}

// Crew member from TMDB credits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrewMember {
    pub id: i64,
    pub name: String,
    pub job: Option<String>,
    pub department: Option<String>,
    pub profile_path: Option<String>,
}

impl CrewMember {
    pub fn image_url(&self) -> Option<String> {
        self.profile_path.as_ref().map(|p| format!("{}{}", IMAGE_BASE, p))
    }
}

// Credits response from TMDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovieCredits {
    pub id: i64,
    pub cast: Vec<CastMember>,
    pub crew: Vec<CrewMember>,
}

/// Get cast and crew for a movie
pub async fn get_movie_credits(
    id: i64,
) -> Result<MovieCredits, Box<dyn std::error::Error + Send + Sync>> {
    let client = create_http_client();
    let response = client
        .get(format!("{}/movie/{}/credits", API_BASE, id))
        .bearer_auth(READ_ACCESS_TOKEN)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("TMDB API error: {}", response.status()).into());
    }

    let credits: MovieCredits = response.json().await?;
    Ok(credits)
}

// Video/trailer from TMDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Video {
    pub id: String,
    pub key: String,
    pub name: String,
    pub site: String,
    #[serde(rename = "type")]
    pub video_type: String,
    pub official: Option<bool>,
}

impl Video {
    pub fn youtube_url(&self) -> Option<String> {
        if self.site == "YouTube" {
            Some(format!("https://www.youtube.com/watch?v={}", self.key))
        } else {
            None
        }
    }

    pub fn youtube_embed_url(&self) -> Option<String> {
        if self.site == "YouTube" {
            Some(format!("https://www.youtube.com/embed/{}", self.key))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VideosResponse {
    results: Vec<Video>,
}

/// Get videos/trailers for a movie
pub async fn get_movie_videos(
    id: i64,
) -> Result<Vec<Video>, Box<dyn std::error::Error + Send + Sync>> {
    let client = create_http_client();
    let response = client
        .get(format!("{}/movie/{}/videos", API_BASE, id))
        .bearer_auth(READ_ACCESS_TOKEN)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("TMDB API error: {}", response.status()).into());
    }

    let videos: VideosResponse = response.json().await?;
    Ok(videos.results)
}

/// Get the main trailer for a movie (prefers official trailers)
pub async fn get_movie_trailer(
    id: i64,
) -> Result<Option<Video>, Box<dyn std::error::Error + Send + Sync>> {
    let videos = get_movie_videos(id).await?;

    // Prefer official trailers from YouTube
    let trailer = videos
        .iter()
        .filter(|v| v.site == "YouTube" && v.video_type == "Trailer")
        .max_by_key(|v| v.official.unwrap_or(false) as i32)
        .cloned();

    // Fallback to any YouTube video
    if trailer.is_some() {
        return Ok(trailer);
    }

    Ok(videos.iter().find(|v| v.site == "YouTube").cloned())
}

/// Get videos/trailers for a TV show
pub async fn get_tv_videos(
    id: i64,
) -> Result<Vec<Video>, Box<dyn std::error::Error + Send + Sync>> {
    let client = create_http_client();
    let response = client
        .get(format!("{}/tv/{}/videos", API_BASE, id))
        .bearer_auth(READ_ACCESS_TOKEN)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("TMDB API error: {}", response.status()).into());
    }

    let videos: VideosResponse = response.json().await?;
    Ok(videos.results)
}

/// Get the main trailer for a TV show (prefers official trailers)
pub async fn get_tv_trailer(
    id: i64,
) -> Result<Option<Video>, Box<dyn std::error::Error + Send + Sync>> {
    let videos = get_tv_videos(id).await?;

    // Prefer official trailers from YouTube
    let trailer = videos
        .iter()
        .filter(|v| v.site == "YouTube" && v.video_type == "Trailer")
        .max_by_key(|v| v.official.unwrap_or(false) as i32)
        .cloned();

    if trailer.is_some() {
        return Ok(trailer);
    }

    // Fallback to any YouTube video (teaser, featurette, etc.)
    Ok(videos.iter().find(|v| v.site == "YouTube").cloned())
}

// ---------------------------------------------------------------------------
// Full TV show support (search, details, seasons, episodes, credits, external)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvShowSearchResult {
    pub id: i64,
    pub name: String,
    pub original_name: Option<String>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub first_air_date: Option<String>,
    pub vote_average: Option<f64>,
    pub vote_count: Option<i64>,
    pub popularity: Option<f64>,
    pub origin_country: Option<Vec<String>>,
    pub genre_ids: Option<Vec<i64>>,
}

impl TvShowSearchResult {
    pub fn poster_url(&self) -> Option<String> {
        self.poster_path
            .as_ref()
            .map(|p| format!("{}{}", IMAGE_BASE, p))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvNetwork {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub logo_path: Option<String>,
    pub origin_country: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvSeasonSummary {
    pub id: i64,
    pub season_number: i32,
    pub episode_count: Option<i32>,
    pub air_date: Option<String>,
    pub name: Option<String>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvShowDetails {
    pub id: i64,
    pub name: String,
    pub original_name: Option<String>,
    pub tagline: Option<String>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub first_air_date: Option<String>,
    pub last_air_date: Option<String>,
    pub status: Option<String>,
    pub vote_average: Option<f64>,
    pub vote_count: Option<i64>,
    pub episode_run_time: Option<Vec<i32>>,
    pub number_of_seasons: Option<i32>,
    pub number_of_episodes: Option<i32>,
    pub genres: Option<Vec<Genre>>,
    pub networks: Option<Vec<TvNetwork>>,
    pub seasons: Option<Vec<TvSeasonSummary>>,
    pub homepage: Option<String>,
    pub in_production: Option<bool>,
    pub original_language: Option<String>,
    pub origin_country: Option<Vec<String>>,
}

impl TvShowDetails {
    pub fn poster_url(&self) -> Option<String> {
        self.poster_path
            .as_ref()
            .map(|p| format!("{}{}", IMAGE_BASE, p))
    }

    /// Best-effort runtime in minutes (TMDB returns an array; take the first value).
    pub fn runtime(&self) -> Option<i32> {
        self.episode_run_time
            .as_ref()
            .and_then(|v| v.iter().copied().find(|r| *r > 0))
    }

    /// Primary network name (first entry, if any).
    pub fn network_name(&self) -> Option<String> {
        self.networks
            .as_ref()
            .and_then(|n| n.first())
            .and_then(|n| n.name.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvEpisode {
    pub id: i64,
    pub name: Option<String>,
    pub overview: Option<String>,
    pub air_date: Option<String>,
    pub episode_number: i32,
    pub season_number: i32,
    pub runtime: Option<i32>,
    pub still_path: Option<String>,
    pub vote_average: Option<f64>,
    pub vote_count: Option<i64>,
}

impl TvEpisode {
    pub fn image_url(&self) -> Option<String> {
        self.still_path
            .as_ref()
            .map(|p| format!("{}{}", IMAGE_BASE, p))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvSeasonDetails {
    pub id: i64,
    pub season_number: i32,
    pub name: Option<String>,
    pub overview: Option<String>,
    pub air_date: Option<String>,
    pub poster_path: Option<String>,
    #[serde(default)]
    pub episodes: Vec<TvEpisode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvCastMember {
    pub id: i64,
    pub name: String,
    pub character: Option<String>,
    pub profile_path: Option<String>,
    pub order: Option<i32>,
}

impl TvCastMember {
    pub fn image_url(&self) -> Option<String> {
        self.profile_path
            .as_ref()
            .map(|p| format!("{}{}", IMAGE_BASE, p))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvCrewMember {
    pub id: i64,
    pub name: String,
    pub job: Option<String>,
    pub department: Option<String>,
    pub profile_path: Option<String>,
}

impl TvCrewMember {
    pub fn image_url(&self) -> Option<String> {
        self.profile_path
            .as_ref()
            .map(|p| format!("{}{}", IMAGE_BASE, p))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvCredits {
    pub id: i64,
    #[serde(default)]
    pub cast: Vec<TvCastMember>,
    #[serde(default)]
    pub crew: Vec<TvCrewMember>,
}

/// Response from `GET /find/{external_id}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalIdFindResponse {
    #[serde(default)]
    pub movie_results: Vec<MovieSearchResult>,
    #[serde(default)]
    pub tv_results: Vec<TvShowSearchResult>,
}

#[derive(Debug, Deserialize)]
struct TvShowSearchResponse {
    results: Vec<TvShowSearchResult>,
}

/// Preferred display language for TMDB metadata. Anything missing in this
/// language falls back to the show's `original_language`.
const PRIMARY_LANGUAGE: &str = "en-US";

fn is_blank_opt(s: &Option<String>) -> bool {
    s.as_ref().map_or(true, |v| v.trim().is_empty())
}

/// Search for TV shows by title (cached).
pub async fn search_tv(
    query: &str,
) -> Result<Vec<TvShowSearchResult>, Box<dyn std::error::Error + Send + Sync>> {
    let cache = get_cache();
    if let Some(cached) = cache.tv_search_cache.get(query).await {
        return Ok(cached);
    }

    let client = create_http_client();
    let response = client
        .get(format!("{}/search/tv", API_BASE))
        .query(&[
            ("query", query),
            ("include_adult", "false"),
            ("language", PRIMARY_LANGUAGE),
        ])
        .bearer_auth(READ_ACCESS_TOKEN)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("TMDB API error: {}", response.status()).into());
    }

    let search: TvShowSearchResponse = response.json().await?;
    let results = search.results;

    cache
        .tv_search_cache
        .insert(query.to_string(), results.clone())
        .await;

    Ok(results)
}

async fn fetch_tv_details_in(
    id: i64,
    language: &str,
) -> Result<TvShowDetails, Box<dyn std::error::Error + Send + Sync>> {
    let client = create_http_client();
    let response = client
        .get(format!("{}/tv/{}", API_BASE, id))
        .bearer_auth(READ_ACCESS_TOKEN)
        .query(&[("language", language)])
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("TMDB API error: {}", response.status()).into());
    }

    Ok(response.json().await?)
}

/// Fetch full TV show details (cached, 24h). Tries `en-US` first and falls
/// back to the show's `original_language` for any blank `name` / `overview`
/// — so anime that only have Japanese metadata still render with the
/// original title rather than empty fields.
pub async fn get_tv_details(
    id: i64,
) -> Result<TvShowDetails, Box<dyn std::error::Error + Send + Sync>> {
    let cache = get_cache();
    if let Some(cached) = cache.tv_details_cache.get(&id).await {
        return Ok(cached);
    }

    let mut details = fetch_tv_details_in(id, PRIMARY_LANGUAGE).await?;

    let needs_fallback = details.name.trim().is_empty() || is_blank_opt(&details.overview);
    if needs_fallback {
        if let Some(orig) = details.original_language.clone() {
            if !orig.is_empty() && orig != "en" {
                if let Ok(orig_details) = fetch_tv_details_in(id, &orig).await {
                    if details.name.trim().is_empty() {
                        details.name = orig_details.name;
                    }
                    if is_blank_opt(&details.overview) {
                        details.overview = orig_details.overview;
                    }
                }
            }
        }
    }

    cache.tv_details_cache.insert(id, details.clone()).await;
    Ok(details)
}

async fn fetch_tv_season_in(
    tv_id: i64,
    season_number: i32,
    language: &str,
) -> Result<TvSeasonDetails, Box<dyn std::error::Error + Send + Sync>> {
    let client = create_http_client();
    let response = client
        .get(format!(
            "{}/tv/{}/season/{}",
            API_BASE, tv_id, season_number
        ))
        .bearer_auth(READ_ACCESS_TOKEN)
        .query(&[("language", language)])
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("TMDB API error: {}", response.status()).into());
    }

    Ok(response.json().await?)
}

/// Fetch a single season's full details, including episode list (cached, 6h).
/// Like `get_tv_details`, falls back to the show's `original_language` for
/// blank season/episode names and overviews.
pub async fn get_tv_season(
    tv_id: i64,
    season_number: i32,
) -> Result<TvSeasonDetails, Box<dyn std::error::Error + Send + Sync>> {
    let cache = get_cache();
    let key = (tv_id, season_number);
    if let Some(cached) = cache.tv_season_cache.get(&key).await {
        return Ok(cached);
    }

    let mut season = fetch_tv_season_in(tv_id, season_number, PRIMARY_LANGUAGE).await?;

    let needs_fallback = is_blank_opt(&season.overview)
        || season
            .episodes
            .iter()
            .any(|e| is_blank_opt(&e.name) || is_blank_opt(&e.overview));

    if needs_fallback {
        // Look up the show's original language; this call is cached after the
        // first hit so the overhead is one extra request per show, not per season.
        if let Ok(details) = get_tv_details(tv_id).await {
            if let Some(orig) = details.original_language {
                if !orig.is_empty() && orig != "en" {
                    if let Ok(orig_season) =
                        fetch_tv_season_in(tv_id, season_number, &orig).await
                    {
                        if is_blank_opt(&season.overview) {
                            season.overview = orig_season.overview;
                        }
                        for ep in &mut season.episodes {
                            if !(is_blank_opt(&ep.name) || is_blank_opt(&ep.overview)) {
                                continue;
                            }
                            if let Some(orig_ep) = orig_season.episodes.iter().find(|oe| {
                                oe.season_number == ep.season_number
                                    && oe.episode_number == ep.episode_number
                            }) {
                                if is_blank_opt(&ep.name) {
                                    ep.name = orig_ep.name.clone();
                                }
                                if is_blank_opt(&ep.overview) {
                                    ep.overview = orig_ep.overview.clone();
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    cache.tv_season_cache.insert(key, season.clone()).await;
    Ok(season)
}

/// Fan-out all seasons (including season 0 / specials) and return a flat,
/// sorted list of episodes. Failures on individual seasons are skipped so a
/// single 404 doesn't blow up the whole fetch.
pub async fn get_tv_episodes(
    tv_id: i64,
) -> Result<Vec<TvEpisode>, Box<dyn std::error::Error + Send + Sync>> {
    let details = get_tv_details(tv_id).await?;
    let seasons = details.seasons.unwrap_or_default();

    let mut all_episodes: Vec<TvEpisode> = Vec::new();
    for summary in seasons {
        match get_tv_season(tv_id, summary.season_number).await {
            Ok(season) => all_episodes.extend(season.episodes),
            Err(e) => {
                eprintln!(
                    "tmdb: failed to fetch season {} of tv {}: {}",
                    summary.season_number, tv_id, e
                );
            }
        }
    }

    all_episodes.sort_by(|a, b| {
        a.season_number
            .cmp(&b.season_number)
            .then(a.episode_number.cmp(&b.episode_number))
    });

    Ok(all_episodes)
}

/// Cast and crew for a TV show (cached, 24h).
pub async fn get_tv_credits(
    id: i64,
) -> Result<TvCredits, Box<dyn std::error::Error + Send + Sync>> {
    let cache = get_cache();
    if let Some(cached) = cache.tv_credits_cache.get(&id).await {
        return Ok(cached);
    }

    let client = create_http_client();
    let response = client
        .get(format!("{}/tv/{}/credits", API_BASE, id))
        .bearer_auth(READ_ACCESS_TOKEN)
        .query(&[("language", PRIMARY_LANGUAGE)])
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("TMDB API error: {}", response.status()).into());
    }

    let credits: TvCredits = response.json().await?;
    cache.tv_credits_cache.insert(id, credits.clone()).await;
    Ok(credits)
}

/// Generic external-id lookup, e.g. `find_by_external_id("tvdb_id", "12345")`.
/// Not cached — used during one-shot migration so caching adds no value.
pub async fn find_by_external_id(
    source: &str,
    external_id: &str,
) -> Result<ExternalIdFindResponse, Box<dyn std::error::Error + Send + Sync>> {
    let client = create_http_client();
    let response = client
        .get(format!("{}/find/{}", API_BASE, external_id))
        .query(&[("external_source", source)])
        .bearer_auth(READ_ACCESS_TOKEN)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("TMDB API error: {}", response.status()).into());
    }

    let found: ExternalIdFindResponse = response.json().await?;
    Ok(found)
}

/// Convenience helper: look up the TMDB TV id for a known TVDB id. Returns
/// `Ok(None)` when TMDB has no mapping (the most common quarantine case).
pub async fn find_tv_by_tvdb_id(
    tvdb_id: i64,
) -> Result<Option<i64>, Box<dyn std::error::Error + Send + Sync>> {
    let result = find_by_external_id("tvdb_id", &tvdb_id.to_string()).await?;
    Ok(result.tv_results.into_iter().next().map(|r| r.id))
}

/// Invalidate all cache entries tied to a specific TV show id.
pub async fn invalidate_tv_show_cache(id: i64) {
    let cache = get_cache();
    cache.tv_details_cache.invalidate(&id).await;
    cache.tv_credits_cache.invalidate(&id).await;
    // Best-effort season invalidation: drop the entire season cache. Seasons
    // are small enough that a full clear is cheaper than tracking which season
    // numbers a given show has cached.
    cache.tv_season_cache.invalidate_all();
}

/// Clear every TV-related cache. Used before bulk resync operations.
pub async fn clear_all_tv_caches() {
    let cache = get_cache();
    cache.tv_search_cache.invalidate_all();
    cache.tv_details_cache.invalidate_all();
    cache.tv_season_cache.invalidate_all();
    cache.tv_credits_cache.invalidate_all();
}
