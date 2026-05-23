use async_graphql::{Context, Object, Result, SimpleObject};
use crate::context::GraphQlContext;

/// Lightweight server info returned by the `serverInfo` query.
#[derive(SimpleObject)]
pub struct ServerInfo {
    pub version: String,
    pub node_id: String,
    pub protocol: Vec<String>,
}

/// A minimal connection summary (extend with real data from connection-manager).
#[derive(SimpleObject)]
pub struct ConnectionSummary {
    pub total: i32,
    pub authenticated: i32,
}

/// A plugin entry.
#[derive(SimpleObject)]
pub struct PluginEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub enabled: bool,
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Server version and node metadata.
    async fn server_info(&self, ctx: &Context<'_>) -> Result<ServerInfo> {
        let cx = ctx.data::<GraphQlContext>()?;
        Ok(ServerInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            node_id: cx.node_id.as_ref().clone(),
            protocol: vec!["tcp".into(), "udp".into(), "ws".into(), "http".into()],
        })
    }

    /// Summary of current connections (stub — wire to connection-manager).
    async fn connections(&self, _ctx: &Context<'_>) -> Result<ConnectionSummary> {
        Ok(ConnectionSummary {
            total: 0,
            authenticated: 0,
        })
    }

    /// List registered plugins (stub — wire to plugin-host).
    async fn plugins(&self, _ctx: &Context<'_>) -> Result<Vec<PluginEntry>> {
        Ok(vec![
            PluginEntry {
                id: "io.draox.clans".into(),
                name: "plugin-clans".into(),
                version: "0.1.0".into(),
                enabled: true,
            },
            PluginEntry {
                id: "io.draox.messaging".into(),
                name: "plugin-messaging".into(),
                version: "0.1.0".into(),
                enabled: true,
            },
        ])
    }
}
