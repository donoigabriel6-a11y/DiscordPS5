//! Rate Limiting for Discord REST API

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use parking_lot::RwLock;
use tokio::sync::Semaphore;
use tokio::time::sleep;
use tracing::{debug, warn};

use crate::{DiscordError, Result as RestResult};

/// Rate limit bucket for a specific endpoint
#[derive(Debug, Clone)]
pub struct RateLimitBucket {
    pub limit: u32,
    pub remaining: u32,
    pub reset_after: f64,
    pub reset_at: Instant,
    pub semaphore: Arc<Semaphore>,
}

impl RateLimitBucket {
    pub fn new(limit: u32) -> Self {
        Self {
            limit,
            remaining: limit,
            reset_after: 0.0,
            reset_at: Instant::now(),
            semaphore: Arc::new(Semaphore::new(limit as usize)),
        }
    }

    pub fn update(&mut self, limit: u32, remaining: u32, reset_after: f64) {
        self.limit = limit;
        self.remaining = remaining;
        self.reset_after = reset_after;
        self.reset_at = Instant::now() + Duration::from_secs_f64(reset_after);

        // Adjust semaphore permits
        let current = self.semaphore.available_permits();
        if current != remaining as usize {
            // Can't easily adjust semaphore, but we track remaining
        }
    }

    pub async fn acquire(&self) -> RestResult<()> {
        if self.remaining == 0 {
            let wait_time = self.reset_at.saturating_duration_since(Instant::now());
            if wait_time > Duration::ZERO {
                debug!("Rate limited, waiting {:?}", wait_time);
                sleep(wait_time).await;
            }
        }

        let _permit = self.semaphore.acquire().await
            .map_err(|_| DiscordError::RateLimited { retry_after: 1 })?;

        Ok(())
    }
}

/// Global rate limiter
#[derive(Debug)]
pub struct RateLimiter {
    buckets: RwLock<HashMap<String, RateLimitBucket>>,
    global_limit: RwLock<Option<f64>>,
    global_reset_at: RwLock<Option<Instant>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            buckets: RwLock::new(HashMap::new()),
            global_limit: RwLock::new(None),
            global_reset_at: RwLock::new(None),
        }
    }

    pub async fn acquire(&self, bucket_key: &str) -> RestResult<()> {
        // Check global rate limit first
        if let Some(reset_at) = *self.global_reset_at.read() {
            if Instant::now() < reset_at {
                let wait = reset_at.saturating_duration_since(Instant::now());
                warn!("Global rate limit, waiting {:?}", wait);
                sleep(wait).await;
            } else {
                *self.global_limit.write() = None;
                *self.global_reset_at.write() = None;
            }
        }

        // Get or create bucket
        let bucket = {
            let mut buckets = self.buckets.write();
            buckets.entry(bucket_key.to_string())
                .or_insert_with(|| RateLimitBucket::new(1))
                .clone()
        };

        bucket.acquire().await
    }

    pub fn update_bucket(&self, bucket_key: &str, limit: u32, remaining: u32, reset_after: f64) {
        let mut buckets = self.buckets.write();
        let bucket = buckets.entry(bucket_key.to_string())
            .or_insert_with(|| RateLimitBucket::new(limit));
        bucket.update(limit, remaining, reset_after);
    }

    pub fn set_global_limit(&self, retry_after: f64) {
        *self.global_limit.write() = Some(retry_after);
        *self.global_reset_at.write() = Some(Instant::now() + Duration::from_secs_f64(retry_after));
        warn!("Global rate limit set: {:.2}s", retry_after);
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}