use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct ApiRateLimiter {
    window: Duration,
    max_requests: u32,
    buckets: Arc<DashMap<String, Vec<Instant>>>,
}

impl ApiRateLimiter {
    pub fn new(window: Duration, max_requests: u32) -> Self {
        Self {
            window,
            max_requests,
            buckets: Arc::new(DashMap::new()),
        }
    }

    pub fn allow(&self, key: &str, route: &str) -> (bool, u32) {
        let bucket_key = format!("{key}:{route}");
        let now = Instant::now();
        let window = self.window;

        let mut entry = self.buckets.entry(bucket_key).or_default();
        entry.retain(|instant| now.duration_since(*instant) < window);

        let remaining = self.max_requests.saturating_sub(entry.len() as u32);
        if remaining == 0 {
            return (false, 0);
        }

        entry.push(now);
        // after push, one fewer slot remains
        (true, remaining.saturating_sub(1))
    }

    pub fn retry_after_secs(&self) -> u64 {
        self.window.as_secs().max(1)
    }

    pub fn limit(&self) -> u32 {
        self.max_requests
    }
}
