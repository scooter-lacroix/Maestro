//! Rate limiting with sliding window algorithm
//!
//! Based on ZeroClaw rate limiting pattern.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

/// Configuration for rate limiting
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per window
    pub limit: u32,
    /// Window duration
    pub window: Duration,
    /// Whether to include retry-after header
    pub include_retry_after: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            limit: 100,
            window: Duration::from_secs(60),
            include_retry_after: true,
        }
    }
}

impl RateLimitConfig {
    /// Create a strict rate limit (30 req/min)
    pub fn strict() -> Self {
        Self {
            limit: 30,
            window: Duration::from_secs(60),
            include_retry_after: true,
        }
    }

    /// Create a lenient rate limit (300 req/min)
    pub fn lenient() -> Self {
        Self {
            limit: 300,
            window: Duration::from_secs(60),
            include_retry_after: true,
        }
    }

    /// Create a burst rate limit (1000 req/min)
    pub fn burst() -> Self {
        Self {
            limit: 1000,
            window: Duration::from_secs(60),
            include_retry_after: true,
        }
    }
}

/// Sliding window rate limiter
#[derive(Debug)]
pub struct SlidingWindowRateLimiter {
    config: RateLimitConfig,
    requests: Mutex<HashMap<String, Vec<Instant>>>,
}

impl SlidingWindowRateLimiter {
    /// Create a new rate limiter with the given config
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            requests: Mutex::new(HashMap::new()),
        }
    }

    /// Check if a request from the given key is allowed
    ///
    /// Returns (allowed, remaining, retry_after_ms)
    pub fn check(&self, key: &str) -> (bool, u32, Option<u64>) {
        let now = Instant::now();
        let window_start = now - self.config.window;

        let mut requests = self.requests.lock();

        // Get or create entry for this key
        let entry = requests.entry(key.to_string()).or_insert_with(Vec::new);

        // Remove expired timestamps
        entry.retain(|&ts| ts > window_start);

        // Check if under limit
        if entry.len() < self.config.limit as usize {
            entry.push(now);
            let remaining = self.config.limit - entry.len() as u32;
            (true, remaining, None)
        } else {
            // Calculate retry-after
            let oldest = entry.first().copied();
            let retry_after = oldest.map(|ts| {
                let elapsed = now.duration_since(ts);
                let remaining = self.config.window.saturating_sub(elapsed);
                remaining.as_millis() as u64
            });
            (false, 0, retry_after)
        }
    }

    /// Get current usage for a key
    pub fn usage(&self, key: &str) -> (u32, u32) {
        let now = Instant::now();
        let window_start = now - self.config.window;

        let requests = self.requests.lock();
        let entry = requests.get(key);

        let count = entry
            .map(|v| v.iter().filter(|&&ts| ts > window_start).count() as u32)
            .unwrap_or(0);

        (count, self.config.limit)
    }

    /// Clear rate limit state for a key
    pub fn reset(&self, key: &str) {
        let mut requests = self.requests.lock();
        requests.remove(key);
    }

    /// Clear all rate limit state
    pub fn reset_all(&self) {
        let mut requests = self.requests.lock();
        requests.clear();
    }

    /// Cleanup expired entries to prevent memory growth
    pub fn cleanup(&self) {
        let now = Instant::now();
        let window_start = now - self.config.window;

        let mut requests = self.requests.lock();
        requests.retain(|_, timestamps| {
            timestamps.retain(|&ts| ts > window_start);
            !timestamps.is_empty()
        });
    }
}

/// Rate limit key extractor trait
pub trait RateLimitKeyExtractor: Send + Sync {
    fn extract(&self, req: &axum::http::Request<axum::body::Body>) -> String;
}

/// Extract key from IP address
#[derive(Clone)]
pub struct IpKeyExtractor;

impl RateLimitKeyExtractor for IpKeyExtractor {
    fn extract(&self, req: &axum::http::Request<axum::body::Body>) -> String {
        // Try X-Forwarded-For first
        if let Some(forwarded) = req.headers().get("x-forwarded-for") {
            if let Ok(s) = forwarded.to_str() {
                if let Some(ip) = s.split(',').next() {
                    return format!("ip:{}", ip.trim());
                }
            }
        }

        // Try X-Real-IP
        if let Some(real_ip) = req.headers().get("x-real-ip") {
            if let Ok(s) = real_ip.to_str() {
                return format!("ip:{}", s);
            }
        }

        // Fallback to unknown
        "ip:unknown".to_string()
    }
}

/// Rate limit key from authenticated user
#[derive(Clone)]
pub struct UserKeyExtractor;

impl RateLimitKeyExtractor for UserKeyExtractor {
    fn extract(&self, req: &axum::http::Request<axum::body::Body>) -> String {
        // Try to get user from authorization header or extension
        if let Some(user_id) = req.extensions().get::<UserId>() {
            return format!("user:{}", user_id.0);
        }

        // Fallback to IP-based
        IpKeyExtractor.extract(req)
    }
}

/// User ID wrapper for extensions
#[derive(Clone, Debug)]
pub struct UserId(pub String);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_under_limit() {
        let limiter = SlidingWindowRateLimiter::new(RateLimitConfig {
            limit: 5,
            window: Duration::from_secs(60),
            ..Default::default()
        });

        for i in 0..5 {
            let (allowed, remaining, _) = limiter.check("test-key");
            assert!(allowed, "Request {} should be allowed", i);
            assert_eq!(remaining, 4 - i as u32);
        }
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let limiter = SlidingWindowRateLimiter::new(RateLimitConfig {
            limit: 2,
            window: Duration::from_secs(60),
            ..Default::default()
        });

        limiter.check("test-key"); // 1
        limiter.check("test-key"); // 2

        let (allowed, _, retry_after) = limiter.check("test-key"); // 3 - should be blocked
        assert!(!allowed);
        assert!(retry_after.is_some());
    }

    #[test]
    fn test_rate_limiter_different_keys() {
        let limiter = SlidingWindowRateLimiter::new(RateLimitConfig {
            limit: 1,
            window: Duration::from_secs(60),
            ..Default::default()
        });

        let (allowed1, _, _) = limiter.check("key1");
        let (allowed2, _, _) = limiter.check("key2");

        assert!(allowed1);
        assert!(allowed2);

        // Both should now be blocked
        let (allowed1_again, _, _) = limiter.check("key1");
        let (allowed2_again, _, _) = limiter.check("key2");

        assert!(!allowed1_again);
        assert!(!allowed2_again);
    }

    #[test]
    fn test_rate_limiter_reset() {
        let limiter = SlidingWindowRateLimiter::new(RateLimitConfig {
            limit: 1,
            window: Duration::from_secs(60),
            ..Default::default()
        });

        limiter.check("test-key");
        let (allowed, _, _) = limiter.check("test-key");
        assert!(!allowed);

        limiter.reset("test-key");
        let (allowed, _, _) = limiter.check("test-key");
        assert!(allowed);
    }
}
