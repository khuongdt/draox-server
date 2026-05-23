//! Per-protocol rate limiting and slowloris detection.
//!
//! Different protocols have different traffic characteristics. HTTP endpoints
//! need stricter rate limits than raw TCP, WebSocket messages have their own
//! frequency, and UDP packets are often high-throughput. This module provides
//! [`ProtocolGuard`] which applies the correct rate limit based on the
//! connection's protocol.
//!
//! Additionally, [`SlowlorisDetector`] watches for connections that send data
//! at abnormally low rates — a classic sign of a slowloris denial-of-service
//! attack that tries to hold connections open indefinitely.

use crate::verdict::GuardVerdict;
use dashmap::DashMap;
use governor::clock::DefaultClock;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter as GovernorRateLimiter};
use server_config::model::{RateLimitingConfig, SlowlorisConfig};
use server_core::Protocol;
use std::net::IpAddr;
use std::num::NonZeroU32;
use std::sync::Arc;
use tracing::debug;

/// Type alias for the governor rate limiter (not keyed — one instance per IP).
type ProtocolRateLimiter = GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>;

/// Per-protocol rate limiter. Different protocols get different rate limits.
///
/// HTTP, WebSocket, and UDP each have their own per-IP rate limiter with
/// protocol-appropriate quotas. TCP connections fall through to the default
/// rate limiter in [`crate::RateLimiter`] and are always allowed here.
pub struct ProtocolGuard {
    http_limiters: DashMap<IpAddr, Arc<ProtocolRateLimiter>>,
    ws_limiters: DashMap<IpAddr, Arc<ProtocolRateLimiter>>,
    udp_limiters: DashMap<IpAddr, Arc<ProtocolRateLimiter>>,
    http_rate: NonZeroU32,
    ws_rate: NonZeroU32,
    udp_rate: NonZeroU32,
    slowloris: SlowlorisDetector,
}

impl ProtocolGuard {
    /// Create a new `ProtocolGuard` from rate-limiting and slowloris configuration.
    pub fn new(rate_config: &RateLimitingConfig, slowloris_config: &SlowlorisConfig) -> Self {
        let http_rate =
            NonZeroU32::new(rate_config.http_rate_per_sec).unwrap_or(NonZeroU32::new(200).unwrap());
        let ws_rate = NonZeroU32::new(rate_config.ws_messages_per_sec)
            .unwrap_or(NonZeroU32::new(60).unwrap());
        let udp_rate = NonZeroU32::new(rate_config.udp_packets_per_sec)
            .unwrap_or(NonZeroU32::new(500).unwrap());

        Self {
            http_limiters: DashMap::new(),
            ws_limiters: DashMap::new(),
            udp_limiters: DashMap::new(),
            http_rate,
            ws_rate,
            udp_rate,
            slowloris: SlowlorisDetector::new(slowloris_config),
        }
    }

    /// Check rate limit for a specific protocol.
    ///
    /// Returns [`GuardVerdict::Allow`] if the request is within the protocol's
    /// rate limit, or [`GuardVerdict::Block`] with a reason if it exceeds it.
    /// TCP connections always return `Allow` because they are handled by the
    /// default [`crate::RateLimiter`].
    pub fn check(&self, ip: IpAddr, protocol: Protocol) -> GuardVerdict {
        match protocol {
            Protocol::Http => self.check_http(ip),
            Protocol::WebSocket => self.check_ws(ip),
            Protocol::Udp => self.check_udp(ip),
            Protocol::Tcp => GuardVerdict::Allow, // TCP uses the default rate limiter
        }
    }

    /// Check rate limit for HTTP/HTTPS requests.
    fn check_http(&self, ip: IpAddr) -> GuardVerdict {
        let limiter = self
            .http_limiters
            .entry(ip)
            .or_insert_with(|| {
                let quota = Quota::per_second(self.http_rate);
                Arc::new(GovernorRateLimiter::direct(quota))
            })
            .clone();

        match limiter.check() {
            Ok(()) => GuardVerdict::Allow,
            Err(_) => {
                debug!("HTTP rate limit exceeded for IP {}", ip);
                GuardVerdict::Block(format!("HTTP rate limit exceeded for {ip}"))
            }
        }
    }

    /// Check rate limit for WebSocket messages.
    fn check_ws(&self, ip: IpAddr) -> GuardVerdict {
        let limiter = self
            .ws_limiters
            .entry(ip)
            .or_insert_with(|| {
                let quota = Quota::per_second(self.ws_rate);
                Arc::new(GovernorRateLimiter::direct(quota))
            })
            .clone();

        match limiter.check() {
            Ok(()) => GuardVerdict::Allow,
            Err(_) => {
                debug!("WebSocket rate limit exceeded for IP {}", ip);
                GuardVerdict::Block(format!("WebSocket rate limit exceeded for {ip}"))
            }
        }
    }

    /// Check rate limit for UDP packets.
    fn check_udp(&self, ip: IpAddr) -> GuardVerdict {
        let limiter = self
            .udp_limiters
            .entry(ip)
            .or_insert_with(|| {
                let quota = Quota::per_second(self.udp_rate);
                Arc::new(GovernorRateLimiter::direct(quota))
            })
            .clone();

        match limiter.check() {
            Ok(()) => GuardVerdict::Allow,
            Err(_) => {
                debug!("UDP rate limit exceeded for IP {}", ip);
                GuardVerdict::Block(format!("UDP rate limit exceeded for {ip}"))
            }
        }
    }

    /// Access the slowloris detector.
    pub fn slowloris(&self) -> &SlowlorisDetector {
        &self.slowloris
    }
}

// ────────────────────────────────────────────────────────
// Slowloris detection
// ────────────────────────────────────────────────────────

/// Per-connection tracking entry for slowloris detection.
struct SlowlorisEntry {
    /// Total bytes received since tracking started.
    bytes_received: u64,
    /// When tracking started for this connection.
    started_at: chrono::DateTime<chrono::Utc>,
}

/// Slowloris detection — tracks connection data rates.
///
/// A slowloris attack keeps many connections open by sending data at an
/// extremely slow rate, tying up server resources. This detector monitors
/// the bytes-per-second rate for each tracked connection and flags those
/// that fall below the configured minimum after the header timeout.
pub struct SlowlorisDetector {
    enabled: bool,
    min_data_rate: u64,
    header_timeout_secs: u64,
    #[allow(dead_code)]
    body_timeout_secs: u64,
    tracking: DashMap<IpAddr, SlowlorisEntry>,
}

impl SlowlorisDetector {
    /// Create a new slowloris detector from configuration.
    pub fn new(config: &SlowlorisConfig) -> Self {
        Self {
            enabled: config.enabled,
            min_data_rate: config.min_data_rate_bytes_sec,
            header_timeout_secs: config.header_timeout_secs,
            body_timeout_secs: config.body_timeout_secs,
            tracking: DashMap::new(),
        }
    }

    /// Start tracking a connection for slowloris behavior.
    pub fn track_start(&self, ip: IpAddr) {
        if !self.enabled {
            return;
        }
        self.tracking.insert(
            ip,
            SlowlorisEntry {
                bytes_received: 0,
                started_at: chrono::Utc::now(),
            },
        );
        debug!("Slowloris tracking started for IP {}", ip);
    }

    /// Update bytes received for slowloris tracking.
    pub fn track_data(&self, ip: IpAddr, bytes: u64) {
        if let Some(mut entry) = self.tracking.get_mut(&ip) {
            entry.bytes_received += bytes;
        }
    }

    /// Check if a connection is exhibiting slowloris behavior.
    ///
    /// Returns `true` if the connection's data rate is below the configured
    /// minimum and the connection has been open longer than the header timeout.
    /// Returns `false` if tracking is disabled or the IP is not being tracked.
    pub fn is_slowloris(&self, ip: IpAddr) -> bool {
        if !self.enabled {
            return false;
        }
        if let Some(entry) = self.tracking.get(&ip) {
            let elapsed = (chrono::Utc::now() - entry.started_at)
                .num_seconds()
                .max(1) as u64;
            let rate = entry.bytes_received / elapsed;
            rate < self.min_data_rate && elapsed > self.header_timeout_secs
        } else {
            false
        }
    }

    /// Stop tracking a connection.
    pub fn track_stop(&self, ip: IpAddr) {
        self.tracking.remove(&ip);
        debug!("Slowloris tracking stopped for IP {}", ip);
    }

    /// Number of connections being tracked.
    pub fn tracking_count(&self) -> usize {
        self.tracking.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rate_config() -> RateLimitingConfig {
        RateLimitingConfig {
            algorithm: "token_bucket".to_string(),
            default_requests_per_sec: 100,
            burst_size: 50,
            http_rate_per_sec: 5,
            ws_messages_per_sec: 3,
            udp_packets_per_sec: 4,
        }
    }

    fn slowloris_config(enabled: bool) -> SlowlorisConfig {
        SlowlorisConfig {
            enabled,
            min_data_rate_bytes_sec: 100,
            header_timeout_secs: 2,
            body_timeout_secs: 10,
        }
    }

    // ── Protocol rate limit tests ──────────────────────────

    #[test]
    fn test_http_rate_limit() {
        let guard = ProtocolGuard::new(
            &RateLimitingConfig {
                http_rate_per_sec: 1,
                ..rate_config()
            },
            &slowloris_config(false),
        );
        let ip: IpAddr = "10.0.0.1".parse().unwrap();

        // First HTTP request within quota
        assert_eq!(guard.check(ip, Protocol::Http), GuardVerdict::Allow);

        // Second HTTP request should be blocked (quota = 1/sec, no burst beyond 1)
        let verdict = guard.check(ip, Protocol::Http);
        assert!(matches!(verdict, GuardVerdict::Block(_)));
    }

    #[test]
    fn test_ws_rate_limit() {
        let guard = ProtocolGuard::new(
            &RateLimitingConfig {
                ws_messages_per_sec: 1,
                ..rate_config()
            },
            &slowloris_config(false),
        );
        let ip: IpAddr = "10.0.0.2".parse().unwrap();

        assert_eq!(guard.check(ip, Protocol::WebSocket), GuardVerdict::Allow);

        let verdict = guard.check(ip, Protocol::WebSocket);
        assert!(matches!(verdict, GuardVerdict::Block(_)));
    }

    #[test]
    fn test_udp_rate_limit() {
        let guard = ProtocolGuard::new(
            &RateLimitingConfig {
                udp_packets_per_sec: 1,
                ..rate_config()
            },
            &slowloris_config(false),
        );
        let ip: IpAddr = "10.0.0.3".parse().unwrap();

        assert_eq!(guard.check(ip, Protocol::Udp), GuardVerdict::Allow);

        let verdict = guard.check(ip, Protocol::Udp);
        assert!(matches!(verdict, GuardVerdict::Block(_)));
    }

    #[test]
    fn test_tcp_bypasses_protocol_guard() {
        let guard = ProtocolGuard::new(&rate_config(), &slowloris_config(false));
        let ip: IpAddr = "10.0.0.4".parse().unwrap();

        // TCP always returns Allow regardless of how many times we check
        for _ in 0..100 {
            assert_eq!(guard.check(ip, Protocol::Tcp), GuardVerdict::Allow);
        }
    }

    // ── Slowloris tests ────────────────────────────────────

    #[test]
    fn test_slowloris_detection() {
        let config = SlowlorisConfig {
            enabled: true,
            min_data_rate_bytes_sec: 100,
            // Use 0 so the elapsed time (>= 1 second clamp) immediately qualifies
            header_timeout_secs: 0,
            body_timeout_secs: 10,
        };
        let detector = SlowlorisDetector::new(&config);
        let ip: IpAddr = "10.0.0.5".parse().unwrap();

        detector.track_start(ip);
        // Send very little data (1 byte) — rate will be < 100 B/s
        detector.track_data(ip, 1);

        // elapsed is clamped to at least 1 second, rate = 1/1 = 1 < 100
        // header_timeout_secs = 0, and elapsed (1) > 0
        assert!(detector.is_slowloris(ip));
    }

    #[test]
    fn test_slowloris_disabled() {
        let detector = SlowlorisDetector::new(&slowloris_config(false));
        let ip: IpAddr = "10.0.0.6".parse().unwrap();

        // Even if we track, disabled means no detection
        detector.track_start(ip);
        assert_eq!(detector.tracking_count(), 0); // track_start is a no-op when disabled
        assert!(!detector.is_slowloris(ip));
    }

    #[test]
    fn test_slowloris_normal_traffic() {
        let config = SlowlorisConfig {
            enabled: true,
            min_data_rate_bytes_sec: 100,
            header_timeout_secs: 0,
            body_timeout_secs: 10,
        };
        let detector = SlowlorisDetector::new(&config);
        let ip: IpAddr = "10.0.0.7".parse().unwrap();

        detector.track_start(ip);
        // Send enough data that rate stays above minimum
        // elapsed is at least 1s, so 1000 bytes / 1s = 1000 > 100
        detector.track_data(ip, 1000);

        assert!(!detector.is_slowloris(ip));
    }

    #[test]
    fn test_track_start_stop() {
        let detector = SlowlorisDetector::new(&slowloris_config(true));
        let ip1: IpAddr = "10.0.0.8".parse().unwrap();
        let ip2: IpAddr = "10.0.0.9".parse().unwrap();

        assert_eq!(detector.tracking_count(), 0);

        detector.track_start(ip1);
        assert_eq!(detector.tracking_count(), 1);

        detector.track_start(ip2);
        assert_eq!(detector.tracking_count(), 2);

        detector.track_stop(ip1);
        assert_eq!(detector.tracking_count(), 1);

        detector.track_stop(ip2);
        assert_eq!(detector.tracking_count(), 0);

        // Stopping a non-tracked IP is a no-op
        detector.track_stop(ip1);
        assert_eq!(detector.tracking_count(), 0);
    }
}
