use crate::channel::{Channel, ChannelId};
use crate::message::{Message, MessageId, MessageReaction, MessageStatus, MessageType};
use dashmap::DashMap;
use server_core::{ClientId, Error, Result};
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::debug;

/// In-memory message store with conversation and channel indexing.
pub struct MessageStore {
    messages: DashMap<MessageId, Message>,
    /// Index: client_id -> list of message IDs they're involved in
    client_messages: DashMap<String, Vec<MessageId>>,
    /// Index: channel_id -> list of message IDs
    channel_messages: DashMap<ChannelId, Vec<MessageId>>,
    /// Channels
    channels: DashMap<ChannelId, Channel>,
    /// Message counter
    message_count: AtomicU64,
    /// Max messages to retain
    max_messages: usize,
}

impl MessageStore {
    pub fn new(max_messages: usize) -> Self {
        Self {
            messages: DashMap::new(),
            client_messages: DashMap::new(),
            channel_messages: DashMap::new(),
            channels: DashMap::new(),
            message_count: AtomicU64::new(0),
            max_messages,
        }
    }

    /// Store a message and index it.
    pub fn store_message(&self, message: Message) -> MessageId {
        let id = message.id.clone();
        let from = message.from.as_str().to_string();
        let to = message.to.clone();
        let msg_type = message.message_type.clone();

        self.messages.insert(id.clone(), message);

        // Index by sender
        self.client_messages
            .entry(from)
            .or_default()
            .push(id.clone());

        // Index by recipient or channel
        match msg_type {
            MessageType::Direct => {
                self.client_messages
                    .entry(to)
                    .or_default()
                    .push(id.clone());
            }
            MessageType::Channel => {
                self.channel_messages
                    .entry(to)
                    .or_default()
                    .push(id.clone());
            }
            _ => {}
        }

        self.message_count.fetch_add(1, Ordering::Relaxed);
        debug!(message_id = %id, "message stored");
        id
    }

    /// Get a message by ID.
    pub fn get_message(&self, id: &MessageId) -> Option<Message> {
        self.messages.get(id).map(|r| r.value().clone())
    }

    /// Get messages for a client (sent and received).
    pub fn get_client_messages(&self, client_id: &ClientId, limit: usize) -> Vec<Message> {
        let key = client_id.as_str().to_string();
        if let Some(ids) = self.client_messages.get(&key) {
            ids.iter()
                .rev()
                .take(limit)
                .filter_map(|id| self.messages.get(id).map(|r| r.value().clone()))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get messages in a channel.
    pub fn get_channel_messages(&self, channel_id: &ChannelId, limit: usize) -> Vec<Message> {
        if let Some(ids) = self.channel_messages.get(channel_id) {
            ids.iter()
                .rev()
                .take(limit)
                .filter_map(|id| self.messages.get(id).map(|r| r.value().clone()))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Create a channel.
    pub fn create_channel(
        &self,
        name: String,
        created_by: ClientId,
    ) -> ChannelId {
        let id = format!("ch_{}", uuid::Uuid::new_v4().as_simple());
        let channel = Channel::new(id.clone(), name, created_by);
        self.channels.insert(id.clone(), channel);
        id
    }

    /// Get a channel by ID.
    pub fn get_channel(&self, id: &ChannelId) -> Option<Channel> {
        self.channels.get(id).map(|r| r.value().clone())
    }

    /// Subscribe a client to a channel.
    pub fn subscribe_channel(
        &self,
        channel_id: &ChannelId,
        client_id: &ClientId,
    ) -> Result<()> {
        let mut channel = self
            .channels
            .get_mut(channel_id)
            .ok_or_else(|| Error::Plugin {
                plugin_id: "io.draox.messaging".to_string(),
                message: format!("channel not found: {channel_id}"),
            })?;
        channel.subscribe(client_id);
        Ok(())
    }

    /// Unsubscribe a client from a channel.
    pub fn unsubscribe_channel(
        &self,
        channel_id: &ChannelId,
        client_id: &ClientId,
    ) -> Result<()> {
        let mut channel = self
            .channels
            .get_mut(channel_id)
            .ok_or_else(|| Error::Plugin {
                plugin_id: "io.draox.messaging".to_string(),
                message: format!("channel not found: {channel_id}"),
            })?;
        channel.unsubscribe(client_id);
        Ok(())
    }

    /// List all channels.
    pub fn list_channels(&self) -> Vec<Channel> {
        self.channels.iter().map(|r| r.value().clone()).collect()
    }

    /// Maximum messages to retain.
    pub fn max_messages(&self) -> usize {
        self.max_messages
    }

    /// Total stored messages.
    pub fn message_count(&self) -> u64 {
        self.message_count.load(Ordering::Relaxed)
    }

    /// Total channels.
    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }

    /// Update the delivery status of a message.
    pub fn update_status(&self, message_id: &MessageId, status: MessageStatus) -> Result<()> {
        let mut msg = self
            .messages
            .get_mut(message_id)
            .ok_or_else(|| Error::Plugin {
                plugin_id: "io.draox.messaging".to_string(),
                message: format!("message not found: {message_id}"),
            })?;
        msg.status = status;
        debug!(message_id = %message_id, ?status, "message status updated");
        Ok(())
    }

    /// Search messages by content (case-insensitive substring match).
    pub fn search_messages(&self, query: &str, limit: usize) -> Vec<Message> {
        let q = query.to_lowercase();
        self.messages
            .iter()
            .filter(|r| r.value().content.to_lowercase().contains(&q))
            .take(limit)
            .map(|r| r.value().clone())
            .collect()
    }

    /// Add a reaction to a message.
    pub fn add_reaction(
        &self,
        message_id: &MessageId,
        emoji: String,
        user_id: String,
    ) -> Result<()> {
        let mut msg = self
            .messages
            .get_mut(message_id)
            .ok_or_else(|| Error::Plugin {
                plugin_id: "io.draox.messaging".to_string(),
                message: format!("message not found: {message_id}"),
            })?;
        if let Some(reaction) = msg.reactions.iter_mut().find(|r| r.emoji == emoji) {
            if !reaction.users.contains(&user_id) {
                reaction.users.push(user_id);
            }
        } else {
            msg.reactions.push(MessageReaction {
                emoji,
                users: vec![user_id],
            });
        }
        Ok(())
    }

    /// Remove a reaction from a message.
    pub fn remove_reaction(
        &self,
        message_id: &MessageId,
        emoji: &str,
        user_id: &str,
    ) -> Result<()> {
        let mut msg = self
            .messages
            .get_mut(message_id)
            .ok_or_else(|| Error::Plugin {
                plugin_id: "io.draox.messaging".to_string(),
                message: format!("message not found: {message_id}"),
            })?;
        if let Some(reaction) = msg.reactions.iter_mut().find(|r| r.emoji == emoji) {
            reaction.users.retain(|u| u != user_id);
        }
        Ok(())
    }

    /// Get thread messages (all replies to a given message).
    pub fn get_thread(&self, parent_id: &MessageId, limit: usize) -> Vec<Message> {
        self.messages
            .iter()
            .filter(|r| r.value().reply_to.as_deref() == Some(parent_id))
            .take(limit)
            .map(|r| r.value().clone())
            .collect()
    }

    /// Delete a channel and its message index.
    pub fn delete_channel(&self, channel_id: &ChannelId) -> Result<()> {
        self.channels
            .remove(channel_id)
            .ok_or_else(|| Error::Plugin {
                plugin_id: "io.draox.messaging".to_string(),
                message: format!("channel not found: {channel_id}"),
            })?;
        self.channel_messages.remove(channel_id);
        debug!(channel_id = %channel_id, "channel deleted");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_and_get_message() {
        let store = MessageStore::new(1000);
        let msg = Message::new(
            MessageType::Direct,
            ClientId::from_str("cli_alice"),
            "cli_bob".to_string(),
            "Hello Bob!".to_string(),
        );
        let id = store.store_message(msg);
        assert_eq!(store.message_count(), 1);

        let retrieved = store.get_message(&id).unwrap();
        assert_eq!(retrieved.content, "Hello Bob!");
    }

    #[test]
    fn test_client_messages_index() {
        let store = MessageStore::new(1000);
        let alice = ClientId::from_str("cli_alice");
        let bob = ClientId::from_str("cli_bob");

        store.store_message(Message::new(
            MessageType::Direct,
            alice.clone(),
            bob.as_str().to_string(),
            "Hi Bob!".to_string(),
        ));
        store.store_message(Message::new(
            MessageType::Direct,
            bob.clone(),
            alice.as_str().to_string(),
            "Hi Alice!".to_string(),
        ));

        let alice_msgs = store.get_client_messages(&alice, 10);
        assert_eq!(alice_msgs.len(), 2);

        let bob_msgs = store.get_client_messages(&bob, 10);
        assert_eq!(bob_msgs.len(), 2);
    }

    #[test]
    fn test_channel_messaging() {
        let store = MessageStore::new(1000);
        let alice = ClientId::from_str("cli_alice");

        let ch_id = store.create_channel("General".to_string(), alice.clone());
        assert_eq!(store.channel_count(), 1);

        store.store_message(Message::new(
            MessageType::Channel,
            alice,
            ch_id.clone(),
            "Hello channel!".to_string(),
        ));

        let msgs = store.get_channel_messages(&ch_id, 10);
        assert_eq!(msgs.len(), 1);
    }

    #[test]
    fn test_channel_subscribe_unsubscribe() {
        let store = MessageStore::new(1000);
        let alice = ClientId::from_str("cli_alice");
        let bob = ClientId::from_str("cli_bob");

        let ch_id = store.create_channel("General".to_string(), alice);
        store.subscribe_channel(&ch_id, &bob).unwrap();

        let ch = store.get_channel(&ch_id).unwrap();
        assert_eq!(ch.subscriber_count(), 2);
        assert!(ch.is_subscribed(&bob));

        store.unsubscribe_channel(&ch_id, &bob).unwrap();
        let ch = store.get_channel(&ch_id).unwrap();
        assert_eq!(ch.subscriber_count(), 1);
    }

    #[test]
    fn test_update_status() {
        let store = MessageStore::new(1000);
        let msg = Message::new(
            MessageType::Direct,
            ClientId::from_str("cli_alice"),
            "cli_bob".to_string(),
            "Hello!".to_string(),
        );
        let id = store.store_message(msg);

        assert_eq!(
            store.get_message(&id).unwrap().status,
            MessageStatus::Sent,
        );

        store.update_status(&id, MessageStatus::Delivered).unwrap();
        assert_eq!(
            store.get_message(&id).unwrap().status,
            MessageStatus::Delivered,
        );

        store.update_status(&id, MessageStatus::Read).unwrap();
        assert_eq!(
            store.get_message(&id).unwrap().status,
            MessageStatus::Read,
        );

        // Updating a non-existent message should fail
        let result = store.update_status(&"msg_nonexistent".to_string(), MessageStatus::Failed);
        assert!(result.is_err());
    }

    #[test]
    fn test_search_messages() {
        let store = MessageStore::new(1000);
        let alice = ClientId::from_str("cli_alice");

        store.store_message(Message::new(
            MessageType::Direct,
            alice.clone(),
            "cli_bob".to_string(),
            "Hello world".to_string(),
        ));
        store.store_message(Message::new(
            MessageType::Direct,
            alice.clone(),
            "cli_bob".to_string(),
            "Goodbye world".to_string(),
        ));
        store.store_message(Message::new(
            MessageType::Direct,
            alice,
            "cli_bob".to_string(),
            "Something else".to_string(),
        ));

        let results = store.search_messages("world", 10);
        assert_eq!(results.len(), 2);

        // Case-insensitive
        let results = store.search_messages("WORLD", 10);
        assert_eq!(results.len(), 2);

        // Limit
        let results = store.search_messages("world", 1);
        assert_eq!(results.len(), 1);

        // No results
        let results = store.search_messages("nonexistent", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_reactions() {
        let store = MessageStore::new(1000);
        let msg = Message::new(
            MessageType::Direct,
            ClientId::from_str("cli_alice"),
            "cli_bob".to_string(),
            "Hello!".to_string(),
        );
        let id = store.store_message(msg);

        // Add a reaction
        store
            .add_reaction(&id, "👍".to_string(), "cli_bob".to_string())
            .unwrap();
        let msg = store.get_message(&id).unwrap();
        assert_eq!(msg.reactions.len(), 1);
        assert_eq!(msg.reactions[0].emoji, "👍");
        assert_eq!(msg.reactions[0].users, vec!["cli_bob"]);

        // Adding the same reaction from the same user should not duplicate
        store
            .add_reaction(&id, "👍".to_string(), "cli_bob".to_string())
            .unwrap();
        let msg = store.get_message(&id).unwrap();
        assert_eq!(msg.reactions[0].users.len(), 1);

        // Another user reacts with the same emoji
        store
            .add_reaction(&id, "👍".to_string(), "cli_charlie".to_string())
            .unwrap();
        let msg = store.get_message(&id).unwrap();
        assert_eq!(msg.reactions[0].users.len(), 2);

        // Different emoji
        store
            .add_reaction(&id, "❤️".to_string(), "cli_bob".to_string())
            .unwrap();
        let msg = store.get_message(&id).unwrap();
        assert_eq!(msg.reactions.len(), 2);

        // Remove a reaction
        store.remove_reaction(&id, "👍", "cli_bob").unwrap();
        let msg = store.get_message(&id).unwrap();
        let thumbs_up = msg.reactions.iter().find(|r| r.emoji == "👍").unwrap();
        assert_eq!(thumbs_up.users.len(), 1);
        assert_eq!(thumbs_up.users[0], "cli_charlie");

        // Reaction on non-existent message should fail
        let result = store.add_reaction(
            &"msg_nonexistent".to_string(),
            "👍".to_string(),
            "cli_bob".to_string(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_threading() {
        let store = MessageStore::new(1000);
        let alice = ClientId::from_str("cli_alice");
        let bob = ClientId::from_str("cli_bob");

        let parent = Message::new(
            MessageType::Channel,
            alice.clone(),
            "ch_general".to_string(),
            "Parent message".to_string(),
        );
        let parent_id = store.store_message(parent);

        // Create replies
        let reply1 = Message::new(
            MessageType::Channel,
            bob.clone(),
            "ch_general".to_string(),
            "Reply 1".to_string(),
        )
        .with_reply_to(parent_id.clone());
        store.store_message(reply1);

        let reply2 = Message::new(
            MessageType::Channel,
            alice,
            "ch_general".to_string(),
            "Reply 2".to_string(),
        )
        .with_reply_to(parent_id.clone());
        store.store_message(reply2);

        // Unrelated message
        store.store_message(Message::new(
            MessageType::Channel,
            bob,
            "ch_general".to_string(),
            "Unrelated".to_string(),
        ));

        let thread = store.get_thread(&parent_id, 10);
        assert_eq!(thread.len(), 2);
        assert!(thread.iter().all(|m| m.reply_to.as_deref() == Some(&parent_id)));

        // Limit
        let thread = store.get_thread(&parent_id, 1);
        assert_eq!(thread.len(), 1);

        // No replies to a non-existent parent
        let thread = store.get_thread(&"msg_nonexistent".to_string(), 10);
        assert!(thread.is_empty());
    }

    #[test]
    fn test_delete_channel() {
        let store = MessageStore::new(1000);
        let alice = ClientId::from_str("cli_alice");

        let ch_id = store.create_channel("General".to_string(), alice.clone());
        assert_eq!(store.channel_count(), 1);

        // Add a message to the channel
        store.store_message(Message::new(
            MessageType::Channel,
            alice,
            ch_id.clone(),
            "Hello channel!".to_string(),
        ));
        assert_eq!(store.get_channel_messages(&ch_id, 10).len(), 1);

        // Delete the channel
        store.delete_channel(&ch_id).unwrap();
        assert_eq!(store.channel_count(), 0);
        assert!(store.get_channel(&ch_id).is_none());

        // Channel message index should be removed
        assert!(store.get_channel_messages(&ch_id, 10).is_empty());

        // Deleting a non-existent channel should fail
        let result = store.delete_channel(&"ch_nonexistent".to_string());
        assert!(result.is_err());
    }
}
