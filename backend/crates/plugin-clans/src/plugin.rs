use crate::manager::ClanManager;
use plugin_sdk::traits::{BoxFuture, Plugin, PluginHealth};
use plugin_sdk::PluginContext;
use server_core::{PluginId, Result};
use std::sync::Arc;
use tracing::info;

const PLUGIN_ID: &str = "io.draox.clans";

/// Built-in Clans plugin.
///
/// Provides clan/group management: create, join, leave, roles, divisions.
pub struct ClansPlugin {
    id: PluginId,
    manager: Option<Arc<ClanManager>>,
}

impl ClansPlugin {
    pub fn new() -> Self {
        Self {
            id: PluginId::from_str(PLUGIN_ID),
            manager: None,
        }
    }

    /// Get the clan manager (available after activation).
    pub fn manager(&self) -> Option<&Arc<ClanManager>> {
        self.manager.as_ref()
    }
}

impl Default for ClansPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for ClansPlugin {
    fn id(&self) -> &PluginId {
        &self.id
    }

    fn name(&self) -> &str {
        "Clans"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn activate(&mut self, _ctx: PluginContext) -> BoxFuture<'_, Result<()>> {
        Box::pin(async {
            let max_members = 100;
            self.manager = Some(Arc::new(ClanManager::new(max_members)));
            info!("Clans plugin activated (max members: {max_members})");
            Ok(())
        })
    }

    fn deactivate(&mut self) -> BoxFuture<'_, Result<()>> {
        Box::pin(async {
            self.manager = None;
            info!("Clans plugin deactivated");
            Ok(())
        })
    }

    fn health_check(&self) -> BoxFuture<'_, PluginHealth> {
        Box::pin(async {
            if self.manager.is_some() {
                PluginHealth::Healthy
            } else {
                PluginHealth::Degraded {
                    reason: "not activated".to_string(),
                }
            }
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
    async fn test_clans_plugin_lifecycle() {
        let mut plugin = ClansPlugin::new();
        assert!(plugin.manager().is_none());

        let ctx = make_context(plugin.id());
        plugin.activate(ctx).await.unwrap();
        assert!(plugin.manager().is_some());

        let health = plugin.health_check().await;
        assert!(health.is_healthy());

        plugin.deactivate().await.unwrap();
        assert!(plugin.manager().is_none());
    }
}
