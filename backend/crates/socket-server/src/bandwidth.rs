use dashmap::DashMap;
use server_core::ConnectionId;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tracing::debug;

/// Per-connection bandwidth state.
struct BandwidthEntry {
    tokens: AtomicU64,
    last_refill: std::sync::Mutex<Instant>,
    bytes_per_sec: u64,
    burst_bytes: u64,
}

/// Per-connection bandwidth throttle using a token bucket.
///
/// Each connection has a bucket of tokens representing bytes that can be
/// sent or received. Tokens refill at `bytes_per_sec` up to `burst_bytes`.
/// Consuming bytes removes tokens; if the bucket is empty the operation
/// is rejected so callers can back off or drop data.
pub struct BandwidthThrottle {
    connections: DashMap<String, BandwidthEntry>,
    default_bytes_per_sec: u64,
    default_burst_bytes: u64,
}

impl BandwidthThrottle {
    /// Create a new throttle with the given defaults for every connection.
    pub fn new(default_bytes_per_sec: u64, default_burst_bytes: u64) -> Self {
        Self {
            connections: DashMap::new(),
            default_bytes_per_sec,
            default_burst_bytes,
        }
    }

    /// Register a connection with the default bandwidth limit.
    pub fn register(&self, conn_id: &ConnectionId) {
        self.register_with_limit(
            conn_id,
            self.default_bytes_per_sec,
            self.default_burst_bytes,
        );
    }

    /// Register a connection with a custom bandwidth limit.
    pub fn register_with_limit(
        &self,
        conn_id: &ConnectionId,
        bytes_per_sec: u64,
        burst_bytes: u64,
    ) {
        debug!(
            conn_id = %conn_id,
            bytes_per_sec,
            burst_bytes,
            "bandwidth: registering connection"
        );
        self.connections.insert(
            conn_id.as_str().to_owned(),
            BandwidthEntry {
                tokens: AtomicU64::new(burst_bytes),
                last_refill: std::sync::Mutex::new(Instant::now()),
                bytes_per_sec,
                burst_bytes,
            },
        );
    }

    /// Try to consume `bytes` tokens from the connection's bucket.
    ///
    /// Returns `true` if the tokens were available and consumed, `false` if
    /// the connection would exceed its limit. Unknown (unregistered)
    /// connections are **not** throttled and always return `true`.
    pub fn try_consume(&self, conn_id: &ConnectionId, bytes: u64) -> bool {
        if let Some(entry) = self.connections.get(conn_id.as_str()) {
            // Refill tokens based on elapsed time
            let mut last = entry.last_refill.lock().unwrap();
            let elapsed = last.elapsed();
            let refill = (elapsed.as_secs_f64() * entry.bytes_per_sec as f64) as u64;
            if refill > 0 {
                let current = entry.tokens.load(Ordering::Relaxed);
                let new_tokens = (current + refill).min(entry.burst_bytes);
                entry.tokens.store(new_tokens, Ordering::Relaxed);
                *last = Instant::now();
            }

            // Try to consume
            let current = entry.tokens.load(Ordering::Relaxed);
            if current >= bytes {
                entry.tokens.fetch_sub(bytes, Ordering::Relaxed);
                true
            } else {
                false
            }
        } else {
            // Unknown connections are not throttled
            true
        }
    }

    /// Unregister a connection, freeing its bandwidth state.
    pub fn unregister(&self, conn_id: &ConnectionId) {
        if self.connections.remove(conn_id.as_str()).is_some() {
            debug!(conn_id = %conn_id, "bandwidth: unregistered connection");
        }
    }

    /// Get the remaining token count for a connection.
    ///
    /// Returns `0` if the connection is not registered.
    pub fn remaining(&self, conn_id: &ConnectionId) -> u64 {
        self.connections
            .get(conn_id.as_str())
            .map(|e| e.tokens.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Number of connections currently tracked.
    pub fn tracked_count(&self) -> usize {
        self.connections.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_id() -> ConnectionId {
        ConnectionId::new()
    }

    #[test]
    fn register_and_consume() {
        let throttle = BandwidthThrottle::new(1024, 1024);
        let id = make_id();
        throttle.register(&id);

        assert!(throttle.try_consume(&id, 512));
        assert_eq!(throttle.remaining(&id), 512);
    }

    #[test]
    fn exceed_limit_returns_false() {
        let throttle = BandwidthThrottle::new(1024, 1024);
        let id = make_id();
        throttle.register(&id);

        // Consume all tokens
        assert!(throttle.try_consume(&id, 1024));
        // Now bucket is empty
        assert!(!throttle.try_consume(&id, 1));
    }

    #[test]
    fn refill_after_time() {
        let throttle = BandwidthThrottle::new(10_000, 10_000);
        let id = make_id();
        throttle.register(&id);

        // Drain the bucket
        assert!(throttle.try_consume(&id, 10_000));
        assert!(!throttle.try_consume(&id, 1));

        // Manually adjust last_refill to simulate elapsed time
        if let Some(entry) = throttle.connections.get(id.as_str()) {
            let mut last = entry.last_refill.lock().unwrap();
            *last = Instant::now() - std::time::Duration::from_secs(1);
        }

        // After 1 second at 10,000 bytes/sec we should have ~10,000 tokens
        assert!(throttle.try_consume(&id, 5_000));
    }

    #[test]
    fn unregister_removes_connection() {
        let throttle = BandwidthThrottle::new(1024, 1024);
        let id = make_id();
        throttle.register(&id);
        assert_eq!(throttle.tracked_count(), 1);

        throttle.unregister(&id);
        assert_eq!(throttle.tracked_count(), 0);
        assert_eq!(throttle.remaining(&id), 0);
    }

    #[test]
    fn unknown_connection_allowed() {
        let throttle = BandwidthThrottle::new(1024, 1024);
        let id = make_id();
        // Not registered — should still return true
        assert!(throttle.try_consume(&id, 999_999));
    }
}
