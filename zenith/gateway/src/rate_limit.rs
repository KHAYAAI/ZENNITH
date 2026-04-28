use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::warn;
use axum::http::StatusCode;

pub struct SimpleRateLimiter {
    max_requests: u32,
    window_duration: Duration,
    request_count: Arc<AtomicU32>,
    window_start: Arc<RwLock<Instant>>,
}

impl SimpleRateLimiter {
    pub fn new(max_requests_per_second: u32) -> Self {
        SimpleRateLimiter {
            max_requests: max_requests_per_second,
            window_duration: Duration::from_secs(1),
            request_count: Arc::new(AtomicU32::new(0)),
            window_start: Arc::new(RwLock::new(Instant::now())),
        }
    }

    pub async fn check(&self) -> Result<(), StatusCode> {
        let mut window_start = self.window_start.write().await;
        let now = Instant::now();

        // Reset counter if window has expired
        if now.duration_since(*window_start) > self.window_duration {
            *window_start = now;
            self.request_count.store(0, Ordering::Relaxed);
        }

        // Increment and check
        let count = self.request_count.fetch_add(1, Ordering::Relaxed);
        if count >= self.max_requests {
            warn!("Rate limit exceeded: {}/{} requests", count, self.max_requests);
            Err(StatusCode::TOO_MANY_REQUESTS)
        } else {
            Ok(())
        }
    }
}

pub struct RateLimitMiddleware {
    global_limiter: Arc<SimpleRateLimiter>,
}

impl RateLimitMiddleware {
    pub fn new() -> Self {
        RateLimitMiddleware {
            global_limiter: Arc::new(SimpleRateLimiter::new(1000)), // 1000 req/s global
        }
    }

    pub async fn check_rate_limit(&self, _ip: String) -> Result<(), StatusCode> {
        self.global_limiter.check().await
    }
}
