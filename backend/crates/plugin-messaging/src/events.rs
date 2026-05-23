use serde::{Deserialize, Serialize};

/// Domain events emitted by the messaging plugin onto the server EventBus.
///
/// Consumers (other plugins, admin dashboards, analytics) can subscribe to these
/// events to react to messaging activity without coupling to plugin internals.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessagingEvent {
    /// A new message was sent.
    MessageSent {
        message_id: String,
        from: String,
        to: String,
        /// Set when the message targets a channel rather than a direct peer.
        channel_id: Option<String>,
    },
    /// A message was delivered to the target client.
    MessageDelivered {
        message_id: String,
        to: String,
    },
    /// A message was read by a user.
    MessageRead {
        message_id: String,
        by: String,
    },
    /// A message was deleted.
    MessageDeleted {
        message_id: String,
        by: String,
    },
    /// A new channel was created.
    ChannelCreated {
        channel_id: String,
        name: String,
        channel_type: String,
    },
    /// A channel was deleted.
    ChannelDeleted {
        channel_id: String,
    },
    /// A user joined a channel.
    UserJoinedChannel {
        channel_id: String,
        user_id: String,
    },
    /// A user left a channel.
    UserLeftChannel {
        channel_id: String,
        user_id: String,
    },
    /// A user's presence status changed.
    PresenceChanged {
        user_id: String,
        /// Human-readable status label, e.g. "online", "away", "offline".
        status: String,
    },
    /// A user started typing in a channel.
    TypingStarted {
        channel_id: String,
        user_id: String,
    },
    /// A file was successfully uploaded.
    FileUploaded {
        file_id: String,
        user_id: String,
        filename: String,
    },
}

impl MessagingEvent {
    /// Returns a stable string tag identifying the event type.
    /// Useful for routing, filtering, and logging.
    pub fn event_type(&self) -> &'static str {
        match self {
            MessagingEvent::MessageSent { .. } => "messaging.message_sent",
            MessagingEvent::MessageDelivered { .. } => "messaging.message_delivered",
            MessagingEvent::MessageRead { .. } => "messaging.message_read",
            MessagingEvent::MessageDeleted { .. } => "messaging.message_deleted",
            MessagingEvent::ChannelCreated { .. } => "messaging.channel_created",
            MessagingEvent::ChannelDeleted { .. } => "messaging.channel_deleted",
            MessagingEvent::UserJoinedChannel { .. } => "messaging.user_joined_channel",
            MessagingEvent::UserLeftChannel { .. } => "messaging.user_left_channel",
            MessagingEvent::PresenceChanged { .. } => "messaging.presence_changed",
            MessagingEvent::TypingStarted { .. } => "messaging.typing_started",
            MessagingEvent::FileUploaded { .. } => "messaging.file_uploaded",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_tags_are_correct() {
        let events: Vec<(MessagingEvent, &str)> = vec![
            (
                MessagingEvent::MessageSent {
                    message_id: "m1".to_string(),
                    from: "alice".to_string(),
                    to: "bob".to_string(),
                    channel_id: None,
                },
                "messaging.message_sent",
            ),
            (
                MessagingEvent::MessageDelivered {
                    message_id: "m1".to_string(),
                    to: "bob".to_string(),
                },
                "messaging.message_delivered",
            ),
            (
                MessagingEvent::MessageRead {
                    message_id: "m1".to_string(),
                    by: "bob".to_string(),
                },
                "messaging.message_read",
            ),
            (
                MessagingEvent::MessageDeleted {
                    message_id: "m1".to_string(),
                    by: "alice".to_string(),
                },
                "messaging.message_deleted",
            ),
            (
                MessagingEvent::ChannelCreated {
                    channel_id: "ch_1".to_string(),
                    name: "General".to_string(),
                    channel_type: "public".to_string(),
                },
                "messaging.channel_created",
            ),
            (
                MessagingEvent::ChannelDeleted {
                    channel_id: "ch_1".to_string(),
                },
                "messaging.channel_deleted",
            ),
            (
                MessagingEvent::UserJoinedChannel {
                    channel_id: "ch_1".to_string(),
                    user_id: "alice".to_string(),
                },
                "messaging.user_joined_channel",
            ),
            (
                MessagingEvent::UserLeftChannel {
                    channel_id: "ch_1".to_string(),
                    user_id: "alice".to_string(),
                },
                "messaging.user_left_channel",
            ),
            (
                MessagingEvent::PresenceChanged {
                    user_id: "alice".to_string(),
                    status: "online".to_string(),
                },
                "messaging.presence_changed",
            ),
            (
                MessagingEvent::TypingStarted {
                    channel_id: "ch_1".to_string(),
                    user_id: "alice".to_string(),
                },
                "messaging.typing_started",
            ),
            (
                MessagingEvent::FileUploaded {
                    file_id: "file_1".to_string(),
                    user_id: "alice".to_string(),
                    filename: "photo.png".to_string(),
                },
                "messaging.file_uploaded",
            ),
        ];

        for (event, expected_tag) in events {
            assert_eq!(
                event.event_type(),
                expected_tag,
                "wrong event_type for {:?}",
                event
            );
        }
    }

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let event = MessagingEvent::MessageSent {
            message_id: "msg_abc".to_string(),
            from: "cli_alice".to_string(),
            to: "cli_bob".to_string(),
            channel_id: Some("ch_general".to_string()),
        };

        let json = serde_json::to_string(&event).expect("serialization failed");
        let deserialized: MessagingEvent =
            serde_json::from_str(&json).expect("deserialization failed");

        assert_eq!(deserialized.event_type(), event.event_type());

        if let MessagingEvent::MessageSent { message_id, channel_id, .. } = deserialized {
            assert_eq!(message_id, "msg_abc");
            assert_eq!(channel_id, Some("ch_general".to_string()));
        } else {
            panic!("wrong variant after deserialization");
        }
    }

    #[test]
    fn test_all_eleven_variants_covered() {
        // This test will fail to compile if a new variant is added to MessagingEvent
        // but the event_type() match arm is forgotten.
        let all: Vec<MessagingEvent> = vec![
            MessagingEvent::MessageSent { message_id: "x".to_string(), from: "a".to_string(), to: "b".to_string(), channel_id: None },
            MessagingEvent::MessageDelivered { message_id: "x".to_string(), to: "b".to_string() },
            MessagingEvent::MessageRead { message_id: "x".to_string(), by: "a".to_string() },
            MessagingEvent::MessageDeleted { message_id: "x".to_string(), by: "a".to_string() },
            MessagingEvent::ChannelCreated { channel_id: "c".to_string(), name: "n".to_string(), channel_type: "public".to_string() },
            MessagingEvent::ChannelDeleted { channel_id: "c".to_string() },
            MessagingEvent::UserJoinedChannel { channel_id: "c".to_string(), user_id: "a".to_string() },
            MessagingEvent::UserLeftChannel { channel_id: "c".to_string(), user_id: "a".to_string() },
            MessagingEvent::PresenceChanged { user_id: "a".to_string(), status: "online".to_string() },
            MessagingEvent::TypingStarted { channel_id: "c".to_string(), user_id: "a".to_string() },
            MessagingEvent::FileUploaded { file_id: "f".to_string(), user_id: "a".to_string(), filename: "f.png".to_string() },
        ];

        // Every variant must produce a non-empty, namespaced event type.
        for event in all {
            let tag = event.event_type();
            assert!(tag.starts_with("messaging."), "event_type '{}' should be namespaced", tag);
        }
    }
}
