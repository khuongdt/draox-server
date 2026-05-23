//! Network-level Prometheus-compatible metrics.
//!
//! All counters use `Ordering::Relaxed` because we only need eventual
//! consistency — we do not synchronise other memory operations with these
//! increments.

use serde::Serialize;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

// ─── Snapshot ─────────────────────────────────────────────────────────────────

/// Point-in-time snapshot of all counters.  Serialisable to JSON.
#[derive(Debug, Clone, Serialize)]
pub struct NetworkMetricsSnapshot {
    pub tcp_connections_total: u64,
    pub tcp_bytes_received: u64,
    pub tcp_bytes_sent: u64,
    pub udp_packets_received: u64,
    pub udp_packets_sent: u64,
    pub ws_messages_received: u64,
    pub ws_messages_sent: u64,
    pub http_requests_total: u64,
    pub active_connections: i64,
}

// ─── Metrics ──────────────────────────────────────────────────────────────────

/// Shared, lock-free network-level metrics store.
///
/// Designed to be wrapped in an `Arc` and shared across tasks.  All methods
/// take `&self` (no locking required).
pub struct NetworkMetrics {
    /// Total number of TCP connections accepted since startup.
    tcp_connections_total: AtomicU64,
    /// Total bytes received over all TCP connections.
    tcp_bytes_received: AtomicU64,
    /// Total bytes sent over all TCP connections.
    tcp_bytes_sent: AtomicU64,
    /// Total UDP packets received.
    udp_packets_received: AtomicU64,
    /// Total UDP packets sent.
    udp_packets_sent: AtomicU64,
    /// Total WebSocket messages received (text + binary).
    ws_messages_received: AtomicU64,
    /// Total WebSocket messages sent.
    ws_messages_sent: AtomicU64,
    /// Total HTTP requests handled.
    http_requests_total: AtomicU64,
    /// Current number of open connections (can go negative on bugs — use i64).
    active_connections: AtomicI64,
}

impl NetworkMetrics {
    pub fn new() -> Self {
        Self {
            tcp_connections_total: AtomicU64::new(0),
            tcp_bytes_received: AtomicU64::new(0),
            tcp_bytes_sent: AtomicU64::new(0),
            udp_packets_received: AtomicU64::new(0),
            udp_packets_sent: AtomicU64::new(0),
            ws_messages_received: AtomicU64::new(0),
            ws_messages_sent: AtomicU64::new(0),
            http_requests_total: AtomicU64::new(0),
            active_connections: AtomicI64::new(0),
        }
    }

    // ── TCP ──────────────────────────────────────────────────────────────────

    /// Increment the total TCP connection counter and the active-connection
    /// gauge.
    pub fn record_tcp_connect(&self) {
        self.tcp_connections_total.fetch_add(1, Ordering::Relaxed);
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement the active-connection gauge when a TCP connection closes.
    pub fn record_tcp_disconnect(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn record_tcp_bytes_in(&self, bytes: u64) {
        self.tcp_bytes_received.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn record_tcp_bytes_out(&self, bytes: u64) {
        self.tcp_bytes_sent.fetch_add(bytes, Ordering::Relaxed);
    }

    // ── UDP ──────────────────────────────────────────────────────────────────

    pub fn record_udp_packet_in(&self) {
        self.udp_packets_received.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_udp_packet_out(&self) {
        self.udp_packets_sent.fetch_add(1, Ordering::Relaxed);
    }

    // ── WebSocket ─────────────────────────────────────────────────────────────

    pub fn record_ws_message_in(&self) {
        self.ws_messages_received.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_ws_message_out(&self) {
        self.ws_messages_sent.fetch_add(1, Ordering::Relaxed);
    }

    // ── HTTP ─────────────────────────────────────────────────────────────────

    pub fn record_http_request(&self) {
        self.http_requests_total.fetch_add(1, Ordering::Relaxed);
    }

    // ── Snapshot & Export ─────────────────────────────────────────────────────

    /// Return a point-in-time copy of all counters.
    pub fn snapshot(&self) -> NetworkMetricsSnapshot {
        NetworkMetricsSnapshot {
            tcp_connections_total: self.tcp_connections_total.load(Ordering::Relaxed),
            tcp_bytes_received: self.tcp_bytes_received.load(Ordering::Relaxed),
            tcp_bytes_sent: self.tcp_bytes_sent.load(Ordering::Relaxed),
            udp_packets_received: self.udp_packets_received.load(Ordering::Relaxed),
            udp_packets_sent: self.udp_packets_sent.load(Ordering::Relaxed),
            ws_messages_received: self.ws_messages_received.load(Ordering::Relaxed),
            ws_messages_sent: self.ws_messages_sent.load(Ordering::Relaxed),
            http_requests_total: self.http_requests_total.load(Ordering::Relaxed),
            active_connections: self.active_connections.load(Ordering::Relaxed),
        }
    }

    /// Render all metrics in the Prometheus text exposition format.
    ///
    /// Each metric is emitted as:
    /// ```text
    /// # HELP <name> <description>
    /// # TYPE <name> <type>
    /// <name> <value>
    /// ```
    pub fn to_prometheus(&self) -> String {
        let s = self.snapshot();
        let mut out = String::with_capacity(512);

        macro_rules! gauge {
            ($name:expr, $help:expr, $value:expr) => {
                out.push_str(&format!(
                    "# HELP {name} {help}\n# TYPE {name} gauge\n{name} {value}\n",
                    name = $name,
                    help = $help,
                    value = $value
                ));
            };
        }
        macro_rules! counter {
            ($name:expr, $help:expr, $value:expr) => {
                out.push_str(&format!(
                    "# HELP {name} {help}\n# TYPE {name} counter\n{name} {value}\n",
                    name = $name,
                    help = $help,
                    value = $value
                ));
            };
        }

        counter!(
            "draox_tcp_connections_total",
            "Total TCP connections accepted since startup.",
            s.tcp_connections_total
        );
        counter!(
            "draox_tcp_bytes_received_total",
            "Total bytes received over TCP connections.",
            s.tcp_bytes_received
        );
        counter!(
            "draox_tcp_bytes_sent_total",
            "Total bytes sent over TCP connections.",
            s.tcp_bytes_sent
        );
        counter!(
            "draox_udp_packets_received_total",
            "Total UDP packets received.",
            s.udp_packets_received
        );
        counter!(
            "draox_udp_packets_sent_total",
            "Total UDP packets sent.",
            s.udp_packets_sent
        );
        counter!(
            "draox_ws_messages_received_total",
            "Total WebSocket messages received.",
            s.ws_messages_received
        );
        counter!(
            "draox_ws_messages_sent_total",
            "Total WebSocket messages sent.",
            s.ws_messages_sent
        );
        counter!(
            "draox_http_requests_total",
            "Total HTTP requests handled.",
            s.http_requests_total
        );
        gauge!(
            "draox_active_connections",
            "Current number of open connections.",
            s.active_connections
        );

        out
    }
}

impl Default for NetworkMetrics {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_tcp_connect_increments_counters() {
        let m = NetworkMetrics::new();
        m.record_tcp_connect();
        m.record_tcp_connect();

        let s = m.snapshot();
        assert_eq!(s.tcp_connections_total, 2);
        assert_eq!(s.active_connections, 2);
    }

    #[test]
    fn test_tcp_disconnect_decrements_active() {
        let m = NetworkMetrics::new();
        m.record_tcp_connect();
        m.record_tcp_connect();
        m.record_tcp_disconnect();

        let s = m.snapshot();
        assert_eq!(s.tcp_connections_total, 2); // total never decrements
        assert_eq!(s.active_connections, 1);
    }

    #[test]
    fn test_byte_and_packet_counters() {
        let m = NetworkMetrics::new();
        m.record_tcp_bytes_in(100);
        m.record_tcp_bytes_out(200);
        m.record_udp_packet_in();
        m.record_udp_packet_out();
        m.record_udp_packet_out();
        m.record_ws_message_in();
        m.record_ws_message_out();
        m.record_http_request();

        let s = m.snapshot();
        assert_eq!(s.tcp_bytes_received, 100);
        assert_eq!(s.tcp_bytes_sent, 200);
        assert_eq!(s.udp_packets_received, 1);
        assert_eq!(s.udp_packets_sent, 2);
        assert_eq!(s.ws_messages_received, 1);
        assert_eq!(s.ws_messages_sent, 1);
        assert_eq!(s.http_requests_total, 1);
    }

    #[test]
    fn test_prometheus_output_contains_all_metrics() {
        let m = NetworkMetrics::new();
        m.record_tcp_connect();
        m.record_http_request();

        let output = m.to_prometheus();
        assert!(output.contains("draox_tcp_connections_total 1"));
        assert!(output.contains("draox_http_requests_total 1"));
        assert!(output.contains("draox_active_connections 1"));
        assert!(output.contains("# TYPE draox_tcp_connections_total counter"));
        assert!(output.contains("# TYPE draox_active_connections gauge"));
    }

    #[test]
    fn test_metrics_arc_shared_across_threads() {
        let m = Arc::new(NetworkMetrics::new());
        let m2 = Arc::clone(&m);

        let handle = std::thread::spawn(move || {
            for _ in 0..50 {
                m2.record_tcp_connect();
            }
        });
        for _ in 0..50 {
            m.record_tcp_connect();
        }
        handle.join().unwrap();

        assert_eq!(m.snapshot().tcp_connections_total, 100);
    }
}
