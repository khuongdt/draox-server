use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tracing::debug;

/// A read receipt: records that a user has read a specific message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadReceipt {
    pub message_id: String,
    pub reader_id: String,
    pub read_at: DateTime<Utc>,
}

/// Tracks read receipts per message and last-read position per (channel, user).
pub struct ReadReceiptTracker {
    /// message_id -> list of receipts
    receipts: DashMap<String, Vec<ReadReceipt>>,
    /// (channel_id, user_id) -> timestamp of the last message the user has read
    last_read: DashMap<(String, String), DateTime<Utc>>,
}

impl ReadReceiptTracker {
    pub fn new() -> Self {
        Self {
            receipts: DashMap::new(),
            last_read: DashMap::new(),
        }
    }

    /// Record that `reader_id` has read `message_id`. Returns the new receipt.
    /// If a receipt already exists for this (message, reader) pair it is not duplicated.
    pub fn mark_read(&self, message_id: &str, reader_id: &str) -> ReadReceipt {
        let receipt = ReadReceipt {
            message_id: message_id.to_string(),
            reader_id: reader_id.to_string(),
            read_at: Utc::now(),
        };

        let mut entry = self
            .receipts
            .entry(message_id.to_string())
            .or_default();

        if !entry.iter().any(|r| r.reader_id == reader_id) {
            entry.push(receipt.clone());
            debug!(
                message_id = %message_id,
                reader_id = %reader_id,
                "read receipt recorded"
            );
        }

        receipt
    }

    /// Get all receipts for a message.
    pub fn get_receipts(&self, message_id: &str) -> Vec<ReadReceipt> {
        self.receipts
            .get(message_id)
            .map(|r| r.value().clone())
            .unwrap_or_default()
    }

    /// Check whether `reader_id` has read `message_id`.
    pub fn is_read_by(&self, message_id: &str, reader_id: &str) -> bool {
        self.receipts
            .get(message_id)
            .map(|r| r.iter().any(|receipt| receipt.reader_id == reader_id))
            .unwrap_or(false)
    }

    /// Update the last-read timestamp for a user in a channel to now.
    pub fn mark_channel_read(&self, channel_id: &str, user_id: &str) {
        let key = (channel_id.to_string(), user_id.to_string());
        self.last_read.insert(key, Utc::now());
        debug!(
            channel_id = %channel_id,
            user_id = %user_id,
            "channel marked as read"
        );
    }

    /// Get the timestamp of the last message the user has read in a channel.
    pub fn last_read_time(&self, channel_id: &str, user_id: &str) -> Option<DateTime<Utc>> {
        let key = (channel_id.to_string(), user_id.to_string());
        self.last_read.get(&key).map(|r| *r.value())
    }

    /// Count how many of `messages` (identified by their timestamps) were sent after the
    /// user's last-read time in the channel — i.e., the number of unread messages.
    ///
    /// `messages` is a slice of message timestamps in the channel.
    pub fn unread_count(
        &self,
        channel_id: &str,
        user_id: &str,
        messages: &[DateTime<Utc>],
    ) -> usize {
        match self.last_read_time(channel_id, user_id) {
            Some(last_read) => messages.iter().filter(|&&ts| ts > last_read).count(),
            // No last-read record → all messages are unread.
            None => messages.len(),
        }
    }
}

impl Default for ReadReceiptTracker {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_mark_read_and_get_receipts() {
        let tracker = ReadReceiptTracker::new();

        let receipt = tracker.mark_read("msg_001", "cli_alice");
        assert_eq!(receipt.message_id, "msg_001");
        assert_eq!(receipt.reader_id, "cli_alice");

        let receipts = tracker.get_receipts("msg_001");
        assert_eq!(receipts.len(), 1);
        assert_eq!(receipts[0].reader_id, "cli_alice");
    }

    #[test]
    fn test_no_duplicate_receipts() {
        let tracker = ReadReceiptTracker::new();

        tracker.mark_read("msg_001", "cli_alice");
        tracker.mark_read("msg_001", "cli_alice"); // duplicate call
        tracker.mark_read("msg_001", "cli_bob");

        let receipts = tracker.get_receipts("msg_001");
        assert_eq!(receipts.len(), 2, "alice should appear only once");
        assert!(receipts.iter().any(|r| r.reader_id == "cli_alice"));
        assert!(receipts.iter().any(|r| r.reader_id == "cli_bob"));
    }

    #[test]
    fn test_is_read_by() {
        let tracker = ReadReceiptTracker::new();

        assert!(!tracker.is_read_by("msg_001", "cli_alice"));

        tracker.mark_read("msg_001", "cli_alice");
        assert!(tracker.is_read_by("msg_001", "cli_alice"));
        assert!(!tracker.is_read_by("msg_001", "cli_bob"));
        assert!(!tracker.is_read_by("msg_999", "cli_alice"));
    }

    #[test]
    fn test_mark_channel_read_and_last_read_time() {
        let tracker = ReadReceiptTracker::new();

        assert!(tracker.last_read_time("ch_general", "cli_alice").is_none());

        tracker.mark_channel_read("ch_general", "cli_alice");
        assert!(tracker.last_read_time("ch_general", "cli_alice").is_some());

        // Different channel/user combinations are independent.
        assert!(tracker.last_read_time("ch_other", "cli_alice").is_none());
        assert!(tracker.last_read_time("ch_general", "cli_bob").is_none());
    }

    #[test]
    fn test_unread_count_no_last_read() {
        let tracker = ReadReceiptTracker::new();
        let now = Utc::now();
        let messages = vec![now, now, now];

        // No last-read entry → everything is unread.
        assert_eq!(tracker.unread_count("ch_general", "cli_alice", &messages), 3);
    }

    #[test]
    fn test_unread_count_with_last_read() {
        let tracker = ReadReceiptTracker::new();

        // Simulate: mark as read, then two new messages arrive afterwards.
        tracker.mark_channel_read("ch_general", "cli_alice");
        let last = tracker
            .last_read_time("ch_general", "cli_alice")
            .unwrap();

        // Sleep a tiny bit so that "new" timestamps are strictly after last_read.
        thread::sleep(Duration::from_millis(5));

        let old_msg = last - chrono::Duration::seconds(1); // before last-read
        let new_msg1 = Utc::now();
        let new_msg2 = Utc::now();
        let messages = vec![old_msg, new_msg1, new_msg2];

        assert_eq!(
            tracker.unread_count("ch_general", "cli_alice", &messages),
            2
        );
    }
}
