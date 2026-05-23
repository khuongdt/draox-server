//! Connection-level heartbeat management.
//!
//! The existing `heartbeat.rs` module handles *session* expiry (cleaning up
//! empty sessions after a grace period). This module tracks individual
//! *connection* heartbeat state — ping/pong cycles — to detect stale
//! connections that are still "open" at the socket layer but no longer
//! responsive.
//!
//! Typical usage:
//! 1. Register a connection on accept.
//! 2. Periodically call `connections_to_ping` and send pings.
//! 3. Call `record_pong` when a pong arrives.
//! 4. Call `check_all` to collect connections that have missed too many heartbeats.
//! 5. Unregister when a connection closes.

use dashmap::DashMap;
use server_core::ConnectionId;
use std::time::{Duration, Instant};

/// Per-connection heartbeat tracking state.
struct HeartbeatState {
    /// When the last ping was sent to this connection.
    last_ping_sent: Instant,
    /// When the most recent pong was received (None until the first pong).
    last_pong_received: Option<Instant>,
    /// Number of consecutive pings without a matching pong.
    missed_count: u32,
}

/// Tracks ping/pong heartbeat state for every registered connection.
///
/// - A connection is "due for a ping" when `now - last_ping_sent >= interval`.
/// - A connection is "timed out" when `now - last_ping_sent >= timeout` **and**
///   no pong has arrived since the last ping.
pub struct HeartbeatManager {
    /// connection_id → heartbeat state.
    intervals: DashMap<ConnectionId, HeartbeatState>,
    /// How often pings should be sent.
    default_interval: Duration,
    /// How long without a pong before a connection is considered dead.
    timeout: Duration,
}

impl HeartbeatManager {
    /// Create a new `HeartbeatManager`.
    ///
    /// - `interval` — how often each connection should receive a ping.
    /// - `timeout` — how long to wait for a pong before declaring the
    ///   connection dead (should be ≥ `interval`).
    pub fn new(interval: Duration, timeout: Duration) -> Self {
        Self {
            intervals: DashMap::new(),
            default_interval: interval,
            timeout,
        }
    }

    /// Start tracking heartbeats for a new connection.
    ///
    /// The first ping will be sent after one `default_interval` has elapsed.
    /// Registering an already-registered connection is a no-op.
    pub fn register(&self, conn_id: ConnectionId) {
        self.intervals.entry(conn_id).or_insert_with(|| HeartbeatState {
            last_ping_sent: Instant::now(),
            last_pong_received: None,
            missed_count: 0,
        });
    }

    /// Stop tracking heartbeats for a connection (called on disconnect).
    pub fn unregister(&self, conn_id: &ConnectionId) {
        self.intervals.remove(conn_id);
    }

    /// Record that a pong was received for `conn_id`.
    ///
    /// Resets `missed_count` to 0 and updates `last_pong_received`.
    /// If the connection is not registered this is a no-op.
    pub fn record_pong(&self, conn_id: &ConnectionId) {
        if let Some(mut state) = self.intervals.get_mut(conn_id) {
            state.last_pong_received = Some(Instant::now());
            state.missed_count = 0;
        }
    }

    /// Check all registered connections for heartbeat timeouts.
    ///
    /// A connection is considered to have missed a heartbeat when:
    /// - A ping was sent more than `timeout` ago, **and**
    /// - No pong has been received since that ping.
    ///
    /// For each such connection `missed_count` is incremented and a new
    /// "ping sent" timestamp is recorded (so the caller only needs to evict
    /// or re-ping — not call `record_pong`).
    ///
    /// Returns the list of connection IDs that have missed at least one
    /// heartbeat in this check cycle. Callers typically disconnect connections
    /// whose `missed_count` exceeds a threshold.
    pub fn check_all(&self) -> Vec<ConnectionId> {
        let now = Instant::now();
        let timeout = self.timeout;
        let mut missed = Vec::new();

        for mut entry in self.intervals.iter_mut() {
            let state = entry.value_mut();
            let elapsed_since_ping = now.duration_since(state.last_ping_sent);

            if elapsed_since_ping >= timeout {
                // Check whether a pong arrived after the last ping.
                let pong_ok = state
                    .last_pong_received
                    .map(|t| t > state.last_ping_sent)
                    .unwrap_or(false);

                if !pong_ok {
                    state.missed_count += 1;
                    // Advance last_ping_sent so we don't flag the same
                    // timeout window repeatedly.
                    state.last_ping_sent = now;
                    missed.push(entry.key().clone());
                } else {
                    // Pong was received; reset and advance the window.
                    state.missed_count = 0;
                    state.last_ping_sent = now;
                }
            }
        }

        missed
    }

    /// Return the IDs of connections that are due for a new ping.
    ///
    /// A connection is due when `now - last_ping_sent >= default_interval`.
    /// Calling this method does **not** update `last_ping_sent`; the caller
    /// is responsible for updating that timestamp (e.g. by calling
    /// `check_all` which also advances the timestamp, or by mutating state
    /// directly after sending the ping).
    pub fn connections_to_ping(&self) -> Vec<ConnectionId> {
        let now = Instant::now();
        let interval = self.default_interval;

        self.intervals
            .iter()
            .filter_map(|entry| {
                let elapsed = now.duration_since(entry.value().last_ping_sent);
                if elapsed >= interval {
                    Some(entry.key().clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Return the current `missed_count` for a connection (for testing/admin).
    #[cfg(test)]
    fn missed_count(&self, conn_id: &ConnectionId) -> Option<u32> {
        self.intervals.get(conn_id).map(|s| s.missed_count)
    }
}

// ────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_unregister() {
        let hm = HeartbeatManager::new(Duration::from_secs(5), Duration::from_secs(15));
        let conn = ConnectionId::new();

        hm.register(conn.clone());
        assert!(hm.intervals.contains_key(&conn));

        hm.unregister(&conn);
        assert!(!hm.intervals.contains_key(&conn));
    }

    #[test]
    fn test_record_pong_resets_missed_count() {
        let hm = HeartbeatManager::new(Duration::from_millis(1), Duration::from_millis(1));
        let conn = ConnectionId::new();
        hm.register(conn.clone());

        // Trigger a missed heartbeat.
        std::thread::sleep(Duration::from_millis(5));
        let missed = hm.check_all();
        assert!(missed.contains(&conn));
        assert_eq!(hm.missed_count(&conn), Some(1));

        // Now send a pong — missed_count should reset on next check.
        hm.record_pong(&conn);
        std::thread::sleep(Duration::from_millis(5));
        let missed2 = hm.check_all();
        // After a pong the connection should not appear in the missed list.
        assert!(!missed2.contains(&conn));
        assert_eq!(hm.missed_count(&conn), Some(0));
    }

    #[test]
    fn test_check_all_detects_missed_heartbeat() {
        let hm = HeartbeatManager::new(Duration::from_millis(1), Duration::from_millis(1));
        let conn = ConnectionId::new();
        hm.register(conn.clone());

        // No time has passed — nothing should be missed yet.
        // (Depending on scheduler, the check may or may not fire.)
        // Wait long enough for the timeout to elapse.
        std::thread::sleep(Duration::from_millis(10));

        let missed = hm.check_all();
        assert!(missed.contains(&conn));
    }

    #[test]
    fn test_connections_to_ping() {
        // Interval of 1 ms so connections become due almost immediately.
        let hm = HeartbeatManager::new(Duration::from_millis(1), Duration::from_secs(30));
        let conn1 = ConnectionId::new();
        let conn2 = ConnectionId::new();

        hm.register(conn1.clone());
        hm.register(conn2.clone());

        std::thread::sleep(Duration::from_millis(5));

        let to_ping = hm.connections_to_ping();
        assert!(to_ping.contains(&conn1));
        assert!(to_ping.contains(&conn2));
    }

    #[test]
    fn test_unregistered_pong_is_noop() {
        let hm = HeartbeatManager::new(Duration::from_secs(5), Duration::from_secs(15));
        let unknown = ConnectionId::new();
        // Should not panic.
        hm.record_pong(&unknown);
    }
}
