use dashmap::DashMap;
use server_config::model::ConnectionLimitsConfig;
use std::net::IpAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tracing::debug;

/// Per-IP concurrent connection limiter.
///
/// Tracks the number of active connections per IP address using atomic
/// counters. When the limit is reached, new connections from that IP
/// are rejected until existing connections are released.
pub struct ConcurrentConnectionLimiter {
    connections: DashMap<IpAddr, Arc<AtomicU32>>,
    max_per_ip: u32,
}

impl ConcurrentConnectionLimiter {
    /// Create a new limiter from connection limits configuration.
    pub fn new(config: &ConnectionLimitsConfig) -> Self {
        Self {
            connections: DashMap::new(),
            max_per_ip: config.max_connections_per_ip,
        }
    }

    /// Try to add a connection for the given IP.
    ///
    /// Returns `true` if the connection was accepted (under the limit).
    /// Returns `false` if the per-IP limit has been reached.
    pub fn try_add(&self, ip: IpAddr) -> bool {
        let counter = self
            .connections
            .entry(ip)
            .or_insert_with(|| Arc::new(AtomicU32::new(0)))
            .clone();

        // Atomically try to increment, but only if under the limit
        loop {
            let current = counter.load(Ordering::Acquire);
            if current >= self.max_per_ip {
                debug!(
                    "Concurrent connection limit reached for IP {} ({}/{})",
                    ip, current, self.max_per_ip
                );
                return false;
            }
            if counter
                .compare_exchange_weak(current, current + 1, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                debug!(
                    "Connection added for IP {} ({}/{})",
                    ip,
                    current + 1,
                    self.max_per_ip
                );
                return true;
            }
            // CAS failed due to contention, retry
        }
    }

    /// Remove a connection for the given IP (call on disconnect).
    ///
    /// Decrements the counter. If it reaches zero, the entry is removed
    /// from the map to avoid unbounded memory growth.
    pub fn remove(&self, ip: IpAddr) {
        let should_cleanup = if let Some(counter) = self.connections.get(&ip) {
            let prev = counter.fetch_sub(1, Ordering::AcqRel);
            debug!("Connection removed for IP {} ({}/{})", ip, prev - 1, self.max_per_ip);
            prev <= 1
        } else {
            false
        };

        // Clean up entry outside the get() borrow to avoid deadlock
        if should_cleanup {
            self.connections.remove_if(&ip, |_, v| v.load(Ordering::Acquire) == 0);
        }
    }

    /// Get the current connection count for an IP.
    pub fn count(&self, ip: IpAddr) -> u32 {
        self.connections
            .get(&ip)
            .map(|c| c.load(Ordering::Acquire))
            .unwrap_or(0)
    }

    /// Number of IPs currently being tracked.
    pub fn tracked_ips(&self) -> usize {
        self.connections.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(max: u32) -> ConnectionLimitsConfig {
        ConnectionLimitsConfig {
            max_connections_per_ip: max,
            max_new_connections_per_sec_per_ip: 10,
            max_new_connections_per_sec_global: 1000,
            max_half_open_connections: 500,
            connection_timeout_secs: 10,
        }
    }

    #[test]
    fn test_add_up_to_limit() {
        let limiter = ConcurrentConnectionLimiter::new(&test_config(3));
        let ip: IpAddr = "192.168.1.1".parse().unwrap();

        assert!(limiter.try_add(ip));
        assert!(limiter.try_add(ip));
        assert!(limiter.try_add(ip));
        assert_eq!(limiter.count(ip), 3);

        // Exceeding the limit should fail
        assert!(!limiter.try_add(ip));
        assert_eq!(limiter.count(ip), 3);
    }

    #[test]
    fn test_remove_allows_new() {
        let limiter = ConcurrentConnectionLimiter::new(&test_config(2));
        let ip: IpAddr = "10.0.0.1".parse().unwrap();

        assert!(limiter.try_add(ip));
        assert!(limiter.try_add(ip));
        assert!(!limiter.try_add(ip));

        // Remove one connection
        limiter.remove(ip);
        assert_eq!(limiter.count(ip), 1);

        // Should be able to add again
        assert!(limiter.try_add(ip));
        assert_eq!(limiter.count(ip), 2);
    }

    #[test]
    fn test_independent_ips() {
        let limiter = ConcurrentConnectionLimiter::new(&test_config(2));
        let ip1: IpAddr = "192.168.1.1".parse().unwrap();
        let ip2: IpAddr = "192.168.1.2".parse().unwrap();

        assert!(limiter.try_add(ip1));
        assert!(limiter.try_add(ip1));
        assert!(!limiter.try_add(ip1));

        // ip2 should still be able to connect
        assert!(limiter.try_add(ip2));
        assert!(limiter.try_add(ip2));
        assert!(!limiter.try_add(ip2));

        assert_eq!(limiter.tracked_ips(), 2);
    }

    #[test]
    fn test_cleanup_on_zero() {
        let limiter = ConcurrentConnectionLimiter::new(&test_config(2));
        let ip: IpAddr = "172.16.0.1".parse().unwrap();

        assert!(limiter.try_add(ip));
        assert_eq!(limiter.tracked_ips(), 1);

        limiter.remove(ip);
        assert_eq!(limiter.count(ip), 0);
        assert_eq!(limiter.tracked_ips(), 0);
    }

    #[test]
    fn test_count_unknown_ip() {
        let limiter = ConcurrentConnectionLimiter::new(&test_config(10));
        let ip: IpAddr = "1.2.3.4".parse().unwrap();

        assert_eq!(limiter.count(ip), 0);
    }
}
