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

    pub fn allow(&self, key: &str, route: &str) -> bool {
        let bucket_key = format!("{key}:{route}");
        let now = Instant::now();
        let window = self.window;

        let mut entry = self.buckets.entry(bucket_key).or_default();
        entry.retain(|instant| now.duration_since(*instant) < window);

        if entry.len() as u32 >= self.max_requests {
            return false;
        }

        entry.push(now);
        true
    }

    pub fn retry_after_secs(&self) -> u64 {
        self.window.as_secs().max(1)
    }
}
