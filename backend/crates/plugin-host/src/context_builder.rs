use cache_layer::CacheBackend;
use crate::handles::{
    BackendCacheHandle, EventBusHandleImpl, InMemoryStorageHandle, NoopConnectionHandle,
    NoopRouterHandle, NoopSchedulerHandle, PluginLoggerImpl,
};
use plugin_sdk::context::PluginContext;
use server_core::event::EventBus;
use server_core::{PluginId, ServerInfo};
use std::sync::Arc;

/// Builds `PluginContext` instances for plugins.
///
/// Holds references to shared server services (EventBus, cache backend, etc.)
/// and creates scoped handles for each plugin.
pub struct ContextBuilder {
    server_info: ServerInfo,
    event_bus: Arc<EventBus>,
    cache_backend: Arc<dyn CacheBackend>,
}

impl ContextBuilder {
    pub fn new(
        server_info: ServerInfo,
        event_bus: Arc<EventBus>,
        cache_backend: Arc<dyn CacheBackend>,
    ) -> Self {
        Self {
            server_info,
            event_bus,
            cache_backend,
        }
    }

    /// Build a `PluginContext` for the given plugin.
    ///
    /// Each plugin gets its own namespace-scoped storage/cache handles
    /// and a logger tagged with its plugin ID.
    pub fn build(&self, plugin_id: &PluginId, config: serde_json::Value) -> PluginContext {
        let id_str = plugin_id.as_str();

        PluginContext {
            plugin_id: plugin_id.clone(),
            server_info: self.server_info.clone(),
            config,
            connections: Arc::new(NoopConnectionHandle),
            storage: Arc::new(InMemoryStorageHandle::new(id_str)),
            cache: Arc::new(BackendCacheHandle::new(id_str, Arc::clone(&self.cache_backend))),
            events: Arc::new(EventBusHandleImpl::new(Arc::clone(&self.event_bus))),
            logger: Arc::new(PluginLoggerImpl::new(plugin_id.clone())),
            router: Arc::new(NoopRouterHandle),
            scheduler: Arc::new(NoopSchedulerHandle),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cache_layer::MemoryCache;
    use server_config::model::MemoryCacheConfig;

    #[test]
    fn test_context_builder_creates_context() {
        let bus = Arc::new(EventBus::new(16));
        let cache: Arc<dyn CacheBackend> = Arc::new(MemoryCache::new(&MemoryCacheConfig {
            max_capacity: 100,
            ttl_secs: 300,
        }));
        let builder = ContextBuilder::new(ServerInfo::default(), bus, cache);

        let plugin_id = PluginId::from_str("io.draox.test");
        let config = serde_json::json!({"enabled": true});
        let ctx = builder.build(&plugin_id, config.clone());

        assert_eq!(ctx.plugin_id(), &plugin_id);
        assert_eq!(ctx.config(), &config);
        assert_eq!(ctx.server_info().name, "Draox Server");
    }
}
