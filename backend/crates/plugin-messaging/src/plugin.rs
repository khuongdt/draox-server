use crate::http_api;
use crate::store::MessageStore;
use axum::Router;
use plugin_sdk::traits::{BoxFuture, Plugin, PluginHealth};
use plugin_sdk::PluginContext;
use server_core::{PluginId, Result};
use std::sync::Arc;
use tracing::info;

const PLUGIN_ID: &str = "io.draox.messaging";

/// Built-in Messaging plugin.
///
/// Provides direct, channel, and broadcast messaging between clients.
pub struct MessagingPlugin {
    id: PluginId,
    store: Option<Arc<MessageStore>>,
}

impl MessagingPlugin {
    pub fn new() -> Self {
        Self {
            id: PluginId::from_str(PLUGIN_ID),
            store: None,
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

    fn activate(&mut self, _ctx: PluginContext) -> BoxFuture<'_, Result<()>> {
        Box::pin(async {
            if self.store.is_none() {
                let max_messages = 100_000;
                self.store = Some(Arc::new(MessageStore::new(max_messages)));
                info!("Messaging plugin activated (max messages: {max_messages})");
            }
            Ok(())
        })
    }

    fn deactivate(&mut self) -> BoxFuture<'_, Result<()>> {
        Box::pin(async {
            self.store = None;
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
        self.store
            .as_ref()
            .map(|store| http_api::router(Arc::clone(store)))
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
