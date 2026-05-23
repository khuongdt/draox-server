//! TCP half-open connection (SYN) tracker.
//!
//! Detects SYN-flood attacks by tracking connections that have been initiated
//! but have not yet completed the TCP handshake. If an IP accumulates more
//! than `max_half_open_per_ip` pending half-open connections, new SYNs from
//! that IP are rejected.

use dashmap::DashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};

/// Tracks TCP connections that connected but have not yet completed the
/// three-way handshake.
///
/// Internally stores a list of [`Instant`] timestamps (one per half-open
/// connection) keyed by remote IP. When `track_syn` is called the tracker
/// first evicts entries older than `handshake_timeout` before enforcing the
/// per-IP limit, so cleanup happens lazily as part of normal operation.
pub struct SynTracker {
    /// Map from remote IP to list of SYN-received timestamps.
    half_open: DashMap<IpAddr, Vec<Instant>>,
    /// Maximum number of concurrent half-open connections allowed per IP.
    max_half_open_per_ip: u32,
    /// How long a half-open entry is kept before it is considered expired.
    handshake_timeout: Duration,
}

impl SynTracker {
    /// Create a new `SynTracker`.
    ///
    /// - `max_per_ip` — reject a new SYN when the IP already has this many
    ///   outstanding half-open connections.
    /// - `timeout` — a half-open entry is considered stale after this duration
    ///   and removed automatically on the next call to `track_syn` or
    ///   `cleanup_expired`.
    pub fn new(max_per_ip: u32, timeout: Duration) -> Self {
        Self {
            half_open: DashMap::new(),
            max_half_open_per_ip: max_per_ip,
            handshake_timeout: timeout,
        }
    }

    /// Record a new incoming SYN from `ip`.
    ///
    /// Returns `true` if the SYN is accepted (the half-open count is within
    /// the limit), or `false` if the IP has already reached its limit and the
    /// SYN should be dropped.
    ///
    /// Expired entries are cleaned up lazily before the limit is checked.
    pub fn track_syn(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let timeout = self.handshake_timeout;
        let limit = self.max_half_open_per_ip as usize;

        let mut entry = self.half_open.entry(ip).or_default();

        // Evict expired entries first (lazy cleanup).
        entry.retain(|&t| now.duration_since(t) < timeout);

        if entry.len() >= limit {
            return false;
        }

        entry.push(now);
        true
    }

    /// Signal that `ip` has completed the handshake.
    ///
    /// Removes the oldest half-open entry for that IP (FIFO). If the IP has
    /// no tracked entries this is a no-op.
    pub fn complete_handshake(&self, ip: IpAddr) {
        if let Some(mut entry) = self.half_open.get_mut(&ip) {
            if !entry.is_empty() {
                entry.remove(0);
            }
        }
    }

    /// Remove all half-open entries that have exceeded `handshake_timeout`.
    ///
    /// This should be called periodically by a background task. Individual
    /// calls to `track_syn` also perform lazy cleanup, so this method exists
    /// mainly to reclaim memory for IPs that stop sending traffic.
    pub fn cleanup_expired(&self) {
        let now = Instant::now();
        let timeout = self.handshake_timeout;

        self.half_open.retain(|_ip, times| {
            times.retain(|&t| now.duration_since(t) < timeout);
            // Remove the map entry entirely if there are no remaining slots.
            !times.is_empty()
        });
    }

    /// Return the current number of half-open connections for `ip`.
    ///
    /// Expired entries are *not* evicted here; call `cleanup_expired` or
    /// `track_syn` to trigger cleanup. This keeps the method `&self` and
    /// avoids mutable borrows in read-heavy contexts.
    pub fn half_open_count(&self, ip: IpAddr) -> u32 {
        let now = Instant::now();
        let timeout = self.handshake_timeout;

        self.half_open
            .get(&ip)
            .map(|entry| {
                entry
                    .iter()
                    .filter(|&&t| now.duration_since(t) < timeout)
                    .count() as u32
            })
            .unwrap_or(0)
    }
}

// ────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn test_track_syn_within_limit() {
        let tracker = SynTracker::new(3, Duration::from_secs(30));
        let addr = ip("10.0.0.1");

        assert!(tracker.track_syn(addr), "first SYN should be accepted");
        assert!(tracker.track_syn(addr), "second SYN should be accepted");
        assert!(tracker.track_syn(addr), "third SYN should be accepted");
        assert_eq!(tracker.half_open_count(addr), 3);
    }

    #[test]
    fn test_track_syn_over_limit_rejected() {
        let tracker = SynTracker::new(2, Duration::from_secs(30));
        let addr = ip("10.0.0.2");

        assert!(tracker.track_syn(addr));
        assert!(tracker.track_syn(addr));
        // Third SYN should be rejected — limit is 2.
        assert!(!tracker.track_syn(addr), "SYN over limit should be rejected");
        assert_eq!(tracker.half_open_count(addr), 2);
    }

    #[test]
    fn test_complete_handshake_decrements_count() {
        let tracker = SynTracker::new(5, Duration::from_secs(30));
        let addr = ip("172.16.0.1");

        tracker.track_syn(addr);
        tracker.track_syn(addr);
        assert_eq!(tracker.half_open_count(addr), 2);

        tracker.complete_handshake(addr);
        assert_eq!(tracker.half_open_count(addr), 1);

        tracker.complete_handshake(addr);
        assert_eq!(tracker.half_open_count(addr), 0);
    }

    #[test]
    fn test_cleanup_expired_removes_stale_entries() {
        // Use a very short timeout so entries expire immediately.
        let tracker = SynTracker::new(10, Duration::from_millis(1));
        let addr = ip("192.168.1.1");

        tracker.track_syn(addr);
        tracker.track_syn(addr);

        // Wait just long enough for entries to expire.
        std::thread::sleep(Duration::from_millis(5));

        tracker.cleanup_expired();

        // All expired entries should be gone.
        assert_eq!(tracker.half_open_count(addr), 0);
    }

    #[test]
    fn test_independent_ips_do_not_share_limits() {
        let tracker = SynTracker::new(1, Duration::from_secs(30));
        let addr1 = ip("1.1.1.1");
        let addr2 = ip("2.2.2.2");

        // Each IP gets its own counter.
        assert!(tracker.track_syn(addr1));
        assert!(tracker.track_syn(addr2));

        // addr1 is at limit, addr2 is at limit independently.
        assert!(!tracker.track_syn(addr1), "addr1 should be at limit");
        assert!(!tracker.track_syn(addr2), "addr2 should be at limit");
    }
}
