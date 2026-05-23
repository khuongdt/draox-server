use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use server_core::ClientId;
use tracing::debug;

/// User presence status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresenceStatus {
    Online,
    Away,
    DoNotDisturb,
    Offline,
}

impl Default for PresenceStatus {
    fn default() -> Self {
        PresenceStatus::Offline
    }
}

/// Presence information for a user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceInfo {
    pub status: PresenceStatus,
    pub status_message: String,
    pub last_seen: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Tracks user presence status.
pub struct PresenceTracker {
    presences: DashMap<String, PresenceInfo>,
}

impl PresenceTracker {
    pub fn new() -> Self {
        Self {
            presences: DashMap::new(),
        }
    }

    /// Set a user's presence status.
    pub fn set_status(&self, client_id: &ClientId, status: PresenceStatus) {
        let key = client_id.as_str().to_string();
        let now = Utc::now();
        let mut entry = self.presences.entry(key.clone()).or_insert_with(|| PresenceInfo {
            status: PresenceStatus::Offline,
            status_message: String::new(),
            last_seen: now,
            updated_at: now,
        });
        entry.status = status;
        entry.updated_at = now;
        if status != PresenceStatus::Offline {
            entry.last_seen = now;
        }
        debug!(client_id = %key, ?status, "presence status updated");
    }

    /// Set a user's status message.
    pub fn set_status_message(&self, client_id: &ClientId, message: String) {
        let key = client_id.as_str().to_string();
        let now = Utc::now();
        let mut entry = self.presences.entry(key.clone()).or_insert_with(|| PresenceInfo {
            status: PresenceStatus::Offline,
            status_message: String::new(),
            last_seen: now,
            updated_at: now,
        });
        entry.status_message = message;
        entry.updated_at = now;
        debug!(client_id = %key, "status message updated");
    }

    /// Get a user's presence info.
    pub fn get(&self, client_id: &ClientId) -> Option<PresenceInfo> {
        self.presences
            .get(client_id.as_str())
            .map(|r| r.value().clone())
    }

    /// Get presence status (defaults to Offline for unknown users).
    pub fn get_status(&self, client_id: &ClientId) -> PresenceStatus {
        self.presences
            .get(client_id.as_str())
            .map(|r| r.value().status)
            .unwrap_or(PresenceStatus::Offline)
    }

    /// Mark user as online with last_seen = now.
    pub fn mark_online(&self, client_id: &ClientId) {
        self.set_status(client_id, PresenceStatus::Online);
    }

    /// Mark user as offline.
    pub fn mark_offline(&self, client_id: &ClientId) {
        self.set_status(client_id, PresenceStatus::Offline);
    }

    /// Get all online users.
    pub fn online_users(&self) -> Vec<String> {
        self.presences
            .iter()
            .filter(|r| r.value().status == PresenceStatus::Online)
            .map(|r| r.key().clone())
            .collect()
    }

    /// Get all users with their presence info.
    pub fn all_presences(&self) -> Vec<(String, PresenceInfo)> {
        self.presences
            .iter()
            .map(|r| (r.key().clone(), r.value().clone()))
            .collect()
    }

    /// Total tracked users.
    pub fn tracked_count(&self) -> usize {
        self.presences.len()
    }
}

impl Default for PresenceTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_get_status() {
        let tracker = PresenceTracker::new();
        let alice = ClientId::from_str("cli_alice");

        assert_eq!(tracker.get_status(&alice), PresenceStatus::Offline);

        tracker.set_status(&alice, PresenceStatus::Online);
        assert_eq!(tracker.get_status(&alice), PresenceStatus::Online);

        tracker.set_status(&alice, PresenceStatus::Away);
        assert_eq!(tracker.get_status(&alice), PresenceStatus::Away);

        tracker.set_status(&alice, PresenceStatus::DoNotDisturb);
        assert_eq!(tracker.get_status(&alice), PresenceStatus::DoNotDisturb);
    }

    #[test]
    fn test_mark_online_offline() {
        let tracker = PresenceTracker::new();
        let alice = ClientId::from_str("cli_alice");

        tracker.mark_online(&alice);
        assert_eq!(tracker.get_status(&alice), PresenceStatus::Online);
        assert!(tracker.get(&alice).is_some());

        tracker.mark_offline(&alice);
        assert_eq!(tracker.get_status(&alice), PresenceStatus::Offline);
    }

    #[test]
    fn test_online_users_filtering() {
        let tracker = PresenceTracker::new();
        let alice = ClientId::from_str("cli_alice");
        let bob = ClientId::from_str("cli_bob");
        let charlie = ClientId::from_str("cli_charlie");

        tracker.mark_online(&alice);
        tracker.mark_online(&bob);
        tracker.set_status(&charlie, PresenceStatus::Away);

        let online = tracker.online_users();
        assert_eq!(online.len(), 2);
        assert!(online.contains(&"cli_alice".to_string()));
        assert!(online.contains(&"cli_bob".to_string()));
        assert!(!online.contains(&"cli_charlie".to_string()));
    }

    #[test]
    fn test_status_message() {
        let tracker = PresenceTracker::new();
        let alice = ClientId::from_str("cli_alice");

        tracker.set_status_message(&alice, "In a meeting".to_string());
        let info = tracker.get(&alice).unwrap();
        assert_eq!(info.status_message, "In a meeting");
        assert_eq!(info.status, PresenceStatus::Offline);

        tracker.mark_online(&alice);
        let info = tracker.get(&alice).unwrap();
        assert_eq!(info.status, PresenceStatus::Online);
        assert_eq!(info.status_message, "In a meeting");
    }

    #[test]
    fn test_default_is_offline() {
        let tracker = PresenceTracker::new();
        let unknown = ClientId::from_str("cli_unknown");

        assert_eq!(tracker.get_status(&unknown), PresenceStatus::Offline);
        assert!(tracker.get(&unknown).is_none());
        assert_eq!(tracker.tracked_count(), 0);

        let default_status = PresenceStatus::default();
        assert_eq!(default_status, PresenceStatus::Offline);
    }
}
