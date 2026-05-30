use crate::context::PluginContext;
use axum::Router;
use server_core::{PluginId, Result};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

// ────────────────────────────────────────────────────────
// Plugin trait — must be implemented by all plugins
// ────────────────────────────────────────────────────────

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait Plugin: Send + Sync + 'static {
    /// Unique plugin identifier (reverse-domain).
    fn id(&self) -> &PluginId;

    /// Human-readable name.
    fn name(&self) -> &str;

    /// Plugin version (semver).
    fn version(&self) -> &str;

    /// First-time initialization. Called when plugin is loaded.
    fn activate(&mut self, ctx: PluginContext) -> BoxFuture<'_, Result<()>>;

    /// Cleanup and shutdown. Called when plugin is unloaded.
    fn deactivate(&mut self) -> BoxFuture<'_, Result<()>>;

    /// Resume after disable. Called when plugin is re-enabled.
    fn on_enable(&mut self) -> BoxFuture<'_, Result<()>> {
        Box::pin(async { Ok(()) })
    }

    /// Pause without unload. Called when plugin is disabled.
    fn on_disable(&mut self) -> BoxFuture<'_, Result<()>> {
        Box::pin(async { Ok(()) })
    }

    /// Health check. Called periodically by plugin-host.
    fn health_check(&self) -> BoxFuture<'_, PluginHealth> {
        Box::pin(async { PluginHealth::Healthy })
    }

    /// Optional: contribute REST endpoints to the admin API HTTP server.
    ///
    /// Plugins that expose an HTTP surface return `Some(router)` where the
    /// router has its own internal state already baked in via
    /// `.with_state(...)`. `admin-api::build_router` merges these via
    /// `axum::Router::merge`, keeping admin-api free of plugin-specific
    /// imports.
    ///
    /// Default returns `None` — plugins without a REST API don't override.
    fn http_router(&self) -> Option<Router> {
        None
    }
}

// ────────────────────────────────────────────────────────
// Plugin health
// ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginHealth {
    Healthy,
    Degraded { reason: String },
    Unhealthy { reason: String },
}

impl PluginHealth {
    pub fn is_healthy(&self) -> bool {
        matches!(self, PluginHealth::Healthy)
    }
}

// ────────────────────────────────────────────────────────
// Plugin state (lifecycle)
// ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginState {
    /// Installed but not yet activated.
    Installed,
    /// Activated and enabled — running.
    ActiveEnabled,
    /// Activated but disabled — paused.
    ActiveDisabled,
    /// Marked for removal.
    Uninstalled,
}

impl PluginState {
    pub fn is_active(&self) -> bool {
        matches!(self, PluginState::ActiveEnabled | PluginState::ActiveDisabled)
    }

    pub fn is_enabled(&self) -> bool {
        matches!(self, PluginState::ActiveEnabled)
    }
}

// ────────────────────────────────────────────────────────
// Activation event
// ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ActivationEvent {
    /// Activate when server starts.
    OnStartup,
    /// Activate on first connection of a specific protocol.
    OnConnection { protocol: String },
    /// Activate when a specific command is invoked.
    OnCommand { command: String },
    /// Activate when a specific route is requested.
    OnRoute { path: String },
}

// ────────────────────────────────────────────────────────
// Plugin contributions
// ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginContributions {
    #[serde(default)]
    pub commands: Vec<CommandContribution>,
    #[serde(default)]
    pub routes: Vec<RouteContribution>,
    #[serde(default)]
    pub events: Vec<String>,
    #[serde(default)]
    pub settings: Vec<SettingContribution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandContribution {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteContribution {
    pub method: String,
    pub path: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingContribution {
    pub key: String,
    pub description: String,
    #[serde(rename = "type")]
    pub value_type: String,
    pub default: Option<serde_json::Value>,
}

// ────────────────────────────────────────────────────────
// Plugin permissions
// ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginPermissions {
    #[serde(default)]
    pub storage: bool,
    #[serde(default)]
    pub cache: bool,
    #[serde(default)]
    pub connections: bool,
    #[serde(default)]
    pub events: bool,
    #[serde(default)]
    pub scheduler: bool,
    #[serde(default)]
    pub network: bool,
    #[serde(default)]
    pub filesystem: bool,
}

// ────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_state_transitions() {
        assert!(!PluginState::Installed.is_active());
        assert!(PluginState::ActiveEnabled.is_active());
        assert!(PluginState::ActiveEnabled.is_enabled());
        assert!(PluginState::ActiveDisabled.is_active());
        assert!(!PluginState::ActiveDisabled.is_enabled());
        assert!(!PluginState::Uninstalled.is_active());
    }

    #[test]
    fn test_plugin_health() {
        assert!(PluginHealth::Healthy.is_healthy());
        assert!(!PluginHealth::Degraded {
            reason: "test".to_string()
        }
        .is_healthy());
    }

    #[test]
    fn test_plugin_permissions_default() {
        let perms = PluginPermissions::default();
        assert!(!perms.storage);
        assert!(!perms.cache);
        assert!(!perms.connections);
    }
}
