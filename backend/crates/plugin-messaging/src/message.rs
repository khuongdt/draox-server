use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use server_core::ClientId;

/// Unique message identifier.
pub type MessageId = String;

/// Message type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    /// Direct message between two clients.
    Direct,
    /// Message to a channel (group).
    Channel,
    /// Broadcast to all connected clients.
    Broadcast,
    /// System notification.
    System,
}

/// Message delivery status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageStatus {
    Sent,
    Delivered,
    Read,
    Failed,
}

/// Content type of a message payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentType {
    Text,
    Image,
    File,
    Embed,
    System,
}

impl Default for ContentType {
    fn default() -> Self {
        ContentType::Text
    }
}

/// A reaction on a message (emoji + list of users who reacted).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageReaction {
    pub emoji: String,
    pub users: Vec<String>,
}

/// A message envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Message {
    pub id: MessageId,
    pub message_type: MessageType,
    pub from: ClientId,
    pub to: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub status: MessageStatus,
    pub content_type: ContentType,
    pub reactions: Vec<MessageReaction>,
    pub reply_to: Option<MessageId>,
    pub edited: bool,
    pub edited_at: Option<DateTime<Utc>>,
}

impl Default for Message {
    fn default() -> Self {
        Self {
            id: format!("msg_{}", uuid::Uuid::new_v4().as_simple()),
            message_type: MessageType::Direct,
            from: ClientId::from_str("unknown"),
            to: String::new(),
            content: String::new(),
            timestamp: Utc::now(),
            status: MessageStatus::Sent,
            content_type: ContentType::Text,
            reactions: Vec::new(),
            reply_to: None,
            edited: false,
            edited_at: None,
        }
    }
}

impl Message {
    pub fn new(
        message_type: MessageType,
        from: ClientId,
        to: String,
        content: String,
    ) -> Self {
        Self {
            id: format!("msg_{}", uuid::Uuid::new_v4().as_simple()),
            message_type,
            from,
            to,
            content,
            timestamp: Utc::now(),
            status: MessageStatus::Sent,
            content_type: ContentType::Text,
            reactions: Vec::new(),
            reply_to: None,
            edited: false,
            edited_at: None,
        }
    }

    pub fn system(to: String, content: String) -> Self {
        Self::new(
            MessageType::System,
            ClientId::from_str("system"),
            to,
            content,
        )
    }

    /// Set the content type of this message (builder-style).
    pub fn with_content_type(mut self, ct: ContentType) -> Self {
        self.content_type = ct;
        self
    }

    /// Set a reply-to reference (builder-style).
    pub fn with_reply_to(mut self, msg_id: MessageId) -> Self {
        self.reply_to = Some(msg_id);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::new(
            MessageType::Direct,
            ClientId::from_str("cli_alice"),
            "cli_bob".to_string(),
            "Hello!".to_string(),
        );

        assert!(msg.id.starts_with("msg_"));
        assert_eq!(msg.message_type, MessageType::Direct);
        assert_eq!(msg.content, "Hello!");
        assert_eq!(msg.status, MessageStatus::Sent);
        assert_eq!(msg.content_type, ContentType::Text);
        assert!(msg.reactions.is_empty());
        assert!(msg.reply_to.is_none());
        assert!(!msg.edited);
        assert!(msg.edited_at.is_none());
    }

    #[test]
    fn test_system_message() {
        let msg = Message::system("cli_alice".to_string(), "Welcome!".to_string());
        assert_eq!(msg.message_type, MessageType::System);
        assert_eq!(msg.from.as_str(), "system");
    }

    #[test]
    fn test_content_type_default() {
        let ct = ContentType::default();
        assert_eq!(ct, ContentType::Text);
    }

    #[test]
    fn test_message_with_reply() {
        let original = Message::new(
            MessageType::Direct,
            ClientId::from_str("cli_alice"),
            "cli_bob".to_string(),
            "Original message".to_string(),
        );
        let original_id = original.id.clone();

        let reply = Message::new(
            MessageType::Direct,
            ClientId::from_str("cli_bob"),
            "cli_alice".to_string(),
            "Reply message".to_string(),
        )
        .with_reply_to(original_id.clone());

        assert_eq!(reply.reply_to, Some(original_id));
        assert_eq!(reply.content, "Reply message");
    }

    #[test]
    fn test_message_with_content_type() {
        let msg = Message::new(
            MessageType::Direct,
            ClientId::from_str("cli_alice"),
            "cli_bob".to_string(),
            "image_url".to_string(),
        )
        .with_content_type(ContentType::Image);

        assert_eq!(msg.content_type, ContentType::Image);
        assert_eq!(msg.content, "image_url");
    }
}
