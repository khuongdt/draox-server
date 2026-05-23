use crate::ban_manager::BanManager;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use server_config::model::BanningConfig;
use std::net::IpAddr;
use std::sync::Arc;
use tracing::{debug, warn};

/// Internal record tracking authentication failures for a single IP.
struct AuthFailureRecord {
    /// Number of failures in the current window.
    count: u32,
    /// When the current failure window started.
    window_start: DateTime<Utc>,
}

/// Tracks authentication failures per IP address and auto-bans after threshold.
///
/// Each IP gets a sliding window: if the number of failures within
/// `window_secs` reaches `threshold`, the IP is automatically banned
/// via the shared `BanManager`.
pub struct AuthFailureTracker {
    records: DashMap<IpAddr, AuthFailureRecord>,
    threshold: u32,
    window_secs: u64,
    ban_manager: Arc<BanManager>,
}

impl AuthFailureTracker {
    /// Create a new tracker from banning config and a shared ban manager.
    pub fn new(config: &BanningConfig, ban_manager: Arc<BanManager>) -> Self {
        Self {
            records: DashMap::new(),
            threshold: config.auth_failure_threshold,
            window_secs: config.auth_failure_window_secs,
            ban_manager,
        }
    }

    /// Record an authentication failure for the given IP.
    ///
    /// Returns `true` if the IP was auto-banned as a result of this failure
    /// (i.e., the failure count reached the configured threshold).
    pub fn record_failure(&self, ip: IpAddr) -> bool {
        let now = Utc::now();
        let mut entry = self.records.entry(ip).or_insert_with(|| AuthFailureRecord {
            count: 0,
            window_start: now,
        });

        // If the window has expired, reset the counter
        let elapsed = (now - entry.window_start).num_seconds() as u64;
        if elapsed >= self.window_secs {
            entry.count = 0;
            entry.window_start = now;
            debug!("Auth failure window reset for IP {}", ip);
        }

        entry.count += 1;
        let count = entry.count;
        let threshold = self.threshold;

        debug!(
            "Auth failure recorded for IP {} (count: {}/{})",
            ip, count, threshold
        );

        if count >= threshold {
            // Drop the entry ref before calling ban (which may also access DashMap)
            drop(entry);
            warn!(
                "IP {} exceeded auth failure threshold ({}/{}), auto-banning",
                ip, count, threshold
            );
            self.ban_manager
                .ban(ip, "auth failure limit exceeded");
            // Remove the record after banning
            self.records.remove(&ip);
            return true;
        }

        false
    }

    /// Reset the failure count for an IP (e.g., on successful authentication).
    pub fn reset(&self, ip: IpAddr) {
        if self.records.remove(&ip).is_some() {
            debug!("Auth failure record reset for IP {}", ip);
        }
    }

    /// Get the current failure count for an IP within the active window.
    ///
    /// Returns 0 if no failures are recorded or the window has expired.
    pub fn failure_count(&self, ip: IpAddr) -> u32 {
        let now = Utc::now();
        if let Some(entry) = self.records.get(&ip) {
            let elapsed = (now - entry.window_start).num_seconds() as u64;
            if elapsed >= self.window_secs {
                return 0;
            }
            entry.count
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> BanningConfig {
        BanningConfig {
            enabled: true,
            max_violations_before_ban: 5,
            initial_ban_duration_secs: 300,
            ban_escalation_multiplier: 2,
            max_ban_duration_secs: 3600,
            auth_failure_threshold: 3,
            auth_failure_window_secs: 300,
        }
    }

    #[test]
    fn test_record_failures_up_to_threshold() {
        let config = test_config();
        let ban_manager = Arc::new(BanManager::new(config.clone()));
        let tracker = AuthFailureTracker::new(&config, Arc::clone(&ban_manager));

        let ip: IpAddr = "192.168.1.10".parse().unwrap();

        // First failure — not banned yet
        assert!(!tracker.record_failure(ip));
        assert_eq!(tracker.failure_count(ip), 1);

        // Second failure — still not banned
        assert!(!tracker.record_failure(ip));
        assert_eq!(tracker.failure_count(ip), 2);

        // Third failure — threshold reached, should be banned
        assert!(tracker.record_failure(ip));
        assert!(ban_manager.is_banned(ip).is_some());
    }

    #[test]
    fn test_window_reset() {
        let config = BanningConfig {
            enabled: true,
            max_violations_before_ban: 5,
            initial_ban_duration_secs: 300,
            ban_escalation_multiplier: 2,
            max_ban_duration_secs: 3600,
            auth_failure_threshold: 3,
            // Use 0 seconds window so it expires immediately
            auth_failure_window_secs: 0,
        };
        let ban_manager = Arc::new(BanManager::new(config.clone()));
        let tracker = AuthFailureTracker::new(&config, Arc::clone(&ban_manager));

        let ip: IpAddr = "10.0.0.1".parse().unwrap();

        // Record two failures
        tracker.record_failure(ip);
        tracker.record_failure(ip);

        // With a 0-second window, the count should have been reset each time.
        // The failure_count also checks window expiration.
        assert_eq!(tracker.failure_count(ip), 0);
    }

    #[test]
    fn test_explicit_reset() {
        let config = test_config();
        let ban_manager = Arc::new(BanManager::new(config.clone()));
        let tracker = AuthFailureTracker::new(&config, Arc::clone(&ban_manager));

        let ip: IpAddr = "172.16.0.5".parse().unwrap();

        tracker.record_failure(ip);
        tracker.record_failure(ip);
        assert_eq!(tracker.failure_count(ip), 2);

        // Explicit reset (e.g., successful auth)
        tracker.reset(ip);
        assert_eq!(tracker.failure_count(ip), 0);
    }

    #[test]
    fn test_independent_ips() {
        let config = test_config();
        let ban_manager = Arc::new(BanManager::new(config.clone()));
        let tracker = AuthFailureTracker::new(&config, Arc::clone(&ban_manager));

        let ip1: IpAddr = "192.168.1.1".parse().unwrap();
        let ip2: IpAddr = "192.168.1.2".parse().unwrap();

        tracker.record_failure(ip1);
        tracker.record_failure(ip1);

        assert_eq!(tracker.failure_count(ip1), 2);
        assert_eq!(tracker.failure_count(ip2), 0);
    }

    #[test]
    fn test_ban_clears_record() {
        let config = test_config();
        let ban_manager = Arc::new(BanManager::new(config.clone()));
        let tracker = AuthFailureTracker::new(&config, Arc::clone(&ban_manager));

        let ip: IpAddr = "10.10.10.10".parse().unwrap();

        // Reach threshold
        tracker.record_failure(ip);
        tracker.record_failure(ip);
        assert!(tracker.record_failure(ip));

        // Record should be cleared after ban
        assert_eq!(tracker.failure_count(ip), 0);
    }
}
