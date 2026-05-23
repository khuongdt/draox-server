use crate::manager::SessionManager;
use server_core::{ClientId, ConnectionId, ConnectionInfo, ConnectionRole, Error};
use socket_server::handler::{BoxFuture, ConnectionHandler};
use socket_server::ConnectionTracker;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Connection handler that creates/manages sessions for incoming connections.
///
/// Sits in the pipeline after traffic-guard (or directly after socket-server)
/// and translates raw connection events into session lifecycle operations.
pub struct SessionHandler {
    manager: Arc<SessionManager>,
    // Retained for future use: sending data back to connections, updating state, etc.
    #[allow(dead_code)]
    tracker: Arc<ConnectionTracker>,
}

impl SessionHandler {
    /// Create a new SessionHandler.
    pub fn new(manager: Arc<SessionManager>, tracker: Arc<ConnectionTracker>) -> Self {
        Self { manager, tracker }
    }
}

impl ConnectionHandler for SessionHandler {
    /// Called when a new connection is accepted.
    ///
    /// Creates a new session for the connection and binds it as Primary.
    fn on_connect<'a>(&'a self, info: &'a ConnectionInfo) -> BoxFuture<'a, server_core::Result<()>> {
        Box::pin(async move {
            let conn_id = info.id.clone();

            // Create a new client identity and session for this connection
            let client_id = ClientId::new();
            let session_id = self.manager.create_session(client_id.clone());

            // Bind the connection as Primary
            self.manager
                .bind_connection(&session_id, conn_id.clone(), ConnectionRole::Primary)?;

            info!(
                session_id = %session_id,
                conn_id = %conn_id,
                client_id = %client_id,
                remote_addr = %info.remote_addr,
                "session created for new connection"
            );

            Ok(())
        })
    }

    /// Called when binary data is received.
    ///
    /// Touches the session to update last_activity.
    fn on_data<'a>(&'a self, conn_id: &'a ConnectionId, _data: &'a [u8]) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            if let Some(session_id) = self.manager.get_session_by_connection(conn_id) {
                // The tracker already updates last_activity on the ConnectionInfo.
                // Session-level touch will be done when needed via session manager.
                let _ = session_id;
            }
            debug!(conn_id = %conn_id, "data received, session touched");
        })
    }

    /// Called when a connection is closed.
    ///
    /// Unbinds the connection from its session. If the session becomes empty,
    /// the heartbeat cleanup task will destroy it after the grace period.
    fn on_disconnect<'a>(
        &'a self,
        conn_id: &'a ConnectionId,
        reason: &'a str,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            if let Some(session_id) = self.manager.unbind_connection(conn_id) {
                info!(
                    session_id = %session_id,
                    conn_id = %conn_id,
                    reason = reason,
                    "connection disconnected from session"
                );
            } else {
                warn!(conn_id = %conn_id, reason = reason, "disconnect for unknown connection");
            }
        })
    }

    /// Called when a connection error occurs.
    fn on_error<'a>(&'a self, conn_id: &'a ConnectionId, err: &'a Error) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            error!(conn_id = %conn_id, error = %err, "connection error");
        })
    }
}

// ────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use server_config::model::SessionConfig;
    use server_core::event::EventBus;
    use server_core::{ConnectionId, Protocol};
    use std::net::SocketAddr;

    fn make_handler() -> (SessionHandler, Arc<SessionManager>) {
        let config = SessionConfig::default();
        let event_bus = Arc::new(EventBus::default());
        let manager = Arc::new(SessionManager::new(config, event_bus));
        let tracker = Arc::new(ConnectionTracker::new(100, 10));
        let handler = SessionHandler::new(Arc::clone(&manager), tracker);
        (handler, manager)
    }

    #[tokio::test]
    async fn test_session_created_on_connect() {
        let (handler, manager) = make_handler();

        let addr: SocketAddr = "127.0.0.1:5000".parse().unwrap();
        let info = ConnectionInfo::new(ConnectionId::new(), Protocol::Tcp, addr);
        let conn_id = info.id.clone();

        // Before connect: no sessions
        assert_eq!(manager.session_count(), 0);

        // Trigger on_connect
        handler.on_connect(&info).await.unwrap();

        // After connect: 1 session with 1 connection
        assert_eq!(manager.session_count(), 1);
        assert_eq!(manager.connection_count(), 1);

        let session_id = manager.get_session_by_connection(&conn_id).unwrap();
        let session = manager.get_session(&session_id).unwrap();
        assert!(session.has_primary());
        assert_eq!(session.connection_count(), 1);
    }
}
