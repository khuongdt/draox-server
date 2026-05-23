use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use server_core::ClientId;
use std::collections::HashSet;

/// Channel identifier.
pub type ChannelId = String;

/// Channel type classification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    Public,
    Private,
    Direct,
    Announcement,
}

impl Default for ChannelType {
    fn default() -> Self {
        ChannelType::Public
    }
}

/// A messaging channel (group chat).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Channel {
    pub id: ChannelId,
    pub name: String,
    pub description: String,
    pub created_by: ClientId,
    pub created_at: DateTime<Utc>,
    pub subscribers: HashSet<String>,
    pub channel_type: ChannelType,
    pub topic: String,
    pub pinned_messages: Vec<String>,
}

impl Default for Channel {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            description: String::new(),
            created_by: ClientId::from_str("unknown"),
            created_at: Utc::now(),
            subscribers: HashSet::new(),
            channel_type: ChannelType::Public,
            topic: String::new(),
            pinned_messages: Vec::new(),
        }
    }
}

impl Channel {
    pub fn new(id: ChannelId, name: String, created_by: ClientId) -> Self {
        let mut subscribers = HashSet::new();
        subscribers.insert(created_by.as_str().to_string());
        Self {
            id,
            name,
            description: String::new(),
            created_by,
            created_at: Utc::now(),
            subscribers,
            channel_type: ChannelType::Public,
            topic: String::new(),
            pinned_messages: Vec::new(),
        }
    }

    pub fn subscribe(&mut self, client_id: &ClientId) -> bool {
        self.subscribers.insert(client_id.as_str().to_string())
    }

    pub fn unsubscribe(&mut self, client_id: &ClientId) -> bool {
        self.subscribers.remove(client_id.as_str())
    }

    pub fn is_subscribed(&self, client_id: &ClientId) -> bool {
        self.subscribers.contains(client_id.as_str())
    }

    pub fn subscriber_count(&self) -> usize {
        self.subscribers.len()
    }

    /// Set the channel topic.
    pub fn set_topic(&mut self, topic: String) {
        self.topic = topic;
    }

    /// Pin a message in this channel.
    pub fn pin_message(&mut self, message_id: String) {
        if !self.pinned_messages.contains(&message_id) {
            self.pinned_messages.push(message_id);
        }
    }

    /// Unpin a message from this channel.
    pub fn unpin_message(&mut self, message_id: &str) {
        self.pinned_messages.retain(|id| id != message_id);
    }

    /// Check if this channel is an announcement channel.
    pub fn is_announcement(&self) -> bool {
        self.channel_type == ChannelType::Announcement
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_creation() {
        let creator = ClientId::from_str("cli_alice");
        let ch = Channel::new("ch_1".to_string(), "General".to_string(), creator.clone());

        assert_eq!(ch.name, "General");
        assert!(ch.is_subscribed(&creator));
        assert_eq!(ch.subscriber_count(), 1);
        assert_eq!(ch.channel_type, ChannelType::Public);
        assert!(ch.topic.is_empty());
        assert!(ch.pinned_messages.is_empty());
    }

    #[test]
    fn test_subscribe_unsubscribe() {
        let creator = ClientId::from_str("cli_alice");
        let bob = ClientId::from_str("cli_bob");
        let mut ch = Channel::new("ch_1".to_string(), "General".to_string(), creator);

        assert!(ch.subscribe(&bob));
        assert_eq!(ch.subscriber_count(), 2);
        assert!(ch.is_subscribed(&bob));

        assert!(ch.unsubscribe(&bob));
        assert_eq!(ch.subscriber_count(), 1);
        assert!(!ch.is_subscribed(&bob));
    }

    #[test]
    fn test_channel_type() {
        let ct = ChannelType::default();
        assert_eq!(ct, ChannelType::Public);

        let creator = ClientId::from_str("cli_alice");
        let mut ch = Channel::new("ch_1".to_string(), "Announcements".to_string(), creator);
        assert!(!ch.is_announcement());

        ch.channel_type = ChannelType::Announcement;
        assert!(ch.is_announcement());
    }

    #[test]
    fn test_pin_unpin() {
        let creator = ClientId::from_str("cli_alice");
        let mut ch = Channel::new("ch_1".to_string(), "General".to_string(), creator);

        ch.pin_message("msg_001".to_string());
        assert_eq!(ch.pinned_messages.len(), 1);

        // Pinning the same message again should not duplicate it
        ch.pin_message("msg_001".to_string());
        assert_eq!(ch.pinned_messages.len(), 1);

        ch.pin_message("msg_002".to_string());
        assert_eq!(ch.pinned_messages.len(), 2);

        ch.unpin_message("msg_001");
        assert_eq!(ch.pinned_messages.len(), 1);
        assert_eq!(ch.pinned_messages[0], "msg_002");

        // Unpinning a non-existent message is a no-op
        ch.unpin_message("msg_999");
        assert_eq!(ch.pinned_messages.len(), 1);
    }

    #[test]
    fn test_topic() {
        let creator = ClientId::from_str("cli_alice");
        let mut ch = Channel::new("ch_1".to_string(), "General".to_string(), creator);

        assert!(ch.topic.is_empty());

        ch.set_topic("Welcome to the general channel".to_string());
        assert_eq!(ch.topic, "Welcome to the general channel");

        ch.set_topic("Updated topic".to_string());
        assert_eq!(ch.topic, "Updated topic");
    }
}
