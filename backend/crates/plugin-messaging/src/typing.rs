use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use server_core::ClientId;
use std::time::Duration;
use tracing::debug;

/// A typing indicator entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypingEntry {
    pub client_id: String,
    pub channel_id: String,
    pub started_at: DateTime<Utc>,
}

/// Tracks typing indicators with auto-expiry.
pub struct TypingTracker {
    /// channel_id -> (client_id -> started_at)
    typing: DashMap<String, DashMap<String, DateTime<Utc>>>,
    /// Auto-expiry timeout
    timeout: Duration,
}

impl TypingTracker {
    pub fn new(timeout_secs: u64) -> Self {
        Self {
            typing: DashMap::new(),
            timeout: Duration::from_secs(timeout_secs),
        }
    }

    /// Start typing in a channel.
    pub fn start_typing(&self, client_id: &ClientId, channel_id: &str) {
        let key = client_id.as_str().to_string();
        let channel_users = self
            .typing
            .entry(channel_id.to_string())
            .or_insert_with(DashMap::new);
        channel_users.insert(key.clone(), Utc::now());
        debug!(client_id = %key, channel_id = %channel_id, "typing started");
    }

    /// Stop typing in a channel.
    pub fn stop_typing(&self, client_id: &ClientId, channel_id: &str) {
        let key = client_id.as_str().to_string();
        if let Some(channel_users) = self.typing.get(channel_id) {
            channel_users.remove(&key);
            debug!(client_id = %key, channel_id = %channel_id, "typing stopped");
        }
    }

    /// Get all currently typing users in a channel (with expired entries filtered out).
    pub fn typing_in_channel(&self, channel_id: &str) -> Vec<String> {
        let now = Utc::now();
        let timeout_chrono = chrono::Duration::from_std(self.timeout)
            .unwrap_or_else(|_| chrono::Duration::seconds(5));

        if let Some(channel_users) = self.typing.get(channel_id) {
            channel_users
                .iter()
                .filter(|entry| {
                    let elapsed = now - *entry.value();
                    elapsed < timeout_chrono
                })
                .map(|entry| entry.key().clone())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Check if a specific user is typing in a channel.
    pub fn is_typing(&self, client_id: &ClientId, channel_id: &str) -> bool {
        let key = client_id.as_str();
        let now = Utc::now();
        let timeout_chrono = chrono::Duration::from_std(self.timeout)
            .unwrap_or_else(|_| chrono::Duration::seconds(5));

        if let Some(channel_users) = self.typing.get(channel_id) {
            if let Some(started_at) = channel_users.get(key) {
                let elapsed = now - *started_at.value();
                return elapsed < timeout_chrono;
            }
        }
        false
    }

    /// Clean up expired typing entries across all channels.
    pub fn cleanup_expired(&self) {
        let now = Utc::now();
        let timeout_chrono = chrono::Duration::from_std(self.timeout)
            .unwrap_or_else(|_| chrono::Duration::seconds(5));

        let mut empty_channels = Vec::new();

        for channel_entry in self.typing.iter() {
            let channel_id = channel_entry.key().clone();
            let channel_users = channel_entry.value();

            // Collect expired keys
            let expired: Vec<String> = channel_users
                .iter()
                .filter(|entry| {
                    let elapsed = now - *entry.value();
                    elapsed >= timeout_chrono
                })
                .map(|entry| entry.key().clone())
                .collect();

            for key in &expired {
                channel_users.remove(key);
            }

            if !expired.is_empty() {
                debug!(
                    channel_id = %channel_id,
                    expired_count = expired.len(),
                    "expired typing entries cleaned up"
                );
            }

            if channel_users.is_empty() {
                empty_channels.push(channel_id);
            }
        }

        // Remove empty channel entries
        for channel_id in empty_channels {
            self.typing.remove(&channel_id);
        }
    }

    /// Total channels with active typing.
    pub fn active_channels(&self) -> usize {
        self.typing.iter().filter(|r| !r.value().is_empty()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_stop_typing() {
        let tracker = TypingTracker::new(10);
        let alice = ClientId::from_str("cli_alice");

        tracker.start_typing(&alice, "ch_general");
        assert!(tracker.is_typing(&alice, "ch_general"));

        tracker.stop_typing(&alice, "ch_general");
        assert!(!tracker.is_typing(&alice, "ch_general"));
    }

    #[test]
    fn test_typing_in_channel_list() {
        let tracker = TypingTracker::new(10);
        let alice = ClientId::from_str("cli_alice");
        let bob = ClientId::from_str("cli_bob");

        tracker.start_typing(&alice, "ch_general");
        tracker.start_typing(&bob, "ch_general");

        let typing = tracker.typing_in_channel("ch_general");
        assert_eq!(typing.len(), 2);
        assert!(typing.contains(&"cli_alice".to_string()));
        assert!(typing.contains(&"cli_bob".to_string()));

        // Non-existent channel returns empty
        let typing = tracker.typing_in_channel("ch_nonexistent");
        assert!(typing.is_empty());
    }

    #[test]
    fn test_expired_entries_filtered() {
        // Use a very short timeout so entries expire immediately
        let tracker = TypingTracker::new(0);
        let alice = ClientId::from_str("cli_alice");

        tracker.start_typing(&alice, "ch_general");

        // With 0-second timeout, the entry should already be expired
        assert!(!tracker.is_typing(&alice, "ch_general"));

        let typing = tracker.typing_in_channel("ch_general");
        assert!(typing.is_empty());

        // Cleanup should remove the expired entry
        tracker.cleanup_expired();
        assert_eq!(tracker.active_channels(), 0);
    }

    #[test]
    fn test_multiple_channels_independent() {
        let tracker = TypingTracker::new(10);
        let alice = ClientId::from_str("cli_alice");
        let bob = ClientId::from_str("cli_bob");

        tracker.start_typing(&alice, "ch_general");
        tracker.start_typing(&bob, "ch_random");

        assert!(tracker.is_typing(&alice, "ch_general"));
        assert!(!tracker.is_typing(&alice, "ch_random"));
        assert!(tracker.is_typing(&bob, "ch_random"));
        assert!(!tracker.is_typing(&bob, "ch_general"));

        assert_eq!(tracker.active_channels(), 2);

        tracker.stop_typing(&alice, "ch_general");
        assert_eq!(tracker.typing_in_channel("ch_general").len(), 0);
        assert_eq!(tracker.typing_in_channel("ch_random").len(), 1);
    }
}
