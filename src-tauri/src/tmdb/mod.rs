use moka::future::Cache;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use std::time::Duration;

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

    let client = Client::new();
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

    let client = Client::new();
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
    let client = Client::new();
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
    let client = Client::new();
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
    let client = Client::new();
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

// TV Show search result from TMDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvSearchResult {
    pub id: i64,
    pub name: String,
    pub first_air_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TvSearchResponse {
    results: Vec<TvSearchResult>,
}

/// Search for a TV show on TMDB by name
pub async fn search_tv_show(
    query: &str,
) -> Result<Vec<TvSearchResult>, Box<dyn std::error::Error + Send + Sync>> {
    let client = Client::new();
    let response = client
        .get(format!("{}/search/tv", API_BASE))
        .query(&[("query", query)])
        .bearer_auth(READ_ACCESS_TOKEN)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("TMDB API error: {}", response.status()).into());
    }

    let search: TvSearchResponse = response.json().await?;
    Ok(search.results)
}

/// Get videos/trailers for a TV show
pub async fn get_tv_videos(
    id: i64,
) -> Result<Vec<Video>, Box<dyn std::error::Error + Send + Sync>> {
    let client = Client::new();
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
