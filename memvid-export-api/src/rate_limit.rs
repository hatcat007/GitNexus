use std::{collections::HashMap, time::Instant};

use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct RateLimitHeaders {
    pub limit: u32,
    pub remaining: u32,
    pub reset_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct RateLimitDecision {
    pub allowed: bool,
    pub headers: RateLimitHeaders,
}

#[derive(Debug)]
struct BucketState {
    tokens: f64,
    last_refill: Instant,
}

#[derive(Debug)]
pub struct RateLimiter {
    per_minute: u32,
    burst: u32,
    buckets: Mutex<HashMap<String, BucketState>>,
}

impl RateLimiter {
    pub fn new(per_minute: u32, burst: u32) -> Self {
        Self {
            per_minute: per_minute.max(1),
            burst: burst.max(1),
            buckets: Mutex::new(HashMap::new()),
        }
    }

    pub async fn check(&self, key: &str) -> RateLimitDecision {
        let now = Instant::now();
        let refill_per_sec = f64::from(self.per_minute) / 60.0;
        let capacity = f64::from(self.burst.max(self.per_minute));

        let mut guard = self.buckets.lock().await;
        let bucket = guard.entry(key.to_string()).or_insert(BucketState {
            tokens: capacity,
            last_refill: now,
        });

        let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
        if elapsed > 0.0 {
            let restored = elapsed * refill_per_sec;
            bucket.tokens = (bucket.tokens + restored).min(capacity);
            bucket.last_refill = now;
        }

        let allowed = bucket.tokens >= 1.0;
        if allowed {
            bucket.tokens -= 1.0;
        }

        let remaining = bucket.tokens.floor().max(0.0) as u32;
        let deficit = (1.0 - bucket.tokens).max(0.0);
        let reset_seconds = if deficit <= 0.0 {
            0
        } else {
            (deficit / refill_per_sec).ceil() as u64
        };

        RateLimitDecision {
            allowed,
            headers: RateLimitHeaders {
                limit: self.per_minute,
                remaining,
                reset_seconds,
            },
        }
    }
}
