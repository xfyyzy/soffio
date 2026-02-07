use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

const CLEANUP_INTERVAL_CALLS: u64 = 256;

#[derive(Debug, Clone)]
struct BucketState {
    tokens: f64,
    last_refill: Instant,
    last_seen: Instant,
}

#[derive(Debug, Clone)]
pub struct ApiRateLimiter {
    window: Duration,
    max_requests: u32,
    refill_per_sec: f64,
    buckets: Arc<DashMap<String, BucketState>>,
    cleanup_tick: Arc<AtomicU64>,
    cleanup_interval_calls: u64,
}

impl ApiRateLimiter {
    pub fn new(window: Duration, max_requests: u32) -> Self {
        let window_secs = window.as_secs_f64().max(0.001);
        Self {
            window,
            max_requests,
            refill_per_sec: max_requests as f64 / window_secs,
            buckets: Arc::new(DashMap::new()),
            cleanup_tick: Arc::new(AtomicU64::new(0)),
            cleanup_interval_calls: CLEANUP_INTERVAL_CALLS,
        }
    }

    pub fn allow(&self, key: &str, route: &str) -> (bool, u32) {
        if self.max_requests == 0 {
            return (false, 0);
        }

        let bucket_key = format!("{key}:{route}");
        let now = Instant::now();
        self.maybe_cleanup(now);

        let mut entry = self
            .buckets
            .entry(bucket_key)
            .or_insert_with(|| BucketState::new(self.max_requests, now));

        let elapsed_secs = now.duration_since(entry.last_refill).as_secs_f64();
        if elapsed_secs > 0.0 {
            let refill = elapsed_secs * self.refill_per_sec;
            entry.tokens = (entry.tokens + refill).min(self.max_requests as f64);
            entry.last_refill = now;
        }
        entry.last_seen = now;

        if entry.tokens < 1.0 {
            return (false, 0);
        }

        entry.tokens -= 1.0;
        (true, entry.tokens.floor() as u32)
    }

    pub fn retry_after_secs(&self) -> u64 {
        self.window.as_secs().max(1)
    }

    pub fn limit(&self) -> u32 {
        self.max_requests
    }

    fn maybe_cleanup(&self, now: Instant) {
        let tick = self.cleanup_tick.fetch_add(1, Ordering::Relaxed) + 1;
        if !tick.is_multiple_of(self.cleanup_interval_calls) {
            return;
        }

        let stale_after = self.stale_after();
        self.buckets
            .retain(|_, state| now.duration_since(state.last_seen) < stale_after);
    }

    fn stale_after(&self) -> Duration {
        match self.window.checked_mul(4) {
            Some(value) => value,
            None => self.window,
        }
    }
}

impl BucketState {
    fn new(max_requests: u32, now: Instant) -> Self {
        Self {
            tokens: max_requests as f64,
            last_refill: now,
            last_seen: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allow_up_to_limit_then_deny() {
        let limiter = ApiRateLimiter::new(Duration::from_secs(60), 2);

        let (allowed, remaining) = limiter.allow("key", "route");
        assert!(allowed);
        assert_eq!(remaining, 1);

        let (allowed, remaining) = limiter.allow("key", "route");
        assert!(allowed);
        assert_eq!(remaining, 0);

        let (allowed, remaining) = limiter.allow("key", "route");
        assert!(!allowed);
        assert_eq!(remaining, 0);
    }

    #[test]
    fn limiter_memory_is_bounded_per_bucket() {
        let limiter = ApiRateLimiter::new(Duration::from_secs(60), 3);
        for _ in 0..10_000 {
            let _ = limiter.allow("key", "route");
        }

        assert_eq!(limiter.buckets.len(), 1);
    }

    #[test]
    fn limiter_tracks_buckets_by_key_and_route() {
        let limiter = ApiRateLimiter::new(Duration::from_secs(60), 3);
        let _ = limiter.allow("key", "route-a");
        let _ = limiter.allow("key", "route-b");
        let _ = limiter.allow("other", "route-a");

        assert_eq!(limiter.buckets.len(), 3);
    }
}
