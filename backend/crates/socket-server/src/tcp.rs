use crate::handler::{ConnectionHandler, OutgoingMessage};
use crate::tracker::ConnectionTracker;
use server_config::model::TcpConfig;
use server_core::event::{EventBus, ServerEvent};
use server_core::types::*;
use server_core::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpSocket};
use tokio::sync::mpsc;
use tokio::time::{self, Duration};
use tracing::{debug, error, info};

pub struct TcpServer {
    config: TcpConfig,
    bind_addr: SocketAddr,
    tracker: Arc<ConnectionTracker>,
    handler: Arc<dyn ConnectionHandler>,
    event_bus: Arc<EventBus>,
}

impl TcpServer {
    pub fn new(
        config: TcpConfig,
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

    /// Bind the TCP listener and start accepting connections in a background task.
    /// Returns the local address the server is listening on.
    pub async fn start(self, shutdown: ShutdownReceiver) -> server_core::Result<SocketAddr> {
        let listener = self.bind_listener()?;
        let addr = listener
            .local_addr()
            .map_err(|e| Error::Transport(e.to_string()))?;
        info!(addr = %addr, "TCP server listening");

        let config = self.config;
        let tracker = self.tracker;
        let handler = self.handler;
        let event_bus = self.event_bus;

        tokio::spawn(async move {
            accept_loop(listener, config, tracker, handler, event_bus, shutdown).await;
        });

        Ok(addr)
    }

    fn bind_listener(&self) -> server_core::Result<TcpListener> {
        let socket = if self.bind_addr.is_ipv4() {
            TcpSocket::new_v4()
        } else {
            TcpSocket::new_v6()
        }
        .map_err(|e| Error::Transport(format!("failed to create TCP socket: {e}")))?;

        socket
            .set_reuseaddr(true)
            .map_err(|e| Error::Transport(format!("set_reuseaddr: {e}")))?;

        if self.config.recv_buffer_size > 0 {
            socket
                .set_recv_buffer_size(self.config.recv_buffer_size as u32)
                .map_err(|e| Error::Transport(format!("set_recv_buffer_size: {e}")))?;
        }
        if self.config.send_buffer_size > 0 {
            socket
                .set_send_buffer_size(self.config.send_buffer_size as u32)
                .map_err(|e| Error::Transport(format!("set_send_buffer_size: {e}")))?;
        }

        socket
            .bind(self.bind_addr)
            .map_err(|e| Error::Transport(format!("TCP bind {}: {e}", self.bind_addr)))?;

        socket
            .listen(self.config.backlog)
            .map_err(|e| Error::Transport(format!("TCP listen: {e}")))
    }
}

async fn accept_loop(
    listener: TcpListener,
    config: TcpConfig,
    tracker: Arc<ConnectionTracker>,
    handler: Arc<dyn ConnectionHandler>,
    event_bus: Arc<EventBus>,
    mut shutdown: ShutdownReceiver,
) {
    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, addr)) => {
                        // Set per-connection socket options
                        if config.nodelay {
                            let _ = stream.set_nodelay(true);
                        }

                        let conn_id = ConnectionId::new();
                        let info = ConnectionInfo::new(conn_id.clone(), Protocol::Tcp, addr);

                        // Register in tracker (checks limits)
                        let rx = match tracker.register(info.clone()) {
                            Ok(rx) => rx,
                            Err(e) => {
                                debug!(addr = %addr, error = %e, "TCP connection rejected (limit)");
                                continue;
                            }
                        };

                        // Ask handler to approve (traffic guard can reject)
                        if let Err(e) = handler.on_connect(&info).await {
                            debug!(addr = %addr, error = %e, "TCP connection rejected (handler)");
                            tracker.unregister(&conn_id);
                            continue;
                        }

                        event_bus.publish(ServerEvent::ConnectionAccepted {
                            connection_id: conn_id.clone(),
                            protocol: Protocol::Tcp,
                            remote_addr: addr.to_string(),
                        });

                        tracker.update_state(&conn_id, ConnectionState::Established);

                        // Spawn connection task
                        let tracker = Arc::clone(&tracker);
                        let handler = Arc::clone(&handler);
                        let event_bus = Arc::clone(&event_bus);
                        let idle_timeout = Duration::from_secs(config.idle_timeout_secs);

                        tokio::spawn(async move {
                            connection_task(
                                stream, conn_id, addr, rx,
                                tracker, handler, event_bus, idle_timeout,
                            ).await;
                        });
                    }
                    Err(e) => {
                        error!(error = %e, "TCP accept error");
                    }
                }
            }
            _ = shutdown.recv() => {
                info!("TCP server shutting down");
                break;
            }
        }
    }
}

async fn connection_task(
    stream: tokio::net::TcpStream,
    conn_id: ConnectionId,
    addr: SocketAddr,
    mut outgoing_rx: mpsc::Receiver<OutgoingMessage>,
    tracker: Arc<ConnectionTracker>,
    handler: Arc<dyn ConnectionHandler>,
    event_bus: Arc<EventBus>,
    idle_timeout: Duration,
) {
    let (mut reader, mut writer) = stream.into_split();
    let mut buf = vec![0u8; 8192];

    let reason = loop {
        tokio::select! {
            result = time::timeout(idle_timeout, reader.read(&mut buf)) => {
                match result {
                    Ok(Ok(0)) => break "closed by peer".to_string(),
                    Ok(Ok(n)) => {
                        tracker.record_received(&conn_id, n as u64);
                        handler.on_data(&conn_id, &buf[..n]).await;
                    }
                    Ok(Err(e)) => {
                        handler.on_error(&conn_id, &Error::Connection(e.to_string())).await;
                        break format!("read error: {e}");
                    }
                    Err(_) => break "idle timeout".to_string(),
                }
            }
            msg = outgoing_rx.recv() => {
                match msg {
                    Some(OutgoingMessage::Binary(data)) => {
                        let len = data.len() as u64;
                        if let Err(e) = writer.write_all(&data).await {
                            handler.on_error(&conn_id, &Error::Connection(e.to_string())).await;
                            break format!("write error: {e}");
                        }
                        tracker.record_sent(&conn_id, len);
                    }
                    Some(OutgoingMessage::Text(text)) => {
                        let len = text.len() as u64;
                        if let Err(e) = writer.write_all(text.as_bytes()).await {
                            handler.on_error(&conn_id, &Error::Connection(e.to_string())).await;
                            break format!("write error: {e}");
                        }
                        tracker.record_sent(&conn_id, len);
                    }
                    Some(OutgoingMessage::Close) | None => break "close requested".to_string(),
                    Some(OutgoingMessage::Ping) => {} // No-op for TCP
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

    debug!(addr = %addr, "TCP connection closed");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::tests::NoopHandler;

    #[tokio::test]
    async fn test_tcp_server_start_and_connect() {
        let tracker = Arc::new(ConnectionTracker::new(100, 10));
        let handler: Arc<dyn ConnectionHandler> = Arc::new(NoopHandler);
        let event_bus = Arc::new(EventBus::new(16));
        let (shutdown, shutdown_rx) = ShutdownSignal::new();

        let mut config = TcpConfig::default();
        config.port = 0; // Random port

        let server = TcpServer::new(
            config,
            "127.0.0.1",
            Arc::clone(&tracker),
            handler,
            event_bus,
        );
        let addr = server.start(shutdown_rx).await.unwrap();

        // Connect a client
        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(tracker.count(), 1);

        // Send data
        stream.write_all(b"hello").await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Close
        drop(stream);
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(tracker.count(), 0);

        shutdown.shutdown();
    }

    #[tokio::test]
    async fn test_tcp_send_to_client() {
        let tracker = Arc::new(ConnectionTracker::new(100, 10));
        let handler: Arc<dyn ConnectionHandler> = Arc::new(NoopHandler);
        let event_bus = Arc::new(EventBus::new(16));
        let (shutdown, shutdown_rx) = ShutdownSignal::new();

        let mut config = TcpConfig::default();
        config.port = 0;

        let server = TcpServer::new(
            config,
            "127.0.0.1",
            Arc::clone(&tracker),
            handler,
            event_bus,
        );
        let addr = server.start(shutdown_rx).await.unwrap();

        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Find the connection ID and send data through tracker
        let conns = tracker.connections();
        assert_eq!(conns.len(), 1);
        let conn_id = conns[0].id.clone();

        tracker
            .send(&conn_id, OutgoingMessage::Binary(b"world".to_vec()))
            .await
            .unwrap();

        // Read it on the client side
        let mut buf = vec![0u8; 64];
        let n = stream.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"world");

        shutdown.shutdown();
    }
}
