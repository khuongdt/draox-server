use dashmap::DashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use tracing::{debug, warn};

/// Per-connection backpressure state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackpressureState {
    /// Normal operation -- writes are flowing.
    Normal,
    /// High watermark reached -- reads should be paused.
    Paused,
}

/// Internal per-connection backpressure tracking.
struct BackpressureEntry {
    buffer_size: AtomicUsize,
    state: std::sync::RwLock<BackpressureState>,
}

/// Manages write-buffer backpressure for connections.
///
/// When a connection's write buffer exceeds the **high watermark**, the
/// manager signals that reads should be paused (to stop accepting more
/// data). When the buffer drains below the **low watermark**, reads
/// resume. This prevents unbounded memory growth when a slow consumer
/// cannot keep up with a fast producer.
pub struct BackpressureManager {
    connections: DashMap<String, BackpressureEntry>,
    high_watermark: usize,
    low_watermark: usize,
}

impl BackpressureManager {
    /// Create a new manager with the given watermarks.
    ///
    /// `high_watermark` is the buffer size at which reads are paused.
    /// `low_watermark` is the buffer size at which reads resume.
    pub fn new(high_watermark: usize, low_watermark: usize) -> Self {
        Self {
            connections: DashMap::new(),
            high_watermark,
            low_watermark,
        }
    }

    /// Register a connection for backpressure tracking.
    pub fn register(&self, conn_id: &str) {
        self.connections.insert(
            conn_id.to_owned(),
            BackpressureEntry {
                buffer_size: AtomicUsize::new(0),
                state: std::sync::RwLock::new(BackpressureState::Normal),
            },
        );
        debug!(conn_id = %conn_id, "backpressure: registered connection");
    }

    /// Unregister a connection, freeing its tracking state.
    pub fn unregister(&self, conn_id: &str) {
        if self.connections.remove(conn_id).is_some() {
            debug!(conn_id = %conn_id, "backpressure: unregistered connection");
        }
    }

    /// Record bytes written to the connection's buffer.
    ///
    /// Returns the new [`BackpressureState`]. When the buffer crosses the
    /// high watermark the state transitions to `Paused`.
    pub fn record_write(&self, conn_id: &str, bytes: usize) -> BackpressureState {
        if let Some(entry) = self.connections.get(conn_id) {
            let new_size = entry.buffer_size.fetch_add(bytes, Ordering::Relaxed) + bytes;
            if new_size >= self.high_watermark {
                let mut state = entry.state.write().unwrap();
                if *state == BackpressureState::Normal {
                    *state = BackpressureState::Paused;
                    warn!(
                        conn_id = %conn_id,
                        buffer_size = new_size,
                        "backpressure: pausing reads"
                    );
                }
                BackpressureState::Paused
            } else {
                *entry.state.read().unwrap()
            }
        } else {
            BackpressureState::Normal
        }
    }

    /// Record bytes drained (sent) from the connection's buffer.
    ///
    /// Returns the new [`BackpressureState`]. When the buffer drops to or
    /// below the low watermark the state transitions back to `Normal`.
    pub fn record_drain(&self, conn_id: &str, bytes: usize) -> BackpressureState {
        if let Some(entry) = self.connections.get(conn_id) {
            let current = entry.buffer_size.load(Ordering::Relaxed);
            let new_size = current.saturating_sub(bytes);
            entry.buffer_size.store(new_size, Ordering::Relaxed);

            if new_size <= self.low_watermark {
                let mut state = entry.state.write().unwrap();
                if *state == BackpressureState::Paused {
                    *state = BackpressureState::Normal;
                    debug!(
                        conn_id = %conn_id,
                        buffer_size = new_size,
                        "backpressure: resuming reads"
                    );
                }
                BackpressureState::Normal
            } else {
                *entry.state.read().unwrap()
            }
        } else {
            BackpressureState::Normal
        }
    }

    /// Get the current backpressure state for a connection.
    ///
    /// Returns `Normal` if the connection is not registered.
    pub fn state(&self, conn_id: &str) -> BackpressureState {
        self.connections
            .get(conn_id)
            .map(|e| *e.state.read().unwrap())
            .unwrap_or(BackpressureState::Normal)
    }

    /// Get the current buffer size for a connection.
    ///
    /// Returns `0` if the connection is not registered.
    pub fn buffer_size(&self, conn_id: &str) -> usize {
        self.connections
            .get(conn_id)
            .map(|e| e.buffer_size.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Number of connections currently in the `Paused` state.
    pub fn paused_count(&self) -> usize {
        self.connections
            .iter()
            .filter(|e| *e.value().state.read().unwrap() == BackpressureState::Paused)
            .count()
    }

    /// Total number of tracked connections.
    pub fn tracked_count(&self) -> usize {
        self.connections.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_state_initially() {
        let mgr = BackpressureManager::new(1024, 256);
        mgr.register("conn_1");
        assert_eq!(mgr.state("conn_1"), BackpressureState::Normal);
        assert_eq!(mgr.buffer_size("conn_1"), 0);
    }

    #[test]
    fn write_triggers_pause_at_high_watermark() {
        let mgr = BackpressureManager::new(1000, 200);
        mgr.register("conn_1");

        // Below high watermark
        let state = mgr.record_write("conn_1", 500);
        assert_eq!(state, BackpressureState::Normal);

        // Reach high watermark
        let state = mgr.record_write("conn_1", 500);
        assert_eq!(state, BackpressureState::Paused);
        assert_eq!(mgr.state("conn_1"), BackpressureState::Paused);
        assert_eq!(mgr.paused_count(), 1);
    }

    #[test]
    fn drain_triggers_resume_at_low_watermark() {
        let mgr = BackpressureManager::new(1000, 200);
        mgr.register("conn_1");

        // Push past high watermark
        mgr.record_write("conn_1", 1200);
        assert_eq!(mgr.state("conn_1"), BackpressureState::Paused);

        // Drain partially -- still above low watermark
        let state = mgr.record_drain("conn_1", 500);
        assert_eq!(state, BackpressureState::Paused);
        assert_eq!(mgr.buffer_size("conn_1"), 700);

        // Drain below low watermark
        let state = mgr.record_drain("conn_1", 600);
        assert_eq!(state, BackpressureState::Normal);
        assert_eq!(mgr.buffer_size("conn_1"), 100);
        assert_eq!(mgr.paused_count(), 0);
    }

    #[test]
    fn multiple_connections_independent() {
        let mgr = BackpressureManager::new(1000, 200);
        mgr.register("conn_1");
        mgr.register("conn_2");

        // Pause only conn_1
        mgr.record_write("conn_1", 1500);
        assert_eq!(mgr.state("conn_1"), BackpressureState::Paused);
        assert_eq!(mgr.state("conn_2"), BackpressureState::Normal);
        assert_eq!(mgr.paused_count(), 1);
        assert_eq!(mgr.tracked_count(), 2);
    }

    #[test]
    fn unregistered_returns_normal() {
        let mgr = BackpressureManager::new(1000, 200);
        assert_eq!(mgr.state("unknown"), BackpressureState::Normal);
        assert_eq!(
            mgr.record_write("unknown", 9999),
            BackpressureState::Normal
        );
        assert_eq!(
            mgr.record_drain("unknown", 9999),
            BackpressureState::Normal
        );
    }
}
