use moka::future::Cache;
use std::time::Duration;

pub struct ApiCache {
    search_cache: Cache<String, Vec<crate::tvdb::SearchResult>>,
    show_cache: Cache<i64, crate::tvdb::SeriesExtended>,
    episodes_cache: Cache<i64, Vec<crate::tvdb::EpisodeBase>>,
}

impl ApiCache {
    pub fn new() -> Self {
        Self {
            // Search results: 1 hour TTL
            search_cache: Cache::builder()
                .time_to_live(Duration::from_secs(3600))
                .build(),
            // Show details: 24 hours TTL
            show_cache: Cache::builder()
                .time_to_live(Duration::from_secs(86400))
                .build(),
            // Episode lists: 6 hours TTL
            episodes_cache: Cache::builder()
                .time_to_live(Duration::from_secs(21600))
                .build(),
        }
    }

    pub async fn get_search(&self, query: &str) -> Option<Vec<crate::tvdb::SearchResult>> {
        self.search_cache.get(query).await
    }

    pub async fn set_search(&self, query: String, results: Vec<crate::tvdb::SearchResult>) {
        self.search_cache.insert(query, results).await;
    }

    pub async fn get_show(&self, id: i64) -> Option<crate::tvdb::SeriesExtended> {
        self.show_cache.get(&id).await
    }

    pub async fn set_show(&self, id: i64, show: crate::tvdb::SeriesExtended) {
        self.show_cache.insert(id, show).await;
    }

    pub async fn get_episodes(&self, id: i64) -> Option<Vec<crate::tvdb::EpisodeBase>> {
        self.episodes_cache.get(&id).await
    }

    pub async fn set_episodes(&self, id: i64, episodes: Vec<crate::tvdb::EpisodeBase>) {
        self.episodes_cache.insert(id, episodes).await;
    }

    /// Clear cache for a specific show (used when syncing to force fresh data)
    pub async fn invalidate_show(&self, id: i64) {
        self.show_cache.invalidate(&id).await;
        self.episodes_cache.invalidate(&id).await;
    }

    /// Clear all caches (used for full resync)
    pub async fn clear_all(&self) {
        self.search_cache.invalidate_all();
        self.show_cache.invalidate_all();
        self.episodes_cache.invalidate_all();
    }
}

impl Default for ApiCache {
    fn default() -> Self {
        Self::new()
    }
}







