use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::net::SocketAddr;
use uuid::Uuid;

// ────────────────────────────────────────────────────────
// ID types — strongly typed wrappers for type safety
// ────────────────────────────────────────────────────────

macro_rules! define_id {
    ($name:ident, $prefix:expr) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            pub fn new() -> Self {
                Self(format!("{}_{}", $prefix, Uuid::new_v4().as_simple()))
            }

            pub fn from_str(s: impl Into<String>) -> Self {
                Self(s.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_string())
            }
        }
    };
}

define_id!(SessionId, "ses");
define_id!(ClientId, "cli");
define_id!(ConnectionId, "con");
define_id!(PluginId, "plg");

// ────────────────────────────────────────────────────────
// Protocol
// ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Tcp,
    Udp,
    WebSocket,
    Http,
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Protocol::Tcp => write!(f, "tcp"),
            Protocol::Udp => write!(f, "udp"),
            Protocol::WebSocket => write!(f, "websocket"),
            Protocol::Http => write!(f, "http"),
        }
    }
}

// ────────────────────────────────────────────────────────
// Connection types
// ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionRole {
    Primary,
    Notification,
    Control,
    Streaming,
}

impl Default for ConnectionRole {
    fn default() -> Self {
        Self::Primary
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionState {
    Connecting,
    Established,
    Closing,
    Closed,
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self::Connecting
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub id: ConnectionId,
    pub session_id: Option<SessionId>,
    pub protocol: Protocol,
    pub role: ConnectionRole,
    pub state: ConnectionState,
    pub remote_addr: SocketAddr,
    pub connected_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

impl ConnectionInfo {
    pub fn new(id: ConnectionId, protocol: Protocol, remote_addr: SocketAddr) -> Self {
        let now = Utc::now();
        Self {
            id,
            session_id: None,
            protocol,
            role: ConnectionRole::default(),
            state: ConnectionState::Connecting,
            remote_addr,
            connected_at: now,
            last_activity: now,
            bytes_sent: 0,
            bytes_received: 0,
        }
    }
}

// ────────────────────────────────────────────────────────
// Session types
// ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub session_id: SessionId,
    pub client_id: ClientId,
    pub connections: Vec<ConnectionId>,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub metadata: serde_json::Value,
    pub authenticated: bool,
}

impl SessionState {
    pub fn new(client_id: ClientId) -> Self {
        let now = Utc::now();
        Self {
            session_id: SessionId::new(),
            client_id,
            connections: Vec::new(),
            created_at: now,
            last_activity: now,
            metadata: serde_json::Value::Object(serde_json::Map::new()),
            authenticated: false,
        }
    }
}

// ────────────────────────────────────────────────────────
// Server info
// ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
    pub started_at: DateTime<Utc>,
    pub protocols: Vec<Protocol>,
}

impl Default for ServerInfo {
    fn default() -> Self {
        Self {
            name: "Draox Server".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            started_at: Utc::now(),
            protocols: vec![Protocol::Tcp, Protocol::Udp, Protocol::WebSocket, Protocol::Http],
        }
    }
}

// ────────────────────────────────────────────────────────
// Shutdown signal
// ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ShutdownSignal {
    sender: tokio::sync::broadcast::Sender<()>,
}

impl ShutdownSignal {
    pub fn new() -> (Self, ShutdownReceiver) {
        let (sender, receiver) = tokio::sync::broadcast::channel(1);
        (Self { sender }, ShutdownReceiver { receiver })
    }

    pub fn shutdown(&self) {
        let _ = self.sender.send(());
    }

    pub fn subscribe(&self) -> ShutdownReceiver {
        ShutdownReceiver {
            receiver: self.sender.subscribe(),
        }
    }
}

impl Default for ShutdownSignal {
    fn default() -> Self {
        Self::new().0
    }
}

pub struct ShutdownReceiver {
    receiver: tokio::sync::broadcast::Receiver<()>,
}

impl ShutdownReceiver {
    pub async fn recv(&mut self) {
        let _ = self.receiver.recv().await;
    }
}

// ────────────────────────────────────────────────────────
// Transport trait — raw protocol abstraction
// ────────────────────────────────────────────────────────

/// Transport abstraction for different protocols.
/// Represents a raw transport layer (TCP, UDP, WS, HTTP).
#[async_trait]
pub trait Transport: Send + Sync + 'static {
    /// Protocol identifier for this transport.
    fn protocol(&self) -> Protocol;

    /// Whether this transport supports reliable delivery.
    /// TCP and WebSocket are reliable; UDP is not.
    fn is_reliable(&self) -> bool;

    /// Maximum message size for this transport (0 = unlimited).
    fn max_message_size(&self) -> usize;
}

// ────────────────────────────────────────────────────────
// Handler trait — incoming data processor
// ────────────────────────────────────────────────────────

/// Handler trait for processing incoming data from connections.
/// Sits between socket-server and connection-manager in the pipeline.
#[async_trait]
pub trait Handler: Send + Sync + 'static {
    /// Handle raw bytes from a connection.
    async fn handle_data(&self, connection_id: &ConnectionId, data: &[u8]) -> crate::Result<()>;

    /// Handle a UTF-8 text message from a connection.
    async fn handle_text(&self, connection_id: &ConnectionId, text: &str) -> crate::Result<()>;

    /// Called when a new connection is established.
    async fn on_connect(&self, connection_id: &ConnectionId) -> crate::Result<()>;

    /// Called when a connection is lost (clean or unclean).
    async fn on_disconnect(&self, connection_id: &ConnectionId) -> crate::Result<()>;
}

// ────────────────────────────────────────────────────────
// Middleware trait — pipeline interceptor
// ────────────────────────────────────────────────────────

/// Middleware trait for intercepting and transforming data in the pipeline.
/// Used by traffic-guard and other pipeline stages.
/// Return `None` from either method to drop/block the message.
#[async_trait]
pub trait Middleware: Send + Sync + 'static {
    /// Process data before it reaches the handler.
    /// Return `None` to drop or block the message.
    async fn process_inbound(
        &self,
        connection_id: &ConnectionId,
        data: &[u8],
    ) -> crate::Result<Option<Vec<u8>>>;

    /// Process data before it is sent back to the client.
    /// Return `None` to drop the message.
    async fn process_outbound(
        &self,
        connection_id: &ConnectionId,
        data: &[u8],
    ) -> crate::Result<Option<Vec<u8>>>;
}

// ────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_id_generation() {
        let id = SessionId::new();
        assert!(id.as_str().starts_with("ses_"));
    }

    #[test]
    fn test_connection_id_uniqueness() {
        let id1 = ConnectionId::new();
        let id2 = ConnectionId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_connection_info_defaults() {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let info = ConnectionInfo::new(ConnectionId::new(), Protocol::Tcp, addr);
        assert_eq!(info.state, ConnectionState::Connecting);
        assert_eq!(info.role, ConnectionRole::Primary);
        assert_eq!(info.bytes_sent, 0);
    }

    #[test]
    fn test_session_state_creation() {
        let session = SessionState::new(ClientId::new());
        assert!(session.connections.is_empty());
        assert!(!session.authenticated);
    }

    #[test]
    fn test_protocol_display() {
        assert_eq!(Protocol::Tcp.to_string(), "tcp");
        assert_eq!(Protocol::WebSocket.to_string(), "websocket");
    }

    #[test]
    fn test_id_serialization() {
        let id = SessionId::from_str("ses_test123");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"ses_test123\"");
        let deserialized: SessionId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, id);
    }

    // ── Trait smoke tests ───────────────────────────────

    /// Minimal Transport implementation for testing.
    struct TcpTransport;

    #[async_trait]
    impl Transport for TcpTransport {
        fn protocol(&self) -> Protocol {
            Protocol::Tcp
        }
        fn is_reliable(&self) -> bool {
            true
        }
        fn max_message_size(&self) -> usize {
            0
        }
    }

    #[test]
    fn test_transport_trait_impl() {
        let t = TcpTransport;
        assert_eq!(t.protocol(), Protocol::Tcp);
        assert!(t.is_reliable());
        assert_eq!(t.max_message_size(), 0);
    }

    /// Minimal Handler implementation for testing.
    struct EchoHandler;

    #[async_trait]
    impl Handler for EchoHandler {
        async fn handle_data(&self, _id: &ConnectionId, _data: &[u8]) -> crate::Result<()> {
            Ok(())
        }
        async fn handle_text(&self, _id: &ConnectionId, _text: &str) -> crate::Result<()> {
            Ok(())
        }
        async fn on_connect(&self, _id: &ConnectionId) -> crate::Result<()> {
            Ok(())
        }
        async fn on_disconnect(&self, _id: &ConnectionId) -> crate::Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_handler_trait_impl() {
        let h = EchoHandler;
        let id = ConnectionId::new();
        h.on_connect(&id).await.unwrap();
        h.handle_data(&id, b"hello").await.unwrap();
        h.handle_text(&id, "world").await.unwrap();
        h.on_disconnect(&id).await.unwrap();
    }

    /// Minimal pass-through Middleware implementation for testing.
    struct PassthroughMiddleware;

    #[async_trait]
    impl Middleware for PassthroughMiddleware {
        async fn process_inbound(
            &self,
            _id: &ConnectionId,
            data: &[u8],
        ) -> crate::Result<Option<Vec<u8>>> {
            Ok(Some(data.to_vec()))
        }
        async fn process_outbound(
            &self,
            _id: &ConnectionId,
            data: &[u8],
        ) -> crate::Result<Option<Vec<u8>>> {
            Ok(Some(data.to_vec()))
        }
    }

    /// Blocking Middleware that drops all messages.
    struct BlockingMiddleware;

    #[async_trait]
    impl Middleware for BlockingMiddleware {
        async fn process_inbound(
            &self,
            _id: &ConnectionId,
            _data: &[u8],
        ) -> crate::Result<Option<Vec<u8>>> {
            Ok(None)
        }
        async fn process_outbound(
            &self,
            _id: &ConnectionId,
            _data: &[u8],
        ) -> crate::Result<Option<Vec<u8>>> {
            Ok(None)
        }
    }

    #[tokio::test]
    async fn test_middleware_passthrough() {
        let m = PassthroughMiddleware;
        let id = ConnectionId::new();
        let payload = b"test payload";
        let result = m.process_inbound(&id, payload).await.unwrap();
        assert_eq!(result, Some(payload.to_vec()));
        let result = m.process_outbound(&id, payload).await.unwrap();
        assert_eq!(result, Some(payload.to_vec()));
    }

    #[tokio::test]
    async fn test_middleware_blocking() {
        let m = BlockingMiddleware;
        let id = ConnectionId::new();
        let result = m.process_inbound(&id, b"secret").await.unwrap();
        assert!(result.is_none(), "blocking middleware must return None");
        let result = m.process_outbound(&id, b"secret").await.unwrap();
        assert!(result.is_none(), "blocking middleware must return None");
    }
}
