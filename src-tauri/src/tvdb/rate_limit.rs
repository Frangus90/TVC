use std::time::{Duration, Instant};
use tokio::sync::Mutex;

pub struct RateLimiter {
    requests: Mutex<Vec<Instant>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            requests: Mutex::new(Vec::new()),
            max_requests,
            window,
        }
    }

    pub async fn wait_if_needed(&self) {
        let mut requests = self.requests.lock().await;
        let now = Instant::now();

        // Remove requests outside the window
        requests.retain(|&time| now.duration_since(time) < self.window);

        // If we're at the limit, wait until the oldest request expires
        if requests.len() >= self.max_requests {
            if let Some(oldest) = requests.first() {
                let wait_time = self.window - now.duration_since(*oldest);
                if wait_time > Duration::ZERO {
                    tokio::time::sleep(wait_time).await;
                    // Clean up again after waiting
                    let now = Instant::now();
                    requests.retain(|&time| now.duration_since(time) < self.window);
                }
            }
        }

        // Record this request
        requests.push(Instant::now());
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        // TVDB API typically allows 120 requests per minute
        Self::new(120, Duration::from_secs(60))
    }
}




