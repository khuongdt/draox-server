use crate::query::LogFilter;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use server_core::event::{EventBus, ServerEvent};
use server_core::types::ShutdownReceiver;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::{debug, warn};

// ────────────────────────────────────────────────────────
// LogEntry
// ────────────────────────────────────────────────────────

/// A single activity log entry representing a server event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: u64,
    pub timestamp: DateTime<Utc>,
    /// High-level category: "connection", "session", "guard", "plugin", "server", "custom".
    pub category: String,
    /// Specific event type within the category (e.g., "accepted", "closed", "blocked").
    pub event_type: String,
    /// Arbitrary JSON payload with event-specific details.
    pub details: serde_json::Value,
}

// ────────────────────────────────────────────────────────
// ActivityLog
// ────────────────────────────────────────────────────────

/// In-memory, thread-safe activity log backed by a DashMap.
///
/// Entries are assigned monotonically increasing IDs. When the number of
/// entries exceeds `max_entries`, the oldest entries are evicted.
pub struct ActivityLog {
    entries: DashMap<u64, LogEntry>,
    next_id: AtomicU64,
    /// Tracks the smallest ID that is still retained.
    min_id: AtomicU64,
    max_entries: usize,
}

impl ActivityLog {
    /// Create a new ActivityLog that retains at most `max_entries` entries.
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: DashMap::new(),
            next_id: AtomicU64::new(0),
            min_id: AtomicU64::new(0),
            max_entries,
        }
    }

    /// Record a new log entry. Returns the assigned entry ID.
    ///
    /// If the log is at capacity, the oldest entry is evicted first.
    pub fn record(
        &self,
        category: &str,
        event_type: &str,
        details: serde_json::Value,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let entry = LogEntry {
            id,
            timestamp: Utc::now(),
            category: category.to_string(),
            event_type: event_type.to_string(),
            details,
        };

        self.entries.insert(id, entry);

        // Evict the oldest entry if we exceeded capacity.
        self.evict_if_needed();

        id
    }

    /// Retrieve a single entry by ID.
    pub fn get(&self, id: u64) -> Option<LogEntry> {
        self.entries.get(&id).map(|r| r.value().clone())
    }

    /// Query entries that match the given filter.
    ///
    /// Results are sorted by ID ascending (oldest first), then truncated
    /// to the requested limit.
    pub fn query(&self, filter: &LogFilter) -> Vec<LogEntry> {
        let mut results: Vec<LogEntry> = self
            .entries
            .iter()
            .filter(|r| {
                let entry = r.value();
                if let Some(ref cat) = filter.category {
                    if entry.category != *cat {
                        return false;
                    }
                }
                if let Some(ref et) = filter.event_type {
                    if entry.event_type != *et {
                        return false;
                    }
                }
                if let Some(from) = filter.from {
                    if entry.timestamp < from {
                        return false;
                    }
                }
                if let Some(to) = filter.to {
                    if entry.timestamp > to {
                        return false;
                    }
                }
                true
            })
            .map(|r| r.value().clone())
            .collect();

        // Sort by ID (ascending) so results are chronological.
        results.sort_by_key(|e| e.id);

        if let Some(limit) = filter.limit {
            results.truncate(limit);
        }

        results
    }

    /// Return the number of entries currently stored.
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// Spawn a background task that listens to the EventBus, converts
    /// each ServerEvent into a LogEntry, and records it.
    ///
    /// The task runs until the shutdown signal is received or the
    /// event bus sender is dropped.
    pub fn start_event_listener(
        self: Arc<Self>,
        event_bus: Arc<EventBus>,
        mut shutdown: ShutdownReceiver,
    ) {
        tokio::spawn(async move {
            let mut rx = event_bus.subscribe_all();

            loop {
                tokio::select! {
                    result = rx.recv() => {
                        match result {
                            Ok(event) => {
                                let (category, event_type, details) =
                                    server_event_to_log_parts(&event);
                                self.record(&category, &event_type, details);
                                debug!(
                                    category = %category,
                                    event_type = %event_type,
                                    "recorded event"
                                );
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                warn!(missed = n, "activity-log listener lagged, missed events");
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                debug!("event bus closed, stopping activity-log listener");
                                break;
                            }
                        }
                    }
                    _ = shutdown.recv() => {
                        debug!("shutdown received, stopping activity-log listener");
                        break;
                    }
                }
            }
        });
    }

    // ── internal ──────────────────────────────────────────

    /// Evict the oldest entry when the map exceeds max_entries.
    fn evict_if_needed(&self) {
        while self.entries.len() > self.max_entries {
            let old_id = self.min_id.fetch_add(1, Ordering::Relaxed);
            self.entries.remove(&old_id);
        }
    }
}

// ────────────────────────────────────────────────────────
// ServerEvent → (category, event_type, details) conversion
// ────────────────────────────────────────────────────────

fn server_event_to_log_parts(event: &ServerEvent) -> (String, String, serde_json::Value) {
    match event {
        // Connection events
        ServerEvent::ConnectionAccepted {
            connection_id,
            protocol,
            remote_addr,
        } => (
            "connection".into(),
            "accepted".into(),
            serde_json::json!({
                "connection_id": connection_id.as_str(),
                "protocol": protocol.to_string(),
                "remote_addr": remote_addr,
            }),
        ),
        ServerEvent::ConnectionClosed {
            connection_id,
            reason,
        } => (
            "connection".into(),
            "closed".into(),
            serde_json::json!({
                "connection_id": connection_id.as_str(),
                "reason": reason,
            }),
        ),
        ServerEvent::ConnectionError {
            connection_id,
            error,
        } => (
            "connection".into(),
            "error".into(),
            serde_json::json!({
                "connection_id": connection_id.as_str(),
                "error": error,
            }),
        ),

        // Session events
        ServerEvent::SessionCreated { session_id } => (
            "session".into(),
            "created".into(),
            serde_json::json!({
                "session_id": session_id.as_str(),
            }),
        ),
        ServerEvent::SessionDestroyed { session_id, reason } => (
            "session".into(),
            "destroyed".into(),
            serde_json::json!({
                "session_id": session_id.as_str(),
                "reason": reason,
            }),
        ),

        // Traffic guard events
        ServerEvent::GuardConnectionBlocked {
            remote_addr,
            reason,
        } => (
            "guard".into(),
            "blocked".into(),
            serde_json::json!({
                "remote_addr": remote_addr,
                "reason": reason,
            }),
        ),
        ServerEvent::GuardIpBanned { ip, duration_secs } => (
            "guard".into(),
            "banned".into(),
            serde_json::json!({
                "ip": ip.to_string(),
                "duration_secs": duration_secs,
            }),
        ),
        ServerEvent::GuardIpUnbanned { ip } => (
            "guard".into(),
            "unbanned".into(),
            serde_json::json!({
                "ip": ip.to_string(),
            }),
        ),
        ServerEvent::GuardAttackDetected {
            attack_type,
            source,
        } => (
            "guard".into(),
            "attack_detected".into(),
            serde_json::json!({
                "attack_type": attack_type,
                "source": source,
            }),
        ),
        ServerEvent::GuardThresholdAdjusted {
            metric,
            old_value,
            new_value,
        } => (
            "guard".into(),
            "threshold_adjusted".into(),
            serde_json::json!({
                "metric": metric,
                "old_value": old_value,
                "new_value": new_value,
            }),
        ),

        // Plugin events
        ServerEvent::PluginActivated { plugin_id } => (
            "plugin".into(),
            "activated".into(),
            serde_json::json!({
                "plugin_id": plugin_id.as_str(),
            }),
        ),
        ServerEvent::PluginDeactivated { plugin_id } => (
            "plugin".into(),
            "deactivated".into(),
            serde_json::json!({
                "plugin_id": plugin_id.as_str(),
            }),
        ),
        ServerEvent::PluginEnabled { plugin_id } => (
            "plugin".into(),
            "enabled".into(),
            serde_json::json!({
                "plugin_id": plugin_id.as_str(),
            }),
        ),
        ServerEvent::PluginDisabled { plugin_id } => (
            "plugin".into(),
            "disabled".into(),
            serde_json::json!({
                "plugin_id": plugin_id.as_str(),
            }),
        ),
        ServerEvent::PluginError { plugin_id, error } => (
            "plugin".into(),
            "error".into(),
            serde_json::json!({
                "plugin_id": plugin_id.as_str(),
                "error": error,
            }),
        ),

        // Server lifecycle events
        ServerEvent::ServerStarted { timestamp } => (
            "server".into(),
            "started".into(),
            serde_json::json!({
                "timestamp": timestamp.to_rfc3339(),
            }),
        ),
        ServerEvent::ServerShuttingDown { reason } => (
            "server".into(),
            "shutting_down".into(),
            serde_json::json!({
                "reason": reason,
            }),
        ),

        // Custom plugin events
        ServerEvent::Custom {
            source,
            name,
            payload,
        } => (
            format!("custom.{source}"),
            name.clone(),
            payload.clone(),
        ),
    }
}

// ────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_record_and_get() {
        let log = ActivityLog::new(100);

        let id = log.record(
            "connection",
            "accepted",
            serde_json::json!({"addr": "127.0.0.1"}),
        );

        let entry = log.get(id).expect("entry should exist");
        assert_eq!(entry.id, id);
        assert_eq!(entry.category, "connection");
        assert_eq!(entry.event_type, "accepted");
        assert_eq!(entry.details["addr"], "127.0.0.1");
    }

    #[test]
    fn test_record_over_max_entries() {
        let max = 5;
        let log = ActivityLog::new(max);

        // Record more entries than max_entries allows.
        for i in 0..10 {
            log.record(
                "test",
                "event",
                serde_json::json!({"index": i}),
            );
        }

        // Should have at most max_entries.
        assert!(
            log.count() <= max,
            "count {} should be <= max {}",
            log.count(),
            max
        );

        // Oldest entries (IDs 0..5) should have been evicted.
        assert!(log.get(0).is_none(), "entry 0 should be evicted");
        assert!(log.get(4).is_none(), "entry 4 should be evicted");

        // Newest entries should still be present.
        assert!(log.get(9).is_some(), "entry 9 should exist");
    }

    #[test]
    fn test_query_by_category() {
        let log = ActivityLog::new(100);

        log.record("connection", "accepted", serde_json::json!({}));
        log.record("session", "created", serde_json::json!({}));
        log.record("connection", "closed", serde_json::json!({}));
        log.record("guard", "blocked", serde_json::json!({}));

        let filter = LogFilter {
            category: Some("connection".into()),
            ..Default::default()
        };

        let results = log.query(&filter);
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|e| e.category == "connection"));
    }

    #[test]
    fn test_query_by_time_range() {
        let log = ActivityLog::new(100);

        // Record three entries — they will all have timestamps very close together,
        // so we test with a range that covers "now".
        let before = Utc::now() - Duration::seconds(1);
        log.record("connection", "accepted", serde_json::json!({}));
        log.record("session", "created", serde_json::json!({}));
        log.record("guard", "blocked", serde_json::json!({}));
        let after = Utc::now() + Duration::seconds(1);

        // All entries should be within the range.
        let filter = LogFilter {
            from: Some(before),
            to: Some(after),
            ..Default::default()
        };
        let results = log.query(&filter);
        assert_eq!(results.len(), 3);

        // No entries should be in a future-only range.
        let future_filter = LogFilter {
            from: Some(Utc::now() + Duration::hours(1)),
            ..Default::default()
        };
        let future_results = log.query(&future_filter);
        assert_eq!(future_results.len(), 0);
    }

    #[test]
    fn test_query_with_limit() {
        let log = ActivityLog::new(100);

        for _ in 0..10 {
            log.record("connection", "accepted", serde_json::json!({}));
        }

        let filter = LogFilter {
            limit: Some(3),
            ..Default::default()
        };

        let results = log.query(&filter);
        assert_eq!(results.len(), 3);
        // Since sorted by ID ascending, the first three entries should be returned.
        assert_eq!(results[0].id, 0);
        assert_eq!(results[1].id, 1);
        assert_eq!(results[2].id, 2);
    }
}
