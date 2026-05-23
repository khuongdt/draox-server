use server_core::{ConnectionId, ConnectionInfo, Error};
use std::future::Future;
use std::pin::Pin;
use tokio::sync::mpsc;

/// Boxed future for trait object compatibility.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Sender for outgoing messages to a connection.
pub type WriteSender = mpsc::Sender<OutgoingMessage>;

/// Messages that can be sent to a connection.
#[derive(Debug, Clone)]
pub enum OutgoingMessage {
    /// Raw binary data (TCP, UDP, WS binary).
    Binary(Vec<u8>),
    /// Text data (WS text, HTTP SSE).
    Text(String),
    /// Ping request (WS only, ignored for TCP/UDP).
    Ping,
    /// Close the connection gracefully.
    Close,
}

/// Handler for connection lifecycle events.
///
/// Implemented by the next layer in the pipeline (e.g., traffic-guard
/// or connection-manager). The handler can reject connections by
/// returning `Err` from `on_connect`.
pub trait ConnectionHandler: Send + Sync + 'static {
    /// Called when a new raw connection is accepted.
    /// Return `Ok(())` to accept, `Err` to reject (closes the connection).
    fn on_connect<'a>(&'a self, info: &'a ConnectionInfo) -> BoxFuture<'a, server_core::Result<()>>;

    /// Called when binary data is received from a connection.
    fn on_data<'a>(&'a self, conn_id: &'a ConnectionId, data: &'a [u8]) -> BoxFuture<'a, ()>;

    /// Called when a text message is received (WebSocket text frames).
    /// Default: no-op.
    fn on_text<'a>(&'a self, _conn_id: &'a ConnectionId, _text: &'a str) -> BoxFuture<'a, ()> {
        Box::pin(async {})
    }

    /// Called when a connection is closed.
    fn on_disconnect<'a>(
        &'a self,
        conn_id: &'a ConnectionId,
        reason: &'a str,
    ) -> BoxFuture<'a, ()>;

    /// Called when a connection error occurs.
    fn on_error<'a>(&'a self, conn_id: &'a ConnectionId, error: &'a Error) -> BoxFuture<'a, ()>;
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// No-op handler for testing. Accepts all connections, ignores all data.
    pub struct NoopHandler;

    impl ConnectionHandler for NoopHandler {
        fn on_connect<'a>(
            &'a self,
            _info: &'a ConnectionInfo,
        ) -> BoxFuture<'a, server_core::Result<()>> {
            Box::pin(async { Ok(()) })
        }

        fn on_data<'a>(&'a self, _conn_id: &'a ConnectionId, _data: &'a [u8]) -> BoxFuture<'a, ()> {
            Box::pin(async {})
        }

        fn on_disconnect<'a>(
            &'a self,
            _conn_id: &'a ConnectionId,
            _reason: &'a str,
        ) -> BoxFuture<'a, ()> {
            Box::pin(async {})
        }

        fn on_error<'a>(
            &'a self,
            _conn_id: &'a ConnectionId,
            _error: &'a Error,
        ) -> BoxFuture<'a, ()> {
            Box::pin(async {})
        }
    }

    #[test]
    fn test_outgoing_message_variants() {
        let msg = OutgoingMessage::Binary(vec![1, 2, 3]);
        assert!(matches!(msg, OutgoingMessage::Binary(_)));

        let msg = OutgoingMessage::Text("hello".to_string());
        assert!(matches!(msg, OutgoingMessage::Text(_)));

        let msg = OutgoingMessage::Ping;
        assert!(matches!(msg, OutgoingMessage::Ping));

        let msg = OutgoingMessage::Close;
        assert!(matches!(msg, OutgoingMessage::Close));
    }
}
