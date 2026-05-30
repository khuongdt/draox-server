use crate::manager::E2EEManager;
use plugin_sdk::traits::{BoxFuture, Plugin, PluginHealth};
use plugin_sdk::PluginContext;
use server_core::{PluginId, Result};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

const PLUGIN_ID: &str = "io.draox.e2ee";

/// Built-in End-to-End Encryption plugin.
///
/// Wraps the lower-level `E2EEManager` so the host's `PluginRegistry`
/// can manage its lifecycle and so it can be reached uniformly through
/// the `Plugin` trait. The manager itself remains a pure crypto utility
/// — this wrapper exists purely to satisfy P3 (every Layer-4 plugin
/// implements `Plugin`).
///
/// In a multi-tenant deployment, the single shared `E2EEManager` is the
/// server's own identity; per-client managers live elsewhere.
pub struct E2eePlugin {
    id:      PluginId,
    manager: Arc<RwLock<Option<Arc<E2EEManager>>>>,
}

impl E2eePlugin {
    pub fn new() -> Self {
        Self {
            id:      PluginId::from_str(PLUGIN_ID),
            manager: Arc::new(RwLock::new(None)),
        }
    }

    /// Access the shared manager after activation. Returns `None` while
    /// the plugin is in `Installed` state.
    pub async fn manager(&self) -> Option<Arc<E2EEManager>> {
        self.manager.read().await.clone()
    }
}

impl Default for E2eePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for E2eePlugin {
    fn id(&self) -> &PluginId {
        &self.id
    }

    fn name(&self) -> &str {
        "E2EE"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn activate(&mut self, _ctx: PluginContext) -> BoxFuture<'_, Result<()>> {
        Box::pin(async {
            let mut guard = self.manager.write().await;
            if guard.is_none() {
                *guard = Some(Arc::new(E2EEManager::new()));
                info!("E2EE plugin activated");
            }
            Ok(())
        })
    }

    fn deactivate(&mut self) -> BoxFuture<'_, Result<()>> {
        Box::pin(async {
            self.manager.write().await.take();
            info!("E2EE plugin deactivated");
            Ok(())
        })
    }

    fn health_check(&self) -> BoxFuture<'_, PluginHealth> {
        Box::pin(async {
            if self.manager.read().await.is_some() {
                PluginHealth::Healthy
            } else {
                PluginHealth::Degraded {
                    reason: "not activated".to_string(),
                }
            }
        })
    }
}
