// Audit trail for admin actions — tracks who did what, when, and from where.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

/// A single audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Monotonically increasing sequence ID (tamper-evident).
    pub sequence_id: u64,
    /// When the action occurred.
    pub timestamp: DateTime<Utc>,
    /// Who performed the action (user ID, API key name, etc.).
    pub actor: String,
    /// What action was performed.
    pub action: AuditAction,
    /// What resource was affected.
    pub resource: String,
    /// Additional details (optional, JSON-serializable context).
    pub details: Option<serde_json::Value>,
    /// Source IP address of the request.
    pub source_ip: Option<String>,
    /// Trace ID for correlating with request logs.
    pub trace_id: Option<String>,
}

/// Categories of auditable actions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    // Plugin management
    PluginActivated,
    PluginDeactivated,
    PluginEnabled,
    PluginDisabled,
    PluginInstalled,
    PluginUninstalled,
    // Connection/session management
    ConnectionDisconnected,
    SessionDestroyed,
    SessionDrained,
    // Traffic guard
    IpBanned,
    IpUnbanned,
    WhitelistUpdated,
    BlacklistUpdated,
    // Configuration
    ConfigUpdated,
    ConfigReloaded,
    // Authentication
    LoginSuccess,
    LoginFailed,
    ApiKeyCreated,
    ApiKeyRevoked,
    // General
    Custom(String),
}

/// The audit log store — append-only with sequence IDs.
pub struct AuditLog {
    entries: RwLock<VecDeque<AuditEntry>>,
    max_entries: usize,
    next_sequence: AtomicU64,
}

impl AuditLog {
    /// Create a new `AuditLog` with the given max capacity.
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: RwLock::new(VecDeque::with_capacity(max_entries.min(1024))),
            max_entries,
            next_sequence: AtomicU64::new(1),
        }
    }

    /// Record an audit entry. Returns the assigned sequence ID.
    pub fn record(
        &self,
        actor: impl Into<String>,
        action: AuditAction,
        resource: impl Into<String>,
        details: Option<serde_json::Value>,
        source_ip: Option<String>,
        trace_id: Option<String>,
    ) -> u64 {
        let sequence_id = self.next_sequence.fetch_add(1, Ordering::Relaxed);

        let entry = AuditEntry {
            sequence_id,
            timestamp: Utc::now(),
            actor: actor.into(),
            action,
            resource: resource.into(),
            details,
            source_ip,
            trace_id,
        };

        let mut entries = self.entries.write().unwrap();
        entries.push_back(entry);
        if entries.len() > self.max_entries {
            entries.pop_front();
        }

        sequence_id
    }

    /// Get all entries (newest first).
    pub fn entries(&self) -> Vec<AuditEntry> {
        let entries = self.entries.read().unwrap();
        let mut result: Vec<AuditEntry> = entries.iter().cloned().collect();
        result.reverse();
        result
    }

    /// Get entry by sequence ID.
    pub fn get_by_id(&self, sequence_id: u64) -> Option<AuditEntry> {
        let entries = self.entries.read().unwrap();
        entries
            .iter()
            .find(|e| e.sequence_id == sequence_id)
            .cloned()
    }

    /// Query entries by action type.
    pub fn query_by_action(&self, action: &AuditAction) -> Vec<AuditEntry> {
        let entries = self.entries.read().unwrap();
        entries
            .iter()
            .filter(|e| &e.action == action)
            .cloned()
            .collect()
    }

    /// Query entries by actor.
    pub fn query_by_actor(&self, actor: &str) -> Vec<AuditEntry> {
        let entries = self.entries.read().unwrap();
        entries
            .iter()
            .filter(|e| e.actor == actor)
            .cloned()
            .collect()
    }

    /// Query entries within a time range (inclusive).
    pub fn query_by_time_range(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Vec<AuditEntry> {
        let entries = self.entries.read().unwrap();
        entries
            .iter()
            .filter(|e| e.timestamp >= from && e.timestamp <= to)
            .cloned()
            .collect()
    }

    /// Total number of entries currently stored.
    pub fn len(&self) -> usize {
        self.entries.read().unwrap().len()
    }

    /// Whether the log is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.read().unwrap().is_empty()
    }

    /// Verify sequence integrity (no gaps, monotonically increasing).
    pub fn verify_integrity(&self) -> bool {
        let entries = self.entries.read().unwrap();
        if entries.len() <= 1 {
            return true;
        }
        entries
            .iter()
            .zip(entries.iter().skip(1))
            .all(|(prev, next)| next.sequence_id == prev.sequence_id + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_audit_entry() {
        let log = AuditLog::new(100);
        let id = log.record(
            "admin",
            AuditAction::PluginInstalled,
            "com.example.plugin",
            Some(serde_json::json!({"version": "1.0"})),
            Some("127.0.0.1".to_string()),
            Some("trace-abc".to_string()),
        );

        assert_eq!(id, 1);
        assert_eq!(log.len(), 1);

        let entry = log.get_by_id(1).expect("entry should exist");
        assert_eq!(entry.actor, "admin");
        assert_eq!(entry.action, AuditAction::PluginInstalled);
        assert_eq!(entry.resource, "com.example.plugin");
        assert_eq!(entry.source_ip.as_deref(), Some("127.0.0.1"));
        assert_eq!(entry.trace_id.as_deref(), Some("trace-abc"));
    }

    #[test]
    fn test_sequence_ids_monotonic() {
        let log = AuditLog::new(100);
        for i in 0..10 {
            let id = log.record(
                format!("actor-{i}"),
                AuditAction::ConfigUpdated,
                "config",
                None,
                None,
                None,
            );
            assert_eq!(id, (i + 1) as u64);
        }

        let entries = log.entries();
        // entries() returns newest first
        for (i, entry) in entries.iter().rev().enumerate() {
            assert_eq!(entry.sequence_id, (i + 1) as u64);
        }
    }

    #[test]
    fn test_max_entries_eviction() {
        let log = AuditLog::new(5);
        for _ in 0..10 {
            log.record("admin", AuditAction::ConfigUpdated, "cfg", None, None, None);
        }

        assert_eq!(log.len(), 5);

        let entries = log.entries();
        // Newest first: sequence IDs 10, 9, 8, 7, 6
        assert_eq!(entries[0].sequence_id, 10);
        assert_eq!(entries[4].sequence_id, 6);
    }

    #[test]
    fn test_query_by_action() {
        let log = AuditLog::new(100);
        log.record("admin", AuditAction::PluginInstalled, "p1", None, None, None);
        log.record("admin", AuditAction::ConfigUpdated, "cfg", None, None, None);
        log.record("admin", AuditAction::PluginInstalled, "p2", None, None, None);
        log.record("admin", AuditAction::IpBanned, "10.0.0.1", None, None, None);

        let installed = log.query_by_action(&AuditAction::PluginInstalled);
        assert_eq!(installed.len(), 2);
        assert!(installed.iter().all(|e| e.action == AuditAction::PluginInstalled));
    }

    #[test]
    fn test_query_by_actor() {
        let log = AuditLog::new(100);
        log.record("alice", AuditAction::LoginSuccess, "auth", None, None, None);
        log.record("bob", AuditAction::LoginSuccess, "auth", None, None, None);
        log.record("alice", AuditAction::ConfigUpdated, "cfg", None, None, None);
        log.record("charlie", AuditAction::LoginFailed, "auth", None, None, None);

        let alice_entries = log.query_by_actor("alice");
        assert_eq!(alice_entries.len(), 2);
        assert!(alice_entries.iter().all(|e| e.actor == "alice"));
    }

    #[test]
    fn test_verify_integrity() {
        let log = AuditLog::new(100);
        for _ in 0..5 {
            log.record("admin", AuditAction::ConfigUpdated, "cfg", None, None, None);
        }

        assert!(log.verify_integrity());
    }

    #[test]
    fn test_get_by_id() {
        let log = AuditLog::new(100);
        log.record("admin", AuditAction::IpBanned, "10.0.0.1", None, None, None);
        log.record("admin", AuditAction::IpUnbanned, "10.0.0.1", None, None, None);
        log.record("admin", AuditAction::ConfigReloaded, "cfg", None, None, None);

        let entry = log.get_by_id(2).expect("entry 2 should exist");
        assert_eq!(entry.action, AuditAction::IpUnbanned);

        assert!(log.get_by_id(999).is_none());
    }

    #[test]
    fn test_query_by_time_range() {
        let log = AuditLog::new(100);
        // Record a few entries — they all get timestamps close to now
        for _ in 0..3 {
            log.record("admin", AuditAction::ConfigUpdated, "cfg", None, None, None);
        }

        let from = Utc::now() - chrono::Duration::seconds(10);
        let to = Utc::now() + chrono::Duration::seconds(10);
        let results = log.query_by_time_range(from, to);
        assert_eq!(results.len(), 3);

        // Query a range in the past — should return nothing
        let past_from = Utc::now() - chrono::Duration::hours(2);
        let past_to = Utc::now() - chrono::Duration::hours(1);
        let empty = log.query_by_time_range(past_from, past_to);
        assert!(empty.is_empty());
    }
}
