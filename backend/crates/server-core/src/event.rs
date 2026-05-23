use crate::types::{ConnectionId, PluginId, Protocol, SessionId};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::sync::Arc;
use tokio::sync::broadcast;

// ────────────────────────────────────────────────────────
// Server events
// ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ServerEvent {
    // Connection events
    ConnectionAccepted {
        connection_id: ConnectionId,
        protocol: Protocol,
        remote_addr: String,
    },
    ConnectionClosed {
        connection_id: ConnectionId,
        reason: String,
    },
    ConnectionError {
        connection_id: ConnectionId,
        error: String,
    },

    // Session events
    SessionCreated {
        session_id: SessionId,
    },
    SessionDestroyed {
        session_id: SessionId,
        reason: String,
    },

    // Traffic guard events
    GuardConnectionBlocked {
        remote_addr: String,
        reason: String,
    },
    GuardIpBanned {
        ip: IpAddr,
        duration_secs: u64,
    },
    GuardIpUnbanned {
        ip: IpAddr,
    },
    GuardAttackDetected {
        attack_type: String,
        source: String,
    },
    GuardThresholdAdjusted {
        metric: String,
        old_value: f64,
        new_value: f64,
    },

    // Plugin events
    PluginActivated {
        plugin_id: PluginId,
    },
    PluginDeactivated {
        plugin_id: PluginId,
    },
    PluginEnabled {
        plugin_id: PluginId,
    },
    PluginDisabled {
        plugin_id: PluginId,
    },
    PluginError {
        plugin_id: PluginId,
        error: String,
    },

    // Server lifecycle
    ServerStarted {
        timestamp: DateTime<Utc>,
    },
    ServerShuttingDown {
        reason: String,
    },

    // Custom plugin events
    Custom {
        source: String,
        name: String,
        payload: serde_json::Value,
    },
}

// ────────────────────────────────────────────────────────
// Event bus — pub/sub for server-wide events
// ────────────────────────────────────────────────────────

pub struct EventBus {
    sender: broadcast::Sender<Arc<ServerEvent>>,
    topic_senders: DashMap<String, broadcast::Sender<Arc<ServerEvent>>>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            topic_senders: DashMap::new(),
        }
    }

    pub fn publish(&self, event: ServerEvent) {
        let event = Arc::new(event);
        // Broadcast to global subscribers (ignore if no receivers)
        let _ = self.sender.send(Arc::clone(&event));

        // Broadcast to topic subscribers
        let topic = event_topic(&event);
        if let Some(sender) = self.topic_senders.get(&topic) {
            let _ = sender.send(event);
        }
    }

    pub fn subscribe_all(&self) -> broadcast::Receiver<Arc<ServerEvent>> {
        self.sender.subscribe()
    }

    pub fn subscribe_topic(&self, topic: impl Into<String>) -> broadcast::Receiver<Arc<ServerEvent>> {
        let topic = topic.into();
        self.topic_senders
            .entry(topic)
            .or_insert_with(|| broadcast::channel(256).0)
            .subscribe()
    }

    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(1024)
    }
}

fn event_topic(event: &ServerEvent) -> String {
    match event {
        ServerEvent::ConnectionAccepted { .. }
        | ServerEvent::ConnectionClosed { .. }
        | ServerEvent::ConnectionError { .. } => "connection".to_string(),

        ServerEvent::SessionCreated { .. }
        | ServerEvent::SessionDestroyed { .. } => "session".to_string(),

        ServerEvent::GuardConnectionBlocked { .. }
        | ServerEvent::GuardIpBanned { .. }
        | ServerEvent::GuardIpUnbanned { .. }
        | ServerEvent::GuardAttackDetected { .. }
        | ServerEvent::GuardThresholdAdjusted { .. } => "guard".to_string(),

        ServerEvent::PluginActivated { .. }
        | ServerEvent::PluginDeactivated { .. }
        | ServerEvent::PluginEnabled { .. }
        | ServerEvent::PluginDisabled { .. }
        | ServerEvent::PluginError { .. } => "plugin".to_string(),

        ServerEvent::ServerStarted { .. }
        | ServerEvent::ServerShuttingDown { .. } => "server".to_string(),

        ServerEvent::Custom { source, name, .. } => format!("custom.{source}.{name}"),
    }
}

// ────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_bus_publish_subscribe() {
        let bus = EventBus::new(16);
        let mut rx = bus.subscribe_all();

        bus.publish(ServerEvent::ServerStarted {
            timestamp: Utc::now(),
        });

        let event = rx.recv().await.unwrap();
        assert!(matches!(&*event, ServerEvent::ServerStarted { .. }));
    }

    #[tokio::test]
    async fn test_event_bus_topic_subscription() {
        let bus = EventBus::new(16);
        let mut rx_guard = bus.subscribe_topic("guard");
        let mut rx_conn = bus.subscribe_topic("connection");

        bus.publish(ServerEvent::GuardIpBanned {
            ip: "1.2.3.4".parse().unwrap(),
            duration_secs: 300,
        });

        // Guard subscriber should receive
        let event = rx_guard.recv().await.unwrap();
        assert!(matches!(&*event, ServerEvent::GuardIpBanned { .. }));

        // Connection subscriber should NOT receive (different topic)
        assert!(rx_conn.try_recv().is_err());
    }
}
