//! Rate limiting for API requests

use governor::{DefaultDirectRateLimiter, Quota, RateLimiter as Governor};
use protocol_v2::VenueId;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// Rate limiter for venue API requests
#[derive(Clone)]
pub struct RateLimiter {
    limiters: HashMap<VenueId, Arc<DefaultDirectRateLimiter>>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new() -> Self {
        Self {
            limiters: HashMap::new(),
        }
    }

    /// Configure rate limit for a venue
    pub fn configure_venue(&mut self, venue: VenueId, requests_per_minute: u32) {
        use std::num::NonZeroU32;

        if let Ok(rate) = NonZeroU32::try_from(requests_per_minute) {
            let quota = Quota::per_minute(rate);
            let limiter = Arc::new(DefaultDirectRateLimiter::direct(quota));
            self.limiters.insert(venue, limiter);
        } else {
            tracing::warn!(
                "Invalid rate limit for venue {:?}: {}",
                venue,
                requests_per_minute
            );
        }
    }

    /// Check if request is allowed (non-blocking)
    pub fn check(&self, venue: VenueId) -> bool {
        self.limiters
            .get(&venue)
            .map(|limiter| limiter.check().is_ok())
            .unwrap_or(true) // Allow if no limiter configured
    }

    /// Wait until request is allowed (blocking)
    pub async fn wait(&self, venue: VenueId) -> Result<(), crate::AdapterError> {
        if let Some(limiter) = self.limiters.get(&venue) {
            limiter.until_ready().await;
            Ok(())
        } else {
            Ok(()) // No limit configured
        }
    }

    /// Try to acquire permission for multiple requests
    pub fn check_n(&self, venue: VenueId, n: u32) -> bool {
        use std::num::NonZeroU32;

        if let Ok(nonzero_n) = NonZeroU32::try_from(n) {
            self.limiters
                .get(&venue)
                .map(|limiter| limiter.check_n(nonzero_n).is_ok())
                .unwrap_or(true)
        } else {
            false // 0 requests is not allowed
        }
    }

    /// Get remaining capacity for a venue (approximation)
    pub fn remaining_capacity(&self, venue: VenueId) -> Option<u32> {
        // With direct rate limiter, capacity info isn't easily accessible
        // Return None to indicate this information isn't available
        self.limiters.get(&venue).map(|_| 0) // Placeholder
    }

    /// Get rate limit configuration for monitoring
    pub fn get_limits(&self) -> HashMap<VenueId, RateLimitInfo> {
        self.limiters
            .iter()
            .map(|(venue, _limiter)| {
                (
                    *venue,
                    RateLimitInfo {
                        requests_per_minute: 0, // Placeholder - not easily accessible
                        remaining_capacity: 0,  // Placeholder
                    },
                )
            })
            .collect()
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        let mut limiter = Self::new();

        // Default rate limits per venue
        limiter.configure_venue(VenueId::Binance, 1200);
        limiter.configure_venue(VenueId::Coinbase, 1000);
        limiter.configure_venue(VenueId::Polygon, 1000);

        limiter
    }
}

/// Configuration for rate limiting
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Requests per minute for each venue
    pub venue_limits: HashMap<VenueId, u32>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        let mut venue_limits = HashMap::new();
        venue_limits.insert(VenueId::Binance, 1200);
        venue_limits.insert(VenueId::Coinbase, 1000);
        venue_limits.insert(VenueId::Polygon, 1000);

        Self { venue_limits }
    }
}

/// Information about rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    /// Configured requests per minute
    pub requests_per_minute: u32,
    /// Current remaining capacity
    pub remaining_capacity: u32,
}

/// Rate limit tracker for monitoring
pub struct RateLimitTracker {
    requests: Arc<dashmap::DashMap<VenueId, RequestStats>>,
}

impl RateLimitTracker {
    /// Create a new tracker
    pub fn new() -> Self {
        Self {
            requests: Arc::new(dashmap::DashMap::new()),
        }
    }

    /// Record a request
    pub fn record_request(&self, venue: VenueId, success: bool) {
        self.requests
            .entry(venue)
            .and_modify(|stats| {
                stats.total += 1;
                if success {
                    stats.successful += 1;
                } else {
                    stats.rate_limited += 1;
                }
                stats.last_request = std::time::Instant::now();
            })
            .or_insert(RequestStats {
                total: 1,
                successful: if success { 1 } else { 0 },
                rate_limited: if !success { 1 } else { 0 },
                last_request: std::time::Instant::now(),
            });
    }

    /// Get statistics for all venues
    pub fn get_stats(&self) -> HashMap<VenueId, RequestStats> {
        self.requests
            .iter()
            .map(|entry| (*entry.key(), entry.value().clone()))
            .collect()
    }

    /// Reset statistics
    pub fn reset(&self) {
        self.requests.clear();
    }
}

impl Default for RateLimitTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Request statistics for monitoring
#[derive(Debug, Clone)]
pub struct RequestStats {
    /// Total requests attempted
    pub total: u64,
    /// Successful requests
    pub successful: u64,
    /// Rate-limited requests
    pub rate_limited: u64,
    /// Time of last request
    pub last_request: std::time::Instant,
}

impl RequestStats {
    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.successful as f64 / self.total as f64
        }
    }

    /// Check if we're being rate limited heavily
    pub fn is_heavily_limited(&self) -> bool {
        self.rate_limited > self.successful
    }
}
