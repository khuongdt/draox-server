use crate::http_api;
use crate::store::MessageStore;
use axum::Router;
use data_store::StorageBackend;
use plugin_sdk::context::EventBusHandle;
use plugin_sdk::traits::{BoxFuture, Plugin, PluginHealth, WsActionContext};
use plugin_sdk::PluginContext;
use server_core::{ClientId, Error, PluginId, Result};
use std::sync::Arc;
use tracing::{info, warn};

pub(crate) const PLUGIN_ID: &str = "io.draox.messaging";

/// Stable id for the pre-seeded system "Draox" channel.
pub const SYSTEM_CHANNEL_ID: &str = "ch_draox";
/// Display name of the pre-seeded system channel.
pub const SYSTEM_CHANNEL_NAME: &str = "Draox";

/// Create the system "Draox" channel if it doesn't already exist.
/// Idempotent — safe to call on every activation.
fn seed_system_draox(store: &Arc<MessageStore>) {
    if store.get_channel(&SYSTEM_CHANNEL_ID.to_string()).is_some() {
        return;
    }
    let creator = ClientId::from_str("system");
    match store.create_channel_with_id(
        SYSTEM_CHANNEL_ID.to_string(),
        SYSTEM_CHANNEL_NAME.to_string(),
        creator,
        true, // is_system
    ) {
        Ok(()) => info!(
            channel_id = SYSTEM_CHANNEL_ID,
            "seeded system channel ({SYSTEM_CHANNEL_NAME})"
        ),
        Err(e) => warn!(error = %e, "failed to seed system channel"),
    }
}

/// Built-in Messaging plugin.
///
/// Provides direct, channel, and broadcast messaging between clients.
pub struct MessagingPlugin {
    id:      PluginId,
    store:   Option<Arc<MessageStore>>,
    events:  Option<Arc<dyn EventBusHandle>>,
    storage: Option<Arc<dyn StorageBackend>>,
}

impl MessagingPlugin {
    pub fn new() -> Self {
        Self {
            id:      PluginId::from_str(PLUGIN_ID),
            store:   None,
            events:  None,
            storage: None,
        }
    }

    /// Build a Messaging plugin that mirrors channel state through to
    /// `storage`. On `activate()` the plugin first re-hydrates the
    /// channel cache from `storage`, then runs the idempotent seed for
    /// the system "Draox" channel.
    pub fn with_storage(storage: Arc<dyn StorageBackend>) -> Self {
        Self {
            id:      PluginId::from_str(PLUGIN_ID),
            store:   None,
            events:  None,
            storage: Some(storage),
        }
    }

    /// Get the message store (available after activation).
    pub fn store(&self) -> Option<&Arc<MessageStore>> {
        self.store.as_ref()
    }
}

impl Default for MessagingPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for MessagingPlugin {
    fn id(&self) -> &PluginId {
        &self.id
    }

    fn name(&self) -> &str {
        "Messaging"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn activate(&mut self, ctx: PluginContext) -> BoxFuture<'_, Result<()>> {
        let events = Arc::clone(&ctx.events);
        Box::pin(async move {
            if self.store.is_none() {
                let max_messages = 100_000;
                let mut store = MessageStore::new(max_messages);
                if let Some(storage) = self.storage.as_ref() {
                    store.attach_storage(Arc::clone(storage));
                }
                let store = Arc::new(store);
                let loaded = store.load_from_storage().await;
                info!(
                    "Messaging plugin activated (max messages: {max_messages}, loaded {loaded} channels from storage)"
                );
                self.store = Some(store);
            }
            self.events = Some(events);
            if let Some(store) = self.store.as_ref() {
                seed_system_draox(store);
            }
            Ok(())
        })
    }

    fn deactivate(&mut self) -> BoxFuture<'_, Result<()>> {
        Box::pin(async {
            self.store = None;
            self.events = None;
            info!("Messaging plugin deactivated");
            Ok(())
        })
    }

    fn health_check(&self) -> BoxFuture<'_, PluginHealth> {
        Box::pin(async {
            if self.store.is_some() {
                PluginHealth::Healthy
            } else {
                PluginHealth::Degraded {
                    reason: "not activated".to_string(),
                }
            }
        })
    }

    fn http_router(&self) -> Option<Router> {
        // Only contribute routes once the plugin has been activated and
        // owns a store. Returning None before activation is safe — admin-api
        // will simply skip this plugin until `activate()` populates the store.
        match (self.store.as_ref(), self.events.as_ref()) {
            (Some(store), Some(events)) => {
                Some(http_api::router(Arc::clone(store), Arc::clone(events)))
            }
            _ => None,
        }
    }

    fn ws_action_prefix(&self) -> Option<&'static str> {
        Some("messaging.")
    }

    fn handle_ws_action(
        &self,
        action: String,
        payload: serde_json::Value,
        ctx: WsActionContext,
    ) -> BoxFuture<'_, Result<serde_json::Value>> {
        Box::pin(async move {
            let store = self
                .store
                .as_ref()
                .ok_or_else(|| Error::Plugin {
                    plugin_id: PLUGIN_ID.to_string(),
                    message:   "messaging plugin not activated".to_string(),
                })?;
            crate::ws_actions::dispatch(store, &ctx, &action, payload).await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use server_core::event::EventBus;
    use server_core::ServerInfo;
    use std::sync::Arc;

    fn make_context(plugin_id: &PluginId) -> PluginContext {
        let bus = Arc::new(EventBus::new(16));
        let cache: Arc<dyn cache_layer::CacheBackend> = Arc::new(
            cache_layer::MemoryCache::new(&server_config::model::MemoryCacheConfig::default()),
        );
        let builder = plugin_host::ContextBuilder::new(ServerInfo::default(), bus, cache);
        builder.build(plugin_id, serde_json::json!({}))
    }

    #[tokio::test]
    async fn test_messaging_plugin_lifecycle() {
        let mut plugin = MessagingPlugin::new();
        assert!(plugin.store().is_none());

        let ctx = make_context(plugin.id());
        plugin.activate(ctx).await.unwrap();
        assert!(plugin.store().is_some());

        let health = plugin.health_check().await;
        assert!(health.is_healthy());

        plugin.deactivate().await.unwrap();
        assert!(plugin.store().is_none());
    }
}
