use crate::handler::{ConnectionHandler, OutgoingMessage};
use crate::tracker::ConnectionTracker;
use axum::extract::connect_info::ConnectInfo;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use futures_util::{SinkExt, StreamExt};
use server_config::model::WebSocketConfig;
use server_core::event::{EventBus, ServerEvent};
use server_core::types::*;
use server_core::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{self, Duration};
use tracing::{debug, info};

pub struct WsServer {
    config: WebSocketConfig,
    bind_addr: SocketAddr,
    tracker: Arc<ConnectionTracker>,
    handler: Arc<dyn ConnectionHandler>,
    event_bus: Arc<EventBus>,
}

impl WsServer {
    pub fn new(
        config: WebSocketConfig,
        host: &str,
        tracker: Arc<ConnectionTracker>,
        handler: Arc<dyn ConnectionHandler>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        let bind_addr: SocketAddr = format!("{host}:{}", config.port)
            .parse()
            .unwrap_or_else(|_| SocketAddr::from(([0, 0, 0, 0], config.port)));
        Self {
            config,
            bind_addr,
            tracker,
            handler,
            event_bus,
        }
    }

    /// Bind the WebSocket server and start handling upgrades in a background task.
    /// Returns the local address.
    pub async fn start(self, shutdown: ShutdownReceiver) -> server_core::Result<SocketAddr> {
        let tracker = self.tracker;
        let handler = self.handler;
        let event_bus = self.event_bus;
        let config = self.config.clone();
        let path = self.config.path.clone();

        let app = Router::new().route(
            &path,
            get({
                let tracker = Arc::clone(&tracker);
                let handler = Arc::clone(&handler);
                let event_bus = Arc::clone(&event_bus);
                let config = config.clone();
                move |ws: WebSocketUpgrade, ConnectInfo(addr): ConnectInfo<SocketAddr>| {
                    let tracker = Arc::clone(&tracker);
                    let handler = Arc::clone(&handler);
                    let event_bus = Arc::clone(&event_bus);
                    let config = config.clone();
                    async move { upgrade_handler(ws, addr, tracker, handler, event_bus, config) }
                }
            }),
        );

        let listener = tokio::net::TcpListener::bind(self.bind_addr)
            .await
            .map_err(|e| Error::Transport(format!("WS bind {}: {e}", self.bind_addr)))?;
        let addr = listener
            .local_addr()
            .map_err(|e| Error::Transport(e.to_string()))?;
        info!(addr = %addr, path = %self.config.path, "WebSocket server listening");

        let mut shutdown = shutdown;
        tokio::spawn(async move {
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(async move {
                shutdown.recv().await;
            })
            .await
            .ok();
        });

        Ok(addr)
    }
}

fn upgrade_handler(
    ws: WebSocketUpgrade,
    addr: SocketAddr,
    tracker: Arc<ConnectionTracker>,
    handler: Arc<dyn ConnectionHandler>,
    event_bus: Arc<EventBus>,
    config: WebSocketConfig,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| {
        handle_ws_connection(socket, addr, tracker, handler, event_bus, config)
    })
}

async fn handle_ws_connection(
    socket: WebSocket,
    addr: SocketAddr,
    tracker: Arc<ConnectionTracker>,
    handler: Arc<dyn ConnectionHandler>,
    event_bus: Arc<EventBus>,
    config: WebSocketConfig,
) {
    let conn_id = ConnectionId::new();
    let info = ConnectionInfo::new(conn_id.clone(), Protocol::WebSocket, addr);

    let rx = match tracker.register(info.clone()) {
        Ok(rx) => rx,
        Err(e) => {
            debug!(addr = %addr, error = %e, "WS connection rejected");
            return;
        }
    };

    if let Err(e) = handler.on_connect(&info).await {
        debug!(addr = %addr, error = %e, "WS connection rejected by handler");
        tracker.unregister(&conn_id);
        return;
    }

    event_bus.publish(ServerEvent::ConnectionAccepted {
        connection_id: conn_id.clone(),
        protocol: Protocol::WebSocket,
        remote_addr: addr.to_string(),
    });

    tracker.update_state(&conn_id, ConnectionState::Established);

    ws_connection_task(socket, conn_id, addr, rx, tracker, handler, event_bus, config).await;
}

async fn ws_connection_task(
    socket: WebSocket,
    conn_id: ConnectionId,
    addr: SocketAddr,
    mut outgoing_rx: mpsc::Receiver<OutgoingMessage>,
    tracker: Arc<ConnectionTracker>,
    handler: Arc<dyn ConnectionHandler>,
    event_bus: Arc<EventBus>,
    config: WebSocketConfig,
) {
    let (mut sender, mut receiver) = socket.split();
    let ping_interval = Duration::from_secs(config.ping_interval_secs);
    let pong_timeout = Duration::from_secs(config.pong_timeout_secs);
    let max_message_size = config.max_message_size;

    let mut ping_timer = time::interval(ping_interval);
    let mut waiting_for_pong = false;
    let mut pong_deadline: Option<time::Instant> = None;

    let reason = loop {
        tokio::select! {
            // Receive from WebSocket
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        let s = text.to_string();
                        if s.len() > max_message_size {
                            break "message too large".to_string();
                        }
                        tracker.record_received(&conn_id, s.len() as u64);
                        handler.on_text(&conn_id, &s).await;
                    }
                    Some(Ok(Message::Binary(data))) => {
                        if data.len() > max_message_size {
                            break "message too large".to_string();
                        }
                        tracker.record_received(&conn_id, data.len() as u64);
                        handler.on_data(&conn_id, &data).await;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        if sender.send(Message::Pong(data)).await.is_err() {
                            break "send pong failed".to_string();
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {
                        waiting_for_pong = false;
                        pong_deadline = None;
                    }
                    Some(Ok(Message::Close(_))) => break "closed by peer".to_string(),
                    Some(Err(e)) => break format!("ws error: {e}"),
                    None => break "connection ended".to_string(),
                }
            }

            // Send outgoing messages
            msg = outgoing_rx.recv() => {
                match msg {
                    Some(OutgoingMessage::Binary(data)) => {
                        let len = data.len() as u64;
                        if sender.send(Message::Binary(data.into())).await.is_err() {
                            break "send failed".to_string();
                        }
                        tracker.record_sent(&conn_id, len);
                    }
                    Some(OutgoingMessage::Text(text)) => {
                        let len = text.len() as u64;
                        if sender.send(Message::Text(text.into())).await.is_err() {
                            break "send failed".to_string();
                        }
                        tracker.record_sent(&conn_id, len);
                    }
                    Some(OutgoingMessage::Ping) => {
                        if sender.send(Message::Ping(vec![].into())).await.is_err() {
                            break "send ping failed".to_string();
                        }
                    }
                    Some(OutgoingMessage::Close) | None => break "close requested".to_string(),
                }
            }

            // Ping/pong heartbeat
            _ = ping_timer.tick() => {
                if waiting_for_pong {
                    if let Some(deadline) = pong_deadline {
                        if deadline.elapsed() > pong_timeout {
                            break "pong timeout".to_string();
                        }
                    }
                } else {
                    if sender.send(Message::Ping(vec![].into())).await.is_err() {
                        break "send ping failed".to_string();
                    }
                    waiting_for_pong = true;
                    pong_deadline = Some(time::Instant::now());
                }
            }
        }
    };

    // Cleanup
    tracker.update_state(&conn_id, ConnectionState::Closing);
    handler.on_disconnect(&conn_id, &reason).await;
    tracker.unregister(&conn_id);

    event_bus.publish(ServerEvent::ConnectionClosed {
        connection_id: conn_id,
        reason,
    });

    debug!(addr = %addr, "WS connection closed");
}

// ─── Subprotocol Negotiation ─────────────────────────────────────────────────

/// WebSocket subprotocol negotiator.
///
/// Holds the list of subprotocols supported by the server.  During the
/// WebSocket handshake the client advertises which protocols it understands
/// via the `Sec-WebSocket-Protocol` header.  `negotiate` returns the first
/// server-supported protocol that also appears in the client's list (priority
/// is given to the server's preference order).
pub struct SubprotocolNegotiator {
    supported: Vec<String>,
}

impl SubprotocolNegotiator {
    /// Create a new negotiator with the given list of supported subprotocols.
    /// The order matters: the first match wins.
    pub fn new(supported: Vec<String>) -> Self {
        Self { supported }
    }

    /// Returns the first subprotocol from `self.supported` that is also
    /// present in `requested`, or `None` if there is no overlap.
    pub fn negotiate(&self, requested: &[String]) -> Option<String> {
        self.supported
            .iter()
            .find(|s| requested.iter().any(|r| r == *s))
            .cloned()
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::tests::NoopHandler;
    use tokio_tungstenite::connect_async;

    #[tokio::test]
    async fn test_ws_server_start_and_connect() {
        let tracker = Arc::new(ConnectionTracker::new(100, 10));
        let handler: Arc<dyn ConnectionHandler> = Arc::new(NoopHandler);
        let event_bus = Arc::new(EventBus::new(16));
        let (shutdown, shutdown_rx) = ShutdownSignal::new();

        let mut config = WebSocketConfig::default();
        config.port = 0;

        let server = WsServer::new(
            config,
            "127.0.0.1",
            Arc::clone(&tracker),
            handler,
            event_bus,
        );
        let addr = server.start(shutdown_rx).await.unwrap();

        // Connect a WebSocket client
        let url = format!("ws://{addr}/ws");
        let (ws_stream, _) = connect_async(&url).await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;

        assert_eq!(tracker.count(), 1);

        // Close
        drop(ws_stream);
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_eq!(tracker.count(), 0);

        shutdown.shutdown();
    }

    // ── SubprotocolNegotiator tests ───────────────────────────────────────────

    #[test]
    fn test_negotiate_returns_first_server_match() {
        let neg = SubprotocolNegotiator::new(vec![
            "chat".to_string(),
            "v2".to_string(),
        ]);
        let requested = vec!["v2".to_string(), "chat".to_string()];
        // Server prefers "chat", so it should win even though client listed "v2" first.
        assert_eq!(neg.negotiate(&requested), Some("chat".to_string()));
    }

    #[test]
    fn test_negotiate_no_overlap_returns_none() {
        let neg = SubprotocolNegotiator::new(vec!["binary".to_string()]);
        let requested = vec!["json".to_string(), "text".to_string()];
        assert_eq!(neg.negotiate(&requested), None);
    }

    #[test]
    fn test_negotiate_empty_requested_returns_none() {
        let neg = SubprotocolNegotiator::new(vec!["chat".to_string()]);
        assert_eq!(neg.negotiate(&[]), None);
    }

    #[test]
    fn test_negotiate_empty_supported_returns_none() {
        let neg = SubprotocolNegotiator::new(vec![]);
        let requested = vec!["chat".to_string()];
        assert_eq!(neg.negotiate(&requested), None);
    }
}
