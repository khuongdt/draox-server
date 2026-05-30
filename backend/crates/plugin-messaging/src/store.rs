use crate::channel::{Channel, ChannelId};
use crate::message::{Message, MessageId, MessageReaction, MessageStatus, MessageType};
use dashmap::DashMap;
use data_store::StorageBackend;
use server_core::{ClientId, Error, Result};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc;
use tracing::{debug, warn};

/// Storage namespace used for channel rows.
const STORAGE_NS: &str = "messaging";
/// Key prefix for channel rows: full key is `channel:{id}`.
const CHANNEL_KEY_PREFIX: &str = "channel:";

fn channel_key(id: &ChannelId) -> String {
    format!("{CHANNEL_KEY_PREFIX}{id}")
}

/// Operations consumed by the per-store persistence writer task.
///
/// Using a serialized queue (instead of per-mutation `tokio::spawn`) means
/// the writer always observes the in-memory DashMap state at the moment it
/// processes a request — which avoids stale-snapshot races where two rapid
/// mutations would each capture their own JSON snapshot and then race each
/// other to the backend.
enum PersistOp {
    Save(ChannelId),
    Delete(ChannelId),
}

/// In-memory message store with conversation and channel indexing.
pub struct MessageStore {
    messages: DashMap<MessageId, Message>,
    /// Index: client_id -> list of message IDs they're involved in
    client_messages: DashMap<String, Vec<MessageId>>,
    /// Index: channel_id -> list of message IDs
    channel_messages: DashMap<ChannelId, Vec<MessageId>>,
    /// Channels. Wrapped in `Arc` so the persistence writer task can hold a
    /// stable reference and re-read the latest state at write time.
    channels: Arc<DashMap<ChannelId, Channel>>,
    /// Message counter
    message_count: AtomicU64,
    /// Max messages to retain
    max_messages: usize,
    /// Sender feeding the per-store persistence writer task. Present iff
    /// `attach_storage` has been called. Mutators push `PersistOp` events;
    /// the writer drains in FIFO order and always re-reads `channels` so
    /// the last in-memory state always wins on disk.
    persist_tx: Option<mpsc::UnboundedSender<PersistOp>>,
    /// Backend handle used by `load_from_storage` only. The same Arc is
    /// also embedded inside the writer task spawned by `attach_storage`.
    loader_storage: Option<Arc<dyn StorageBackend>>,
}

impl MessageStore {
    pub fn new(max_messages: usize) -> Self {
        Self {
            messages: DashMap::new(),
            client_messages: DashMap::new(),
            channel_messages: DashMap::new(),
            channels: Arc::new(DashMap::new()),
            message_count: AtomicU64::new(0),
            max_messages,
            persist_tx: None,
            loader_storage: None,
        }
    }

    /// Attach a persistent backend. Spawns the persistence writer task
    /// that drains an internal queue and writes channel state through to
    /// the backend in FIFO order. Re-attaching replaces the previous
    /// writer (the old `tx` is dropped, the old writer exits cleanly).
    pub fn attach_storage(&mut self, storage: Arc<dyn StorageBackend>) {
        let (tx, mut rx) = mpsc::unbounded_channel::<PersistOp>();
        let channels = Arc::clone(&self.channels);
        let backend = Arc::clone(&storage);
        tokio::spawn(async move {
            while let Some(op) = rx.recv().await {
                match op {
                    PersistOp::Save(id) => {
                        let key = channel_key(&id);
                        let value = {
                            let Some(ch_ref) = channels.get(&id) else {
                                // The channel was removed before the writer
                                // got to it — make sure the backend matches.
                                let _ = backend.delete(STORAGE_NS, &key).await;
                                continue;
                            };
                            match serde_json::to_value(ch_ref.value()) {
                                Ok(v)  => v,
                                Err(e) => {
                                    warn!(error = %e, key = %key, "serialize channel failed");
                                    continue;
                                }
                            }
                        };
                        if let Err(e) = backend.set(STORAGE_NS, &key, value).await {
                            warn!(error = %e, key = %key, "persist channel failed");
                        }
                    }
                    PersistOp::Delete(id) => {
                        let key = channel_key(&id);
                        if let Err(e) = backend.delete(STORAGE_NS, &key).await {
                            warn!(error = %e, key = %key, "delete persisted channel failed");
                        }
                    }
                }
            }
        });
        self.persist_tx = Some(tx);
        // We don't keep the Arc<dyn StorageBackend> around on Self anymore —
        // it lives inside the writer task. `load_from_storage` needs it too,
        // so we hand it back via a separate path.
        self.loader_storage = Some(storage);
    }

    /// Re-populate `channels` from the persistent backend. Idempotent.
    /// Returns the number of channels loaded.
    pub async fn load_from_storage(&self) -> usize {
        let Some(storage) = self.loader_storage.as_ref() else { return 0 };
        let keys = match storage.list_keys(STORAGE_NS, CHANNEL_KEY_PREFIX).await {
            Ok(k)  => k,
            Err(e) => { warn!(error = %e, "list channel keys failed"); return 0 }
        };
        let mut loaded = 0;
        for key in keys {
            match storage.get(STORAGE_NS, &key).await {
                Ok(Some(value)) => match serde_json::from_value::<Channel>(value) {
                    Ok(ch) => {
                        self.channels.insert(ch.id.clone(), ch);
                        loaded += 1;
                    }
                    Err(e) => warn!(error = %e, key = %key, "deserialize channel failed"),
                },
                Ok(None) => {}
                Err(e)   => warn!(error = %e, key = %key, "get channel failed"),
            }
        }
        loaded
    }

    /// Queue a re-persist of the channel currently stored under `id`. The
    /// writer task will re-read the latest state at processing time, so
    /// stale-snapshot races are avoided.
    fn persist_channel_by_id(&self, id: &ChannelId) {
        if let Some(tx) = self.persist_tx.as_ref() {
            let _ = tx.send(PersistOp::Save(id.clone()));
        }
    }

    /// Queue a delete of the persisted channel row.
    fn delete_persisted_channel(&self, id: &ChannelId) {
        if let Some(tx) = self.persist_tx.as_ref() {
            let _ = tx.send(PersistOp::Delete(id.clone()));
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
        self.persist_channel_by_id(&id);
        id
    }

    /// Create a channel with a caller-supplied id. Used by the seed path
    /// to install the system "Draox" channel with a stable id. Returns
    /// `Err` if the id is already taken so seed calls are idempotent.
    pub fn create_channel_with_id(
        &self,
        id: ChannelId,
        name: String,
        created_by: ClientId,
        is_system: bool,
    ) -> Result<()> {
        if self.channels.contains_key(&id) {
            return Err(Error::Plugin {
                plugin_id: "io.draox.messaging".to_string(),
                message:   format!("channel already exists: {id}"),
            });
        }
        let mut channel = Channel::new(id.clone(), name, created_by);
        channel.is_system = is_system;
        self.channels.insert(id.clone(), channel);
        self.persist_channel_by_id(&id);
        Ok(())
    }

    /// Freeze or unfreeze a channel. Frozen channels reject new messages
    /// and new subscriptions; existing members keep read access.
    pub fn set_channel_frozen(&self, channel_id: &ChannelId, frozen: bool) -> Result<()> {
        {
            let mut ch = self
                .channels
                .get_mut(channel_id)
                .ok_or_else(|| Error::Plugin {
                    plugin_id: "io.draox.messaging".to_string(),
                    message:   format!("channel not found: {channel_id}"),
                })?;
            ch.frozen = frozen;
        }
        self.persist_channel_by_id(channel_id);
        Ok(())
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
        {
            let mut channel = self
                .channels
                .get_mut(channel_id)
                .ok_or_else(|| Error::Plugin {
                    plugin_id: "io.draox.messaging".to_string(),
                    message: format!("channel not found: {channel_id}"),
                })?;
            channel.subscribe(client_id);
        }
        self.persist_channel_by_id(channel_id);
        Ok(())
    }

    /// Unsubscribe a client from a channel.
    pub fn unsubscribe_channel(
        &self,
        channel_id: &ChannelId,
        client_id: &ClientId,
    ) -> Result<()> {
        {
            let mut channel = self
                .channels
                .get_mut(channel_id)
                .ok_or_else(|| Error::Plugin {
                    plugin_id: "io.draox.messaging".to_string(),
                    message: format!("channel not found: {channel_id}"),
                })?;
            channel.unsubscribe(client_id);
        }
        self.persist_channel_by_id(channel_id);
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

    /// Delete a single message from the store.
    pub fn delete_message(&self, message_id: &MessageId) -> Result<()> {
        self.messages
            .remove(message_id)
            .ok_or_else(|| Error::Plugin {
                plugin_id: "io.draox.messaging".to_string(),
                message: format!("message not found: {message_id}"),
            })?;
        Ok(())
    }

    /// Edit a message's text content. Marks the message as edited and
    /// sets `edited_at` to the current time.
    pub fn edit_message(&self, message_id: &MessageId, new_content: String) -> Result<Message> {
        let mut msg = self
            .messages
            .get_mut(message_id)
            .ok_or_else(|| Error::Plugin {
                plugin_id: "io.draox.messaging".to_string(),
                message: format!("message not found: {message_id}"),
            })?;
        msg.content = new_content;
        msg.edited = true;
        msg.edited_at = Some(chrono::Utc::now());
        Ok(msg.clone())
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
        self.delete_persisted_channel(channel_id);
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

    #[tokio::test]
    async fn test_channel_persists_and_reloads() {
        use data_store::SqliteStorage;

        let backend: Arc<dyn StorageBackend> = Arc::new(
            SqliteStorage::new_in_memory().await.expect("sqlite memory"),
        );

        // First store: create + mutate, then drop.
        {
            let mut store = MessageStore::new(1000);
            store.attach_storage(Arc::clone(&backend));
            store
                .create_channel_with_id(
                    "ch_persist".to_string(),
                    "Persist".to_string(),
                    ClientId::from_str("cli_alice"),
                    true, // is_system
                )
                .unwrap();
            store
                .subscribe_channel(&"ch_persist".to_string(), &ClientId::from_str("cli_bob"))
                .unwrap();
            store.set_channel_frozen(&"ch_persist".to_string(), true).unwrap();
        }

        // Give the fire-and-forget tokio::spawn writes time to land in
        // the backend before we read.
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;

        // Second store on the same backend re-hydrates from storage.
        let store = MessageStore::new(1000);
        let mut store_mut = store; // we need &mut for attach
        store_mut.attach_storage(Arc::clone(&backend));
        let store = store_mut;
        let loaded = store.load_from_storage().await;
        assert_eq!(loaded, 1, "exactly one channel should be reloaded");

        let ch = store.get_channel(&"ch_persist".to_string()).expect("reloaded channel");
        assert_eq!(ch.name, "Persist");
        assert!(ch.is_system, "is_system survived round-trip");
        assert!(ch.frozen, "frozen survived round-trip");
        assert!(ch.is_subscribed(&ClientId::from_str("cli_bob")), "subscriber survived round-trip");
    }

    #[tokio::test]
    async fn test_channel_delete_purges_persisted_row() {
        use data_store::SqliteStorage;

        let backend: Arc<dyn StorageBackend> = Arc::new(
            SqliteStorage::new_in_memory().await.expect("sqlite memory"),
        );

        let mut store = MessageStore::new(1000);
        store.attach_storage(Arc::clone(&backend));
        store
            .create_channel_with_id(
                "ch_doomed".to_string(),
                "Doomed".to_string(),
                ClientId::from_str("cli_alice"),
                false,
            )
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;

        store.delete_channel(&"ch_doomed".to_string()).unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;

        // A fresh store sees no row for ch_doomed.
        let mut fresh = MessageStore::new(1000);
        fresh.attach_storage(Arc::clone(&backend));
        let loaded = fresh.load_from_storage().await;
        assert_eq!(loaded, 0);
        assert!(fresh.get_channel(&"ch_doomed".to_string()).is_none());
    }
}
