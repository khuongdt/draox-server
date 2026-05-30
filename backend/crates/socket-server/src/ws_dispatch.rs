use crate::handler::BoxFuture;
use serde::{Deserialize, Serialize};
use server_core::{ConnectionId, Result};

/// Dispatcher for WebSocket request frames.
///
/// `socket-server` parses inbound text frames as `WsFrame`. For frames with
/// `type == "request"`, it calls `dispatch(action, payload, connection_id)`
/// and writes a `response` frame back to the client carrying the returned
/// JSON (on success) or an error message.
///
/// `socket-server` does NOT import any plugin crate. The concrete dispatcher
/// lives in `plugin-host` (or whichever higher layer is wired in at startup);
/// `socket-server` only sees the trait.
pub trait WsActionDispatcher: Send + Sync + 'static {
    /// Handle a WS request frame. Implementors typically route by action
    /// prefix to the right plugin and forward the call.
    fn dispatch<'a>(
        &'a self,
        action: String,
        payload: serde_json::Value,
        connection_id: &'a ConnectionId,
    ) -> BoxFuture<'a, Result<serde_json::Value>>;
}

/// Wire-level shape of a WebSocket text frame.
///
/// Matches the SDK at `tools/sdk-web/src/types.ts::WsFrame`. Fields are
/// optional so the same struct round-trips request, response, event and
/// ping/pong frames.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WsFrame {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub frame_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ts: Option<i64>,
}

impl WsFrame {
    pub fn response_ok(id: Option<String>, data: serde_json::Value) -> Self {
        Self {
            id,
            frame_type: Some("response".to_string()),
            success: Some(true),
            data: Some(data),
            ..Default::default()
        }
    }

    pub fn response_err(id: Option<String>, error: impl Into<String>) -> Self {
        Self {
            id,
            frame_type: Some("response".to_string()),
            success: Some(false),
            error: Some(error.into()),
            ..Default::default()
        }
    }

    pub fn event(category: impl Into<String>, name: impl Into<String>, data: serde_json::Value, timestamp: impl Into<String>) -> Self {
        Self {
            frame_type: Some("event".to_string()),
            category:   Some(category.into()),
            name:       Some(name.into()),
            data:       Some(data),
            timestamp:  Some(timestamp.into()),
            ..Default::default()
        }
    }

    pub fn pong(ts: Option<i64>) -> Self {
        Self {
            frame_type: Some("pong".to_string()),
            ts,
            ..Default::default()
        }
    }
}
