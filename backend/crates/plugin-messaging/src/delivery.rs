use dashmap::DashMap;
use tokio::sync::mpsc;
use tracing::{debug, warn};

/// A message envelope routed to a specific client via WebSocket or similar transport.
#[derive(Debug, Clone)]
pub struct DeliveryMessage {
    /// ID of the original message being delivered.
    pub message_id: String,
    /// Target client ID.
    pub target_id: String,
    /// Serialised message content (JSON, binary, etc.).
    pub content: String,
    /// Message type tag (e.g. "direct", "channel", "system").
    pub message_type: String,
}

/// Result of a delivery attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeliveryResult {
    /// Message was placed into the client's channel successfully.
    Delivered,
    /// Client is offline; the caller should enqueue for later delivery.
    Queued,
    /// Delivery failed for the given reason (e.g. channel full).
    Failed(String),
}

/// Capacity of the per-client channel buffer.
const CLIENT_CHANNEL_CAPACITY: usize = 256;

/// Routes messages to online clients via bounded mpsc channels.
///
/// Each connected client calls `register_client` to obtain a `Receiver`. When the
/// client disconnects, `unregister_client` removes its sender so subsequent
/// `deliver` calls return `DeliveryResult::Queued`.
pub struct MessageDelivery {
    /// client_id -> mpsc::Sender
    online_senders: DashMap<String, mpsc::Sender<DeliveryMessage>>,
}

impl MessageDelivery {
    pub fn new() -> Self {
        Self {
            online_senders: DashMap::new(),
        }
    }

    /// Register a client and return the `Receiver` end of its delivery channel.
    /// If a sender for this client already exists it is replaced.
    pub fn register_client(&self, client_id: &str) -> mpsc::Receiver<DeliveryMessage> {
        let (tx, rx) = mpsc::channel(CLIENT_CHANNEL_CAPACITY);
        self.online_senders.insert(client_id.to_string(), tx);
        debug!(client_id = %client_id, "client registered for delivery");
        rx
    }

    /// Unregister a client (called on disconnect).
    pub fn unregister_client(&self, client_id: &str) {
        if self.online_senders.remove(client_id).is_some() {
            debug!(client_id = %client_id, "client unregistered from delivery");
        }
    }

    /// Check whether a client currently has an active delivery channel.
    pub fn is_online(&self, client_id: &str) -> bool {
        self.online_senders.contains_key(client_id)
    }

    /// Attempt to deliver a message to its target client.
    ///
    /// - `Delivered` — placed in the channel buffer.
    /// - `Queued`    — target is offline.
    /// - `Failed`    — channel buffer is full.
    pub async fn deliver(&self, msg: DeliveryMessage) -> DeliveryResult {
        let target = msg.target_id.clone();

        let sender = match self.online_senders.get(&target) {
            Some(s) => s.clone(),
            None => {
                debug!(target_id = %target, "target offline, message should be queued");
                return DeliveryResult::Queued;
            }
        };

        match sender.try_send(msg) {
            Ok(()) => {
                debug!(target_id = %target, "message delivered");
                DeliveryResult::Delivered
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
                warn!(target_id = %target, "delivery channel full");
                DeliveryResult::Failed(format!("channel full for client {target}"))
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                // Channel closed — treat as offline and clean up.
                self.online_senders.remove(&target);
                warn!(target_id = %target, "delivery channel closed unexpectedly");
                DeliveryResult::Queued
            }
        }
    }

    /// Number of currently registered (online) clients.
    pub fn online_count(&self) -> usize {
        self.online_senders.len()
    }
}

impl Default for MessageDelivery {
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

    fn make_msg(target: &str) -> DeliveryMessage {
        DeliveryMessage {
            message_id: "msg_001".to_string(),
            target_id: target.to_string(),
            content: r#"{"text":"hello"}"#.to_string(),
            message_type: "direct".to_string(),
        }
    }

    #[tokio::test]
    async fn test_register_and_deliver() {
        let delivery = MessageDelivery::new();
        let mut rx = delivery.register_client("cli_alice");

        assert!(delivery.is_online("cli_alice"));
        assert_eq!(delivery.online_count(), 1);

        let result = delivery.deliver(make_msg("cli_alice")).await;
        assert_eq!(result, DeliveryResult::Delivered);

        let received = rx.recv().await.expect("should receive message");
        assert_eq!(received.target_id, "cli_alice");
        assert_eq!(received.message_type, "direct");
    }

    #[tokio::test]
    async fn test_offline_client_returns_queued() {
        let delivery = MessageDelivery::new();

        // "cli_bob" never registered
        let result = delivery.deliver(make_msg("cli_bob")).await;
        assert_eq!(result, DeliveryResult::Queued);
    }

    #[tokio::test]
    async fn test_unregister_client() {
        let delivery = MessageDelivery::new();
        let _rx = delivery.register_client("cli_alice");

        assert!(delivery.is_online("cli_alice"));
        delivery.unregister_client("cli_alice");
        assert!(!delivery.is_online("cli_alice"));
        assert_eq!(delivery.online_count(), 0);

        // Delivery after unregistration should be Queued.
        let result = delivery.deliver(make_msg("cli_alice")).await;
        assert_eq!(result, DeliveryResult::Queued);
    }

    #[tokio::test]
    async fn test_online_count_multiple_clients() {
        let delivery = MessageDelivery::new();

        let _rx_a = delivery.register_client("cli_alice");
        let _rx_b = delivery.register_client("cli_bob");
        let _rx_c = delivery.register_client("cli_charlie");

        assert_eq!(delivery.online_count(), 3);

        delivery.unregister_client("cli_bob");
        assert_eq!(delivery.online_count(), 2);
        assert!(!delivery.is_online("cli_bob"));
        assert!(delivery.is_online("cli_alice"));
        assert!(delivery.is_online("cli_charlie"));
    }

    #[tokio::test]
    async fn test_multiple_deliveries_to_same_client() {
        let delivery = MessageDelivery::new();
        let mut rx = delivery.register_client("cli_alice");

        for i in 0..5u32 {
            let msg = DeliveryMessage {
                message_id: format!("msg_{i:03}"),
                target_id: "cli_alice".to_string(),
                content: format!("content {i}"),
                message_type: "channel".to_string(),
            };
            let result = delivery.deliver(msg).await;
            assert_eq!(result, DeliveryResult::Delivered);
        }

        let mut received = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            received.push(msg);
        }
        assert_eq!(received.len(), 5);
    }
}
