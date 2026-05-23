use std::sync::Arc;

/// Services injected into every GraphQL resolver via the async-graphql Data API.
/// Add more service handles here as the schema grows.
#[derive(Clone)]
pub struct GraphQlContext {
    /// Opaque service handle — resolvers receive this via `ctx.data::<GraphQlContext>()`.
    pub node_id: Arc<String>,
    // Future: connection_manager, data_store, plugin_host handles…
}

impl GraphQlContext {
    pub fn new(node_id: impl Into<String>) -> Self {
        Self {
            node_id: Arc::new(node_id.into()),
        }
    }
}
