use dashmap::DashMap;
use server_core::ClientId;
use std::sync::Arc;
use crate::broadcast::PresenceBroadcaster;
use crate::status::{PresenceStatus, UserPresence};

pub struct PresenceManager {
    presences: Arc<DashMap<String, UserPresence>>,
    broadcaster: Arc<PresenceBroadcaster>,
}

impl PresenceManager {
    pub fn new() -> Self {
        Self {
            presences: Arc::new(DashMap::new()),
            broadcaster: Arc::new(PresenceBroadcaster::new(1024)),
        }
    }

    /// Called when a client connects.
    pub fn on_connect(&self, client_id: &ClientId) {
        let mut presence = self
            .presences
            .entry(client_id.as_str().to_string())
            .or_insert_with(|| UserPresence::new(client_id.clone()))
            .clone();

        let old = presence.status.clone();
        presence.set_online();
        self.presences.insert(client_id.as_str().to_string(), presence.clone());
        self.broadcaster.announce(old, &presence);
    }

    /// Called when a client disconnects.
    pub fn on_disconnect(&self, client_id: &ClientId) {
        if let Some(mut entry) = self.presences.get_mut(client_id.as_str()) {
            let old = entry.status.clone();
            entry.set_offline();
            self.broadcaster.announce(old, &*entry);
        }
    }

    /// Set a client's presence status manually.
    pub fn set_status(&self, client_id: &ClientId, status: PresenceStatus) {
        let mut entry = self
            .presences
            .entry(client_id.as_str().to_string())
            .or_insert_with(|| UserPresence::new(client_id.clone()));

        let old = entry.status.clone();
        entry.status = status;
        let updated = entry.clone();
        drop(entry);
        self.broadcaster.announce(old, &updated);
    }

    /// Update last activity time (call on any client message).
    pub fn touch(&self, client_id: &ClientId) {
        if let Some(mut entry) = self.presences.get_mut(client_id.as_str()) {
            entry.touch();
        }
    }

    /// Get presence for a specific client.
    pub fn get_presence(&self, client_id: &ClientId) -> Option<UserPresence> {
        self.presences.get(client_id.as_str()).map(|e| e.clone())
    }

    /// Get presence for multiple clients at once.
    pub fn get_presences(&self, client_ids: &[ClientId]) -> Vec<UserPresence> {
        client_ids
            .iter()
            .filter_map(|id| self.get_presence(id))
            .collect()
    }

    /// All known presences.
    pub fn all_presences(&self) -> Vec<UserPresence> {
        self.presences.iter().map(|e| e.value().clone()).collect()
    }

    /// Subscribe to presence change events.
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<crate::broadcast::PresenceChanged> {
        self.broadcaster.subscribe()
    }
}

impl Default for PresenceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_on_connect_sets_online() {
        let mgr = PresenceManager::new();
        let client = ClientId::from_str("cli_test");
        mgr.on_connect(&client);
        let p = mgr.get_presence(&client).unwrap();
        assert!(p.status.is_online());
    }

    #[test]
    fn test_on_disconnect_sets_offline() {
        let mgr = PresenceManager::new();
        let client = ClientId::from_str("cli_test2");
        mgr.on_connect(&client);
        mgr.on_disconnect(&client);
        let p = mgr.get_presence(&client).unwrap();
        assert_eq!(p.status, PresenceStatus::Offline);
    }

    #[test]
    fn test_custom_status() {
        let mgr = PresenceManager::new();
        let client = ClientId::from_str("cli_test3");
        mgr.on_connect(&client);
        mgr.set_status(
            &client,
            PresenceStatus::Custom {
                text: "In a meeting".to_string(),
                emoji: Some("🤝".to_string()),
            },
        );
        let p = mgr.get_presence(&client).unwrap();
        assert!(matches!(p.status, PresenceStatus::Custom { .. }));
    }

    #[tokio::test]
    async fn test_broadcast_on_status_change() {
        let mgr = PresenceManager::new();
        let mut rx = mgr.subscribe();
        let client = ClientId::from_str("cli_bcast");
        mgr.on_connect(&client);
        let event = rx.try_recv().unwrap();
        assert_eq!(event.client_id, client);
        assert_eq!(event.new_status, PresenceStatus::Online);
    }
}
