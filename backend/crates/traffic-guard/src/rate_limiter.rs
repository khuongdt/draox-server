use crate::verdict::GuardVerdict;
use dashmap::DashMap;
use governor::clock::DefaultClock;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter as GovernorRateLimiter};
use server_config::model::RateLimitingConfig;
use std::net::IpAddr;
use std::num::NonZeroU32;
use std::sync::Arc;
use tracing::debug;

/// Type alias for the governor rate limiter used per IP.
type IpRateLimiter = GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>;

/// Per-IP rate limiter using the governor token bucket algorithm.
///
/// Each IP address gets its own rate limiter with the configured
/// requests-per-second quota and burst size.
pub struct RateLimiter {
    limiters: DashMap<IpAddr, Arc<IpRateLimiter>>,
    requests_per_sec: NonZeroU32,
    burst_size: NonZeroU32,
}

impl RateLimiter {
    /// Create a new RateLimiter from configuration.
    pub fn new(config: &RateLimitingConfig) -> Self {
        let requests_per_sec =
            NonZeroU32::new(config.default_requests_per_sec).unwrap_or(NonZeroU32::new(100).unwrap());
        let burst_size =
            NonZeroU32::new(config.burst_size).unwrap_or(NonZeroU32::new(50).unwrap());

        Self {
            limiters: DashMap::new(),
            requests_per_sec,
            burst_size,
        }
    }

    /// Check if a request from the given IP is allowed under the rate limit.
    pub fn check(&self, ip: IpAddr) -> GuardVerdict {
        let limiter = self
            .limiters
            .entry(ip)
            .or_insert_with(|| {
                let quota = Quota::per_second(self.requests_per_sec)
                    .allow_burst(self.burst_size);
                Arc::new(GovernorRateLimiter::direct(quota))
            })
            .clone();

        match limiter.check() {
            Ok(()) => GuardVerdict::Allow,
            Err(_not_until) => {
                debug!("Rate limit exceeded for IP {}", ip);
                GuardVerdict::Block(format!("rate limit exceeded for {ip}"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> RateLimitingConfig {
        RateLimitingConfig {
            algorithm: "token_bucket".to_string(),
            default_requests_per_sec: 10,
            burst_size: 5,
            http_rate_per_sec: 200,
            ws_messages_per_sec: 60,
            udp_packets_per_sec: 500,
        }
    }

    #[test]
    fn test_rate_allow() {
        let limiter = RateLimiter::new(&test_config());
        let ip: IpAddr = "192.168.1.1".parse().unwrap();

        // First request should be allowed (within burst)
        let verdict = limiter.check(ip);
        assert_eq!(verdict, GuardVerdict::Allow);
    }

    #[test]
    fn test_rate_exceeded() {
        let config = RateLimitingConfig {
            algorithm: "token_bucket".to_string(),
            default_requests_per_sec: 1,
            burst_size: 1,
            http_rate_per_sec: 200,
            ws_messages_per_sec: 60,
            udp_packets_per_sec: 500,
        };
        let limiter = RateLimiter::new(&config);
        let ip: IpAddr = "10.0.0.1".parse().unwrap();

        // First request should be allowed (uses the burst)
        let verdict = limiter.check(ip);
        assert_eq!(verdict, GuardVerdict::Allow);

        // Second request immediately should be blocked (burst exhausted)
        let verdict = limiter.check(ip);
        assert!(matches!(verdict, GuardVerdict::Block(_)));
    }
}
