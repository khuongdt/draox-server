use serde::{Deserialize, Serialize};
use server_core::ClientId;
use tokio::sync::broadcast;
use crate::status::{PresenceStatus, UserPresence};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceChanged {
    pub client_id: ClientId,
    pub old_status: PresenceStatus,
    pub new_status: PresenceStatus,
}

/// Broadcast channel for presence change events.
pub struct PresenceBroadcaster {
    tx: broadcast::Sender<PresenceChanged>,
}

impl PresenceBroadcaster {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    pub fn announce(&self, old_status: PresenceStatus, presence: &UserPresence) {
        let event = PresenceChanged {
            client_id: presence.client_id.clone(),
            old_status,
            new_status: presence.status.clone(),
        };
        // Ignore send errors — no subscribers is fine
        let _ = self.tx.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<PresenceChanged> {
        self.tx.subscribe()
    }
}
