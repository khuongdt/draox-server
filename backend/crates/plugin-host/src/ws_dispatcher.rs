use crate::registry::PluginRegistry;
use plugin_sdk::context::EventBusHandle;
use plugin_sdk::traits::WsActionContext;
use plugin_sdk::Identity;
use server_core::event::{EventBus, ServerEvent};
use server_core::{ConnectionId, Result};
use socket_server::WsActionDispatcher;
use socket_server::handler::BoxFuture;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Concrete `WsActionDispatcher` (the trait lives in `socket-server`).
///
/// Wraps a `PluginRegistry` and an `EventBus` so it can build a
/// `WsActionContext` for each call. socket-server holds this behind
/// the trait — no plugin imports needed there.
pub struct PluginWsDispatcher {
    registry:  Arc<PluginRegistry>,
    event_bus: Arc<EventBus>,
}

impl PluginWsDispatcher {
    pub fn new(registry: Arc<PluginRegistry>, event_bus: Arc<EventBus>) -> Self {
        Self { registry, event_bus }
    }
}

impl WsActionDispatcher for PluginWsDispatcher {
    fn dispatch<'a>(
        &'a self,
        action: String,
        payload: serde_json::Value,
        connection_id: &'a ConnectionId,
    ) -> BoxFuture<'a, Result<serde_json::Value>> {
        let conn_id = connection_id.clone();
        Box::pin(async move {
            // For now WS connections are anonymous (no auth handshake on the
            // socket). Phase 5 may add token validation on connect; until then
            // we provide an "anonymous" identity so plugin handlers run.
            let identity = Identity::new("anonymous", "user");
            let events: Arc<dyn EventBusHandle> = Arc::new(EventBusBridge {
                bus: Arc::clone(&self.event_bus),
            });
            let ctx = WsActionContext {
                identity,
                connection_id: conn_id,
                events,
            };
            self.registry.dispatch_ws_action(&action, payload, ctx).await
        })
    }
}

/// Thin adapter so plugin code can call `ctx.events.publish(...)` /
/// `ctx.events.subscribe(...)` through the EventBusHandle trait without
/// caring that the concrete bus is `server_core::event::EventBus`.
struct EventBusBridge {
    bus: Arc<EventBus>,
}

impl EventBusHandle for EventBusBridge {
    fn publish(&self, event: ServerEvent) {
        self.bus.publish(event);
    }
    fn subscribe(&self, topic: &str) -> broadcast::Receiver<Arc<ServerEvent>> {
        self.bus.subscribe_topic(topic.to_string())
    }
}
