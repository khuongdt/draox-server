use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

// ────────────────────────────────────────────────────────
// MetricsCollector
// ────────────────────────────────────────────────────────

/// Thread-safe, lock-free metrics collector using atomics.
///
/// All counter operations use `Ordering::Relaxed` since exact
/// cross-thread ordering is not critical for metrics.
pub struct MetricsCollector {
    pub connections_total: AtomicU64,
    pub connections_active: AtomicI64,
    pub bytes_received_total: AtomicU64,
    pub bytes_sent_total: AtomicU64,
    pub requests_total: AtomicU64,
    pub errors_total: AtomicU64,
}

impl MetricsCollector {
    /// Create a new MetricsCollector with all counters at zero.
    pub fn new() -> Self {
        Self {
            connections_total: AtomicU64::new(0),
            connections_active: AtomicI64::new(0),
            bytes_received_total: AtomicU64::new(0),
            bytes_sent_total: AtomicU64::new(0),
            requests_total: AtomicU64::new(0),
            errors_total: AtomicU64::new(0),
        }
    }

    /// Increment total connection count and active connection gauge.
    pub fn increment_connections(&self) {
        self.connections_total.fetch_add(1, Ordering::Relaxed);
        self.connections_active.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement the active connection gauge.
    pub fn decrement_connections(&self) {
        self.connections_active.fetch_sub(1, Ordering::Relaxed);
    }

    /// Add `n` to the total bytes received counter.
    pub fn record_bytes_received(&self, n: u64) {
        self.bytes_received_total.fetch_add(n, Ordering::Relaxed);
    }

    /// Add `n` to the total bytes sent counter.
    pub fn record_bytes_sent(&self, n: u64) {
        self.bytes_sent_total.fetch_add(n, Ordering::Relaxed);
    }

    /// Increment the total requests counter.
    pub fn increment_requests(&self) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment the total errors counter.
    pub fn increment_errors(&self) {
        self.errors_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Take a point-in-time snapshot of all metrics.
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            connections_total: self.connections_total.load(Ordering::Relaxed),
            connections_active: self.connections_active.load(Ordering::Relaxed),
            bytes_received_total: self.bytes_received_total.load(Ordering::Relaxed),
            bytes_sent_total: self.bytes_sent_total.load(Ordering::Relaxed),
            requests_total: self.requests_total.load(Ordering::Relaxed),
            errors_total: self.errors_total.load(Ordering::Relaxed),
            timestamp: Utc::now(),
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

// ────────────────────────────────────────────────────────
// MetricsSnapshot
// ────────────────────────────────────────────────────────

/// An immutable, serializable snapshot of all server metrics at a
/// specific point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub connections_total: u64,
    pub connections_active: i64,
    pub bytes_received_total: u64,
    pub bytes_sent_total: u64,
    pub requests_total: u64,
    pub errors_total: u64,
    pub timestamp: DateTime<Utc>,
}

// ────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_increment() {
        let m = MetricsCollector::new();

        m.increment_connections();
        m.increment_connections();
        m.increment_connections();
        m.decrement_connections();

        m.increment_requests();
        m.increment_requests();

        m.increment_errors();

        assert_eq!(m.connections_total.load(Ordering::Relaxed), 3);
        assert_eq!(m.connections_active.load(Ordering::Relaxed), 2);
        assert_eq!(m.requests_total.load(Ordering::Relaxed), 2);
        assert_eq!(m.errors_total.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_metrics_snapshot() {
        let m = MetricsCollector::new();

        m.increment_connections();
        m.increment_connections();
        m.decrement_connections();
        m.increment_requests();
        m.increment_errors();
        m.record_bytes_received(1024);
        m.record_bytes_sent(512);

        let snap = m.snapshot();

        assert_eq!(snap.connections_total, 2);
        assert_eq!(snap.connections_active, 1);
        assert_eq!(snap.requests_total, 1);
        assert_eq!(snap.errors_total, 1);
        assert_eq!(snap.bytes_received_total, 1024);
        assert_eq!(snap.bytes_sent_total, 512);
        // Timestamp should be roughly "now".
        assert!(snap.timestamp <= Utc::now());
    }

    #[test]
    fn test_metrics_bytes_tracking() {
        let m = MetricsCollector::new();

        m.record_bytes_received(100);
        m.record_bytes_received(200);
        m.record_bytes_received(300);

        m.record_bytes_sent(50);
        m.record_bytes_sent(150);

        assert_eq!(m.bytes_received_total.load(Ordering::Relaxed), 600);
        assert_eq!(m.bytes_sent_total.load(Ordering::Relaxed), 200);
    }
}
