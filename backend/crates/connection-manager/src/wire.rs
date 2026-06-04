//! Wire protocol types shared across TCP, UDP, and WebSocket connections.
//!
//! All protocols use the same JSON message envelope.
//! TCP uses newline-delimited framing; UDP uses per-datagram; WebSocket uses built-in frames.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Inbound message sent by a client over any transport.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct WireRequest {
    /// Request/correlation ID. Echoed in the response so the client can match them.
    pub id: Option<String>,
    /// Frame type: `"request"` for RPC calls, `"ping"` for keep-alive.
    #[serde(rename = "type")]
    pub frame_type: Option<String>,
    /// Action name, e.g. `"auth"`, `"messaging.send_message"`, `"clans.create"`.
    pub action: Option<String>,
    /// Action-specific payload.
    pub payload: Option<Value>,
    /// Optional JWT carried in subsequent (post-auth) requests.
    pub token: Option<String>,
    /// Millisecond timestamp for ping messages.
    pub ts: Option<i64>,
}

impl WireRequest {
    /// Return true if this is a ping message.
    pub fn is_ping(&self) -> bool {
        self.frame_type.as_deref() == Some("ping")
    }

    /// Return true if this is a request message.
    pub fn is_request(&self) -> bool {
        self.frame_type.as_deref() == Some("request")
    }
}

/// Outbound response / event sent by the server.
#[derive(Debug, Clone, Serialize)]
pub struct WireResponse {
    /// Frame type: `"response"` for RPC replies, `"pong"` for keep-alive replies.
    #[serde(rename = "type")]
    pub frame_type: String,
    /// Echoed from the request `id` field (for response frames).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// `true` on success, `false` on error (for response frames).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
    /// Response payload on success (for response frames).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    /// Error message on failure (for response frames).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl WireResponse {
    /// Successful response with a data payload.
    pub fn ok(id: Option<String>, data: Value) -> Self {
        Self {
            frame_type: "response".into(),
            id,
            success: Some(true),
            data: Some(data),
            error: None,
        }
    }

    /// Error response.
    pub fn err(id: Option<String>, message: &str) -> Self {
        Self {
            frame_type: "response".into(),
            id,
            success: Some(false),
            data: None,
            error: Some(message.to_string()),
        }
    }

    /// Pong reply to a ping message.
    pub fn pong() -> Self {
        Self {
            frame_type: "pong".into(),
            id: None,
            success: None,
            data: None,
            error: None,
        }
    }

    /// Serialize to a JSON string and append the given delimiter (for TCP framing).
    pub fn to_line(&self, delimiter: &[u8]) -> Vec<u8> {
        let mut bytes = serde_json::to_vec(self).unwrap_or_else(|_| b"{}".to_vec());
        bytes.extend_from_slice(delimiter);
        bytes
    }
}

/// Minimal JWT claims decoded for wire-protocol authentication.
///
/// Intentionally minimal — connection-manager does not depend on admin-api.
/// The full `JwtClaims` (with AdminRole enum) lives in admin-api/src/auth.rs.
#[derive(Debug, Deserialize)]
pub(crate) struct TcpJwtClaims {
    /// Subject = username / user_id.
    pub sub: String,
    /// Role string: `"admin"`, `"operator"`, `"viewer"`.
    pub role: String,
    /// Expiry as a UNIX timestamp.
    pub exp: u64,
}

// ────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_wire_request_deserialize_request() {
        let raw = r#"{"id":"req_1","type":"request","action":"auth","payload":{"token":"t"}}"#;
        let req: WireRequest = serde_json::from_str(raw).unwrap();
        assert_eq!(req.id.as_deref(), Some("req_1"));
        assert!(req.is_request());
        assert_eq!(req.action.as_deref(), Some("auth"));
    }

    #[test]
    fn test_wire_request_deserialize_ping() {
        let raw = r#"{"type":"ping","ts":1234567890}"#;
        let req: WireRequest = serde_json::from_str(raw).unwrap();
        assert!(req.is_ping());
        assert_eq!(req.ts, Some(1234567890));
    }

    #[test]
    fn test_wire_response_ok_serializes() {
        let resp = WireResponse::ok(Some("req_1".into()), json!({"session_id":"ses_abc"}));
        let s = serde_json::to_string(&resp).unwrap();
        assert!(s.contains(r#""type":"response""#));
        assert!(s.contains(r#""success":true"#));
        assert!(s.contains("ses_abc"));
        assert!(!s.contains("error"));
    }

    #[test]
    fn test_wire_response_err_serializes() {
        let resp = WireResponse::err(Some("req_2".into()), "not authenticated");
        let s = serde_json::to_string(&resp).unwrap();
        assert!(s.contains(r#""success":false"#));
        assert!(s.contains("not authenticated"));
        assert!(!s.contains("data"));
    }

    #[test]
    fn test_wire_response_pong() {
        let resp = WireResponse::pong();
        let s = serde_json::to_string(&resp).unwrap();
        assert!(s.contains(r#""type":"pong""#));
        assert!(!s.contains("success"));
        assert!(!s.contains("data"));
    }

    #[test]
    fn test_to_line_appends_delimiter() {
        let resp = WireResponse::pong();
        let line = resp.to_line(b"\n");
        assert!(line.ends_with(b"\n"));
    }

    #[test]
    fn test_tcp_jwt_claims_deserialize() {
        let raw = r#"{"sub":"admin","role":"admin","exp":9999999999,"iat":1000000000}"#;
        let claims: TcpJwtClaims = serde_json::from_str(raw).unwrap();
        assert_eq!(claims.sub, "admin");
        assert_eq!(claims.role, "admin");
    }
}
