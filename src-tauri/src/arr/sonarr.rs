use reqwest::Client;
use crate::error::AppError;
use super::models::{SonarrSeries, ArrSystemStatus};

pub struct SonarrClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl SonarrClient {
    pub fn new(base_url: &str, api_key: &str) -> Self {
        // Normalize base URL (remove trailing slash)
        let base_url = base_url.trim_end_matches('/').to_string();

        Self {
            client: Client::new(),
            base_url,
            api_key: api_key.to_string(),
        }
    }

    /// Test connection to Sonarr server
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
                "Sonarr API error ({}): {}",
                status, body
            )));
        }

        let status: ArrSystemStatus = response.json().await?;
        Ok(status)
    }

    /// Get all series from Sonarr library
    pub async fn get_series(&self) -> Result<Vec<SonarrSeries>, AppError> {
        let url = format!("{}/api/v3/series", self.base_url);

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
                "Failed to get Sonarr series ({}): {}",
                status, body
            )));
        }

        let series: Vec<SonarrSeries> = response.json().await?;
        Ok(series)
    }

    /// Get a specific series by ID
    #[allow(dead_code)]
    pub async fn get_series_by_id(&self, id: i64) -> Result<SonarrSeries, AppError> {
        let url = format!("{}/api/v3/series/{}", self.base_url, id);

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
                "Failed to get Sonarr series {} ({}): {}",
                id, status, body
            )));
        }

        let series: SonarrSeries = response.json().await?;
        Ok(series)
    }
}
