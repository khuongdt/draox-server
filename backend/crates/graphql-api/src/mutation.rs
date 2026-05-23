use async_graphql::{Context, Object, Result};
use crate::context::GraphQlContext;

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    /// Enable or disable a plugin by its ID (stub — wire to plugin-host).
    async fn set_plugin_enabled(
        &self,
        _ctx: &Context<'_>,
        plugin_id: String,
        enabled: bool,
    ) -> Result<bool> {
        tracing::info!(plugin_id, enabled, "GraphQL: set_plugin_enabled");
        Ok(enabled)
    }

    /// Broadcast a server-wide notification to all connected clients (stub).
    async fn broadcast_notification(
        &self,
        _ctx: &Context<'_>,
        message: String,
    ) -> Result<i32> {
        tracing::info!(message, "GraphQL: broadcast_notification");
        Ok(0) // returns number of clients notified
    }
}
