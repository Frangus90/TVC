use reqwest::Client;
use std::time::Duration;
use crate::error::AppError;
use super::models::{RadarrMovie, ArrSystemStatus};

/// Create an HTTP client with proper timeout configuration
fn create_http_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| Client::new()) // Fallback to default if builder fails
}

pub struct RadarrClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl RadarrClient {
    pub fn new(base_url: &str, api_key: &str) -> Self {
        // Normalize base URL (remove trailing slash)
        let base_url = base_url.trim_end_matches('/').to_string();

        Self {
            client: create_http_client(),
            base_url,
            api_key: api_key.to_string(),
        }
    }

    /// Test connection to Radarr server
    pub async fn test_connection(&self) -> Result<ArrSystemStatus, AppError> {
        let url = format!("{}/api/v3/system/status", self.base_url);

        let response = self
            .client
            .get(&url)
            .header("X-Api-Key", &self.api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Api(format!(
                "Radarr API error ({}): {}",
                status, body
            )));
        }

        let status: ArrSystemStatus = response.json().await?;
        Ok(status)
    }

    /// Get all movies from Radarr library
    pub async fn get_movies(&self) -> Result<Vec<RadarrMovie>, AppError> {
        let url = format!("{}/api/v3/movie", self.base_url);

        let response = self
            .client
            .get(&url)
            .header("X-Api-Key", &self.api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Api(format!(
                "Failed to get Radarr movies ({}): {}",
                status, body
            )));
        }

        let movies: Vec<RadarrMovie> = response.json().await?;
        Ok(movies)
    }

    /// Get a specific movie by ID
    #[allow(dead_code)]
    pub async fn get_movie_by_id(&self, id: i64) -> Result<RadarrMovie, AppError> {
        let url = format!("{}/api/v3/movie/{}", self.base_url, id);

        let response = self
            .client
            .get(&url)
            .header("X-Api-Key", &self.api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Api(format!(
                "Failed to get Radarr movie {} ({}): {}",
                id, status, body
            )));
        }

        let movie: RadarrMovie = response.json().await?;
        Ok(movie)
    }
}
