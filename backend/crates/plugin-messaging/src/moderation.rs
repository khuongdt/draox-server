use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use server_core::ClientId;
use tracing::{debug, warn};

/// Result of content moderation check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModerationAction {
    Allow,
    Block(String),
    Warn(String),
}

/// Mute entry for a user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MuteEntry {
    pub muted_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub reason: String,
}

/// Content moderation engine.
pub struct ContentModerator {
    /// Word blocklist (lowercase)
    blocked_words: Vec<String>,
    /// Muted users: client_id -> MuteEntry
    muted_users: DashMap<String, MuteEntry>,
    /// Message rate tracking: client_id -> timestamps
    rate_tracking: DashMap<String, Vec<DateTime<Utc>>>,
    /// Max messages per minute
    max_messages_per_minute: u32,
}

impl ContentModerator {
    pub fn new(blocked_words: Vec<String>, max_messages_per_minute: u32) -> Self {
        let normalized: Vec<String> = blocked_words.into_iter().map(|w| w.to_lowercase()).collect();
        Self {
            blocked_words: normalized,
            muted_users: DashMap::new(),
            rate_tracking: DashMap::new(),
            max_messages_per_minute,
        }
    }

    /// Check content for blocked words.
    pub fn check_content(&self, content: &str) -> ModerationAction {
        let lower = content.to_lowercase();
        for word in &self.blocked_words {
            if lower.contains(word.as_str()) {
                warn!(blocked_word = %word, "blocked word detected in content");
                return ModerationAction::Block(format!("blocked word detected: {word}"));
            }
        }
        ModerationAction::Allow
    }

    /// Check if a user is muted (accounting for expiry).
    pub fn is_muted(&self, client_id: &ClientId) -> bool {
        let key = client_id.as_str();
        if let Some(entry) = self.muted_users.get(key) {
            if let Some(expires_at) = entry.expires_at {
                if Utc::now() >= expires_at {
                    // Mute has expired — remove it
                    drop(entry);
                    self.muted_users.remove(key);
                    return false;
                }
            }
            true
        } else {
            false
        }
    }

    /// Mute a user.
    pub fn mute_user(&self, client_id: &ClientId, reason: String, duration_secs: Option<u64>) {
        let key = client_id.as_str().to_string();
        let now = Utc::now();
        let expires_at =
            duration_secs.map(|secs| now + chrono::Duration::seconds(secs as i64));

        let entry = MuteEntry {
            muted_at: now,
            expires_at,
            reason: reason.clone(),
        };

        self.muted_users.insert(key.clone(), entry);
        debug!(client_id = %key, reason = %reason, "user muted");
    }

    /// Unmute a user.
    pub fn unmute_user(&self, client_id: &ClientId) -> bool {
        let key = client_id.as_str();
        let removed = self.muted_users.remove(key).is_some();
        if removed {
            debug!(client_id = %key, "user unmuted");
        }
        removed
    }

    /// Check message rate for spam detection.
    pub fn check_rate(&self, client_id: &ClientId) -> ModerationAction {
        let now = Utc::now();
        let key = client_id.as_str().to_string();
        let mut entry = self.rate_tracking.entry(key).or_default();

        // Clean up entries older than 1 minute
        let one_min_ago = now - chrono::Duration::minutes(1);
        entry.retain(|t| *t > one_min_ago);

        entry.push(now);

        if entry.len() > self.max_messages_per_minute as usize {
            ModerationAction::Block("message rate limit exceeded".to_string())
        } else if entry.len() > (self.max_messages_per_minute as f64 * 0.8) as usize {
            ModerationAction::Warn("approaching message rate limit".to_string())
        } else {
            ModerationAction::Allow
        }
    }

    /// Full moderation check (mute + content + rate).
    pub fn check_message(&self, client_id: &ClientId, content: &str) -> ModerationAction {
        // 1. Mute check
        if self.is_muted(client_id) {
            return ModerationAction::Block("user is muted".to_string());
        }
        // 2. Content check
        let content_result = self.check_content(content);
        if content_result != ModerationAction::Allow {
            return content_result;
        }
        // 3. Rate check
        self.check_rate(client_id)
    }

    /// Get mute info for a user.
    pub fn get_mute(&self, client_id: &ClientId) -> Option<MuteEntry> {
        let key = client_id.as_str();
        self.muted_users.get(key).map(|r| r.value().clone())
    }

    /// Number of currently muted users.
    pub fn muted_count(&self) -> usize {
        self.muted_users.len()
    }

    /// Add a word to the blocklist.
    pub fn add_blocked_word(&mut self, word: String) {
        let lower = word.to_lowercase();
        if !self.blocked_words.contains(&lower) {
            self.blocked_words.push(lower);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_filter_blocks_bad_words() {
        let moderator = ContentModerator::new(
            vec!["spam".to_string(), "badword".to_string()],
            60,
        );

        assert_eq!(moderator.check_content("Hello world"), ModerationAction::Allow);
        assert_eq!(
            moderator.check_content("This is spam content"),
            ModerationAction::Block("blocked word detected: spam".to_string()),
        );
        // Case insensitive
        assert_eq!(
            moderator.check_content("SPAM IS BAD"),
            ModerationAction::Block("blocked word detected: spam".to_string()),
        );
        assert_eq!(
            moderator.check_content("Contains BadWord here"),
            ModerationAction::Block("blocked word detected: badword".to_string()),
        );
    }

    #[test]
    fn test_mute_unmute() {
        let moderator = ContentModerator::new(vec![], 60);
        let alice = ClientId::from_str("cli_alice");

        assert!(!moderator.is_muted(&alice));

        moderator.mute_user(&alice, "spamming".to_string(), None);
        assert!(moderator.is_muted(&alice));
        assert_eq!(moderator.muted_count(), 1);

        let mute = moderator.get_mute(&alice).unwrap();
        assert_eq!(mute.reason, "spamming");
        assert!(mute.expires_at.is_none());

        assert!(moderator.unmute_user(&alice));
        assert!(!moderator.is_muted(&alice));
        assert_eq!(moderator.muted_count(), 0);

        // Unmuting an already unmuted user returns false
        assert!(!moderator.unmute_user(&alice));
    }

    #[test]
    fn test_rate_limiting() {
        let moderator = ContentModerator::new(vec![], 5);
        let alice = ClientId::from_str("cli_alice");

        // First 4 messages should be allowed (80% of 5 = 4)
        for _ in 0..4 {
            assert_eq!(moderator.check_rate(&alice), ModerationAction::Allow);
        }

        // 5th message should trigger warning (> 80% threshold)
        assert_eq!(
            moderator.check_rate(&alice),
            ModerationAction::Warn("approaching message rate limit".to_string()),
        );

        // 6th message should be blocked (> limit)
        assert_eq!(
            moderator.check_rate(&alice),
            ModerationAction::Block("message rate limit exceeded".to_string()),
        );
    }

    #[test]
    fn test_full_check_message_flow() {
        let moderator = ContentModerator::new(vec!["badword".to_string()], 60);
        let alice = ClientId::from_str("cli_alice");
        let bob = ClientId::from_str("cli_bob");

        // Normal message passes
        assert_eq!(
            moderator.check_message(&alice, "Hello world"),
            ModerationAction::Allow,
        );

        // Blocked content
        assert_eq!(
            moderator.check_message(&alice, "This has badword"),
            ModerationAction::Block("blocked word detected: badword".to_string()),
        );

        // Muted user is blocked regardless of content
        moderator.mute_user(&bob, "testing".to_string(), None);
        assert_eq!(
            moderator.check_message(&bob, "Hello world"),
            ModerationAction::Block("user is muted".to_string()),
        );
    }

    #[test]
    fn test_mute_expiry() {
        let moderator = ContentModerator::new(vec![], 60);
        let alice = ClientId::from_str("cli_alice");

        // Mute with 0-second duration (expires immediately)
        moderator.mute_user(&alice, "testing".to_string(), Some(0));

        // The mute should have already expired
        assert!(!moderator.is_muted(&alice));
        assert_eq!(moderator.muted_count(), 0);
    }

    #[test]
    fn test_adding_blocked_words() {
        let mut moderator = ContentModerator::new(vec![], 60);

        assert_eq!(moderator.check_content("spam here"), ModerationAction::Allow);

        moderator.add_blocked_word("SPAM".to_string());
        assert_eq!(
            moderator.check_content("spam here"),
            ModerationAction::Block("blocked word detected: spam".to_string()),
        );

        // Adding the same word (case-insensitive) should not duplicate
        moderator.add_blocked_word("spam".to_string());
        assert_eq!(moderator.blocked_words.len(), 1);
    }
}
