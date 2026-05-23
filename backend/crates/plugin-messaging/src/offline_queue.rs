use crate::message::Message;
use dashmap::DashMap;
use server_core::ClientId;
use std::collections::VecDeque;
use std::sync::RwLock;
use tracing::debug;

/// Queue messages for offline users, deliver on reconnect.
pub struct OfflineQueue {
    queues: DashMap<String, RwLock<VecDeque<Message>>>,
    max_per_user: usize,
}

impl OfflineQueue {
    pub fn new(max_per_user: usize) -> Self {
        Self {
            queues: DashMap::new(),
            max_per_user,
        }
    }

    /// Queue a message for an offline user.
    pub fn enqueue(&self, recipient: &ClientId, message: Message) {
        let key = recipient.as_str().to_string();
        let queue = self
            .queues
            .entry(key.clone())
            .or_insert_with(|| RwLock::new(VecDeque::new()));
        let mut q = queue.write().unwrap();
        q.push_back(message);
        // Evict oldest if over limit
        while q.len() > self.max_per_user {
            q.pop_front();
        }
        debug!(recipient = %key, queue_size = q.len(), "message enqueued for offline user");
    }

    /// Drain all queued messages for a user (call on reconnect).
    pub fn drain(&self, client_id: &ClientId) -> Vec<Message> {
        let key = client_id.as_str().to_string();
        if let Some(queue) = self.queues.get(&key) {
            let mut q = queue.write().unwrap();
            let messages: Vec<Message> = q.drain(..).collect();
            debug!(
                client_id = %key,
                count = messages.len(),
                "offline messages drained"
            );
            messages
        } else {
            Vec::new()
        }
    }

    /// Peek at queued messages without removing them.
    pub fn peek(&self, client_id: &ClientId, limit: usize) -> Vec<Message> {
        let key = client_id.as_str().to_string();
        if let Some(queue) = self.queues.get(&key) {
            let q = queue.read().unwrap();
            q.iter().take(limit).cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// Get the queue size for a user.
    pub fn queue_size(&self, client_id: &ClientId) -> usize {
        let key = client_id.as_str().to_string();
        if let Some(queue) = self.queues.get(&key) {
            let q = queue.read().unwrap();
            q.len()
        } else {
            0
        }
    }

    /// Check if there are queued messages.
    pub fn has_messages(&self, client_id: &ClientId) -> bool {
        self.queue_size(client_id) > 0
    }

    /// Total queued messages across all users.
    pub fn total_queued(&self) -> usize {
        self.queues
            .iter()
            .map(|entry| entry.value().read().unwrap().len())
            .sum()
    }

    /// Number of users with queued messages.
    pub fn user_count(&self) -> usize {
        self.queues
            .iter()
            .filter(|entry| !entry.value().read().unwrap().is_empty())
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{MessageType, Message};

    fn make_message(from: &str, to: &str, content: &str) -> Message {
        Message::new(
            MessageType::Direct,
            ClientId::from_str(from),
            to.to_string(),
            content.to_string(),
        )
    }

    #[test]
    fn test_enqueue_and_drain() {
        let queue = OfflineQueue::new(100);
        let bob = ClientId::from_str("cli_bob");

        queue.enqueue(&bob, make_message("cli_alice", "cli_bob", "Hello Bob!"));
        queue.enqueue(&bob, make_message("cli_alice", "cli_bob", "Are you there?"));
        assert_eq!(queue.queue_size(&bob), 2);

        let messages = queue.drain(&bob);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].content, "Hello Bob!");
        assert_eq!(messages[1].content, "Are you there?");

        // After drain, queue should be empty
        assert_eq!(queue.queue_size(&bob), 0);
        assert!(queue.drain(&bob).is_empty());
    }

    #[test]
    fn test_max_queue_eviction() {
        let queue = OfflineQueue::new(3);
        let bob = ClientId::from_str("cli_bob");

        queue.enqueue(&bob, make_message("cli_alice", "cli_bob", "Message 1"));
        queue.enqueue(&bob, make_message("cli_alice", "cli_bob", "Message 2"));
        queue.enqueue(&bob, make_message("cli_alice", "cli_bob", "Message 3"));
        queue.enqueue(&bob, make_message("cli_alice", "cli_bob", "Message 4"));
        queue.enqueue(&bob, make_message("cli_alice", "cli_bob", "Message 5"));

        // Only the last 3 should remain
        assert_eq!(queue.queue_size(&bob), 3);

        let messages = queue.drain(&bob);
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].content, "Message 3");
        assert_eq!(messages[1].content, "Message 4");
        assert_eq!(messages[2].content, "Message 5");
    }

    #[test]
    fn test_peek_without_removing() {
        let queue = OfflineQueue::new(100);
        let bob = ClientId::from_str("cli_bob");

        queue.enqueue(&bob, make_message("cli_alice", "cli_bob", "Hello!"));
        queue.enqueue(&bob, make_message("cli_alice", "cli_bob", "World!"));

        let peeked = queue.peek(&bob, 1);
        assert_eq!(peeked.len(), 1);
        assert_eq!(peeked[0].content, "Hello!");

        // Queue should still have both messages
        assert_eq!(queue.queue_size(&bob), 2);

        let peeked_all = queue.peek(&bob, 10);
        assert_eq!(peeked_all.len(), 2);
    }

    #[test]
    fn test_has_messages() {
        let queue = OfflineQueue::new(100);
        let bob = ClientId::from_str("cli_bob");
        let alice = ClientId::from_str("cli_alice");

        assert!(!queue.has_messages(&bob));

        queue.enqueue(&bob, make_message("cli_alice", "cli_bob", "Hello!"));
        assert!(queue.has_messages(&bob));
        assert!(!queue.has_messages(&alice));
    }

    #[test]
    fn test_total_queued_count() {
        let queue = OfflineQueue::new(100);
        let bob = ClientId::from_str("cli_bob");
        let charlie = ClientId::from_str("cli_charlie");

        assert_eq!(queue.total_queued(), 0);
        assert_eq!(queue.user_count(), 0);

        queue.enqueue(&bob, make_message("cli_alice", "cli_bob", "Msg 1"));
        queue.enqueue(&bob, make_message("cli_alice", "cli_bob", "Msg 2"));
        queue.enqueue(&charlie, make_message("cli_alice", "cli_charlie", "Msg 3"));

        assert_eq!(queue.total_queued(), 3);
        assert_eq!(queue.user_count(), 2);

        queue.drain(&bob);
        assert_eq!(queue.total_queued(), 1);
        assert_eq!(queue.user_count(), 1);
    }
}
