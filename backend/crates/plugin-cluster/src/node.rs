use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub type NodeId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub node_id: NodeId,
    pub address: String,
    pub http_port: u16,
    pub ws_port: u16,
    pub connection_count: usize,
    pub last_heartbeat: DateTime<Utc>,
}

impl NodeInfo {
    pub fn new(node_id: NodeId, address: String, http_port: u16, ws_port: u16) -> Self {
        Self {
            node_id,
            address,
            http_port,
            ws_port,
            connection_count: 0,
            last_heartbeat: Utc::now(),
        }
    }

    pub fn is_alive(&self, timeout_secs: i64) -> bool {
        let elapsed = Utc::now() - self.last_heartbeat;
        elapsed.num_seconds() < timeout_secs
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMessage {
    pub from_node: NodeId,
    pub topic: String,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

impl ClusterMessage {
    pub fn new(from_node: NodeId, topic: String, payload: serde_json::Value) -> Self {
        Self {
            from_node,
            topic,
            payload,
            timestamp: Utc::now(),
        }
    }
}
