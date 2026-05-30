use crate::http_api;
use crate::manager::ClanManager;
use axum::Router;
use plugin_sdk::context::EventBusHandle;
use plugin_sdk::traits::{BoxFuture, Plugin, PluginHealth};
use plugin_sdk::PluginContext;
use server_core::{ClientId, PluginId, Result};
use std::sync::Arc;
use tracing::{info, warn};

pub(crate) const PLUGIN_ID: &str = "io.draox.clans";

/// Stable id for the pre-seeded system "Draox" clan.
pub const SYSTEM_CLAN_ID: &str = "clan_draox";
/// Display name of the pre-seeded system clan.
pub const SYSTEM_CLAN_NAME: &str = "Draox";
/// Tag used for the system clan.
pub const SYSTEM_CLAN_TAG: &str = "DRAOX";

fn seed_system_draox(manager: &Arc<ClanManager>) {
    if manager.get_clan(&SYSTEM_CLAN_ID.to_string()).is_ok() {
        return;
    }
    let owner = ClientId::from_str("system");
    match manager.create_clan_with_id(
        SYSTEM_CLAN_ID.to_string(),
        SYSTEM_CLAN_NAME.to_string(),
        SYSTEM_CLAN_TAG.to_string(),
        owner,
        true, // is_system
    ) {
        Ok(()) => info!(
            clan_id = SYSTEM_CLAN_ID,
            "seeded system clan ({SYSTEM_CLAN_NAME})"
        ),
        Err(e) => warn!(error = %e, "failed to seed system clan"),
    }
}

/// Built-in Clans plugin.
///
/// Provides clan/group management: create, join, leave, roles, divisions.
pub struct ClansPlugin {
    id:      PluginId,
    manager: Option<Arc<ClanManager>>,
    events:  Option<Arc<dyn EventBusHandle>>,
}

impl ClansPlugin {
    pub fn new() -> Self {
        Self {
            id:      PluginId::from_str(PLUGIN_ID),
            manager: None,
            events:  None,
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

    fn activate(&mut self, ctx: PluginContext) -> BoxFuture<'_, Result<()>> {
        let events = Arc::clone(&ctx.events);
        Box::pin(async move {
            let max_members = 100;
            let manager = Arc::new(ClanManager::new(max_members));
            seed_system_draox(&manager);
            self.manager = Some(manager);
            self.events  = Some(events);
            info!("Clans plugin activated (max members: {max_members})");
            Ok(())
        })
    }

    fn deactivate(&mut self) -> BoxFuture<'_, Result<()>> {
        Box::pin(async {
            self.manager = None;
            self.events  = None;
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

    fn http_router(&self) -> Option<Router> {
        match (self.manager.as_ref(), self.events.as_ref()) {
            (Some(mgr), Some(events)) => {
                Some(http_api::router(Arc::clone(mgr), Arc::clone(events)))
            }
            _ => None,
        }
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
