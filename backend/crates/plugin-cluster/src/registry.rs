use dashmap::DashMap;
use fred::prelude::*;
use server_core::{ClientId, SessionId};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::node::NodeId;

const SESSION_KEY_PREFIX: &str = "draox:session:";
const SESSION_TTL_SECS: u64 = 3600;

/// Session location record stored in the shared registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionLocation {
    pub session_id: SessionId,
    pub client_id: ClientId,
    pub node_id: NodeId,
    pub node_address: String,
}

/// Shared session registry backed by Redis.
/// Any node can look up which node owns a given session.
pub struct SharedSessionRegistry {
    redis: Arc<Client>,
    local_node: NodeId,
    local_cache: Arc<DashMap<String, SessionLocation>>,
}

impl SharedSessionRegistry {
    pub async fn new(redis_url: &str, local_node: NodeId) -> anyhow::Result<Self> {
        let config = Config::from_url(redis_url)?;
        let client = Arc::new(Client::new(config, None, None, None));
        client.connect();
        client.wait_for_connect().await?;
        Ok(Self {
            redis: client,
            local_node,
            local_cache: Arc::new(DashMap::new()),
        })
    }

    /// Register a session as belonging to this node.
    pub async fn register_session(
        &self,
        session_id: &SessionId,
        client_id: &ClientId,
        node_address: &str,
    ) -> anyhow::Result<()> {
        let location = SessionLocation {
            session_id: session_id.clone(),
            client_id: client_id.clone(),
            node_id: self.local_node.clone(),
            node_address: node_address.to_string(),
        };
        let key = format!("{}{}", SESSION_KEY_PREFIX, session_id);
        let value = serde_json::to_string(&location)?;
        self.redis
            .set::<Value, _, _>(&key, value, Some(Expiration::EX(SESSION_TTL_SECS as i64)), None, false)
            .await?;
        self.local_cache.insert(session_id.as_str().to_string(), location);
        Ok(())
    }

    /// Look up which node owns the given session.
    pub async fn locate_session(
        &self,
        session_id: &SessionId,
    ) -> anyhow::Result<Option<SessionLocation>> {
        // Check local cache first
        if let Some(loc) = self.local_cache.get(session_id.as_str()) {
            return Ok(Some(loc.clone()));
        }
        let key = format!("{}{}", SESSION_KEY_PREFIX, session_id);
        let value: Option<String> = self.redis.get(&key).await?;
        match value {
            Some(json) => {
                let location: SessionLocation = serde_json::from_str(&json)?;
                Ok(Some(location))
            }
            None => Ok(None),
        }
    }

    /// Remove a session from the registry (on disconnect).
    pub async fn unregister_session(&self, session_id: &SessionId) -> anyhow::Result<()> {
        let key = format!("{}{}", SESSION_KEY_PREFIX, session_id);
        self.redis.del::<Value, _>(&key).await?;
        self.local_cache.remove(session_id.as_str());
        Ok(())
    }

    /// Refresh TTL of a session entry (call on activity).
    pub async fn touch_session(&self, session_id: &SessionId) -> anyhow::Result<()> {
        let key = format!("{}{}", SESSION_KEY_PREFIX, session_id);
        self.redis
            .expire::<Value, _>(&key, SESSION_TTL_SECS as i64, None)
            .await?;
        Ok(())
    }
}
