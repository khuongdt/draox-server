use crate::handler::ConnectionHandler;
use crate::http::HttpServer;
use crate::tcp::TcpServer;
use crate::tracker::ConnectionTracker;
use crate::udp::UdpServer;
use crate::ws::WsServer;
use server_config::DraoxConfig;
use server_core::event::EventBus;
use server_core::types::ShutdownSignal;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

/// Addresses of all started protocol listeners.
#[derive(Debug, Default)]
pub struct ListenerAddresses {
    pub tcp: Option<SocketAddr>,
    pub udp: Option<SocketAddr>,
    pub ws: Option<SocketAddr>,
    pub http: Option<SocketAddr>,
}

/// Orchestrates all protocol servers (TCP, UDP, WebSocket, HTTP).
///
/// Creates a shared `ConnectionTracker` and starts each enabled server
/// in a background task.
pub struct MultiProtocolListener {
    config: Arc<DraoxConfig>,
    tracker: Arc<ConnectionTracker>,
    handler: Arc<dyn ConnectionHandler>,
    event_bus: Arc<EventBus>,
}

impl MultiProtocolListener {
    pub fn new(
        config: Arc<DraoxConfig>,
        handler: Arc<dyn ConnectionHandler>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        let tracker = Arc::new(ConnectionTracker::new(
            config.server.max_connections,
            config.traffic_guard.connection_limits.max_connections_per_ip,
        ));

        Self {
            config,
            tracker,
            handler,
            event_bus,
        }
    }

    /// Create with an externally-provided `ConnectionTracker`.
    ///
    /// Use this when the tracker must be shared with other components
    /// (e.g. `SessionHandler`, `AppState`) that are created before the
    /// listener.
    pub fn with_tracker(
        config: Arc<DraoxConfig>,
        tracker: Arc<ConnectionTracker>,
        handler: Arc<dyn ConnectionHandler>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        Self {
            config,
            tracker,
            handler,
            event_bus,
        }
    }

    /// Get a reference to the shared connection tracker.
    pub fn tracker(&self) -> &Arc<ConnectionTracker> {
        &self.tracker
    }

    /// Start all enabled protocol listeners.
    /// Returns the bound addresses of each started server.
    pub async fn start(
        &self,
        shutdown: &ShutdownSignal,
    ) -> server_core::Result<ListenerAddresses> {
        let mut addrs = ListenerAddresses::default();
        let host = &self.config.server.host;

        // TCP
        if self.config.tcp.enabled {
            let tcp = TcpServer::new(
                self.config.tcp.clone(),
                host,
                Arc::clone(&self.tracker),
                Arc::clone(&self.handler),
                Arc::clone(&self.event_bus),
            );
            addrs.tcp = Some(tcp.start(shutdown.subscribe()).await?);
        }

        // UDP
        if self.config.udp.enabled {
            let udp = UdpServer::new(
                self.config.udp.clone(),
                host,
                Arc::clone(&self.tracker),
                Arc::clone(&self.handler),
                Arc::clone(&self.event_bus),
            );
            addrs.udp = Some(udp.start(shutdown.subscribe()).await?);
        }

        // WebSocket
        if self.config.websocket.enabled {
            let ws = WsServer::new(
                self.config.websocket.clone(),
                host,
                Arc::clone(&self.tracker),
                Arc::clone(&self.handler),
                Arc::clone(&self.event_bus),
            );
            addrs.ws = Some(ws.start(shutdown.subscribe()).await?);
        }

        // HTTP
        if self.config.http.enabled {
            let http = HttpServer::new(self.config.http.clone(), host);
            addrs.http = Some(http.start(shutdown.subscribe()).await?);
        }

        info!(
            tcp = ?addrs.tcp,
            udp = ?addrs.udp,
            ws = ?addrs.ws,
            http = ?addrs.http,
            "all protocol listeners started"
        );
        Ok(addrs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::tests::NoopHandler;

    fn test_config() -> DraoxConfig {
        let mut config = DraoxConfig::default();
        config.tcp.port = 0;
        config.udp.port = 0;
        config.websocket.port = 0;
        config.http.port = 0;
        config.server.host = "127.0.0.1".to_string();
        config
    }

    #[tokio::test]
    async fn test_start_all_listeners() {
        let config = Arc::new(test_config());
        let handler: Arc<dyn ConnectionHandler> = Arc::new(NoopHandler);
        let event_bus = Arc::new(EventBus::new(16));
        let (shutdown, _) = ShutdownSignal::new();

        let listener = MultiProtocolListener::new(config, handler, event_bus);
        let addrs = listener.start(&shutdown).await.unwrap();

        assert!(addrs.tcp.is_some());
        assert!(addrs.udp.is_some());
        assert!(addrs.ws.is_some());
        assert!(addrs.http.is_some());

        shutdown.shutdown();
    }

    #[tokio::test]
    async fn test_disabled_protocols() {
        let mut config = DraoxConfig::default();
        config.tcp.enabled = false;
        config.udp.enabled = false;
        config.websocket.enabled = false;
        config.http.port = 0;
        config.server.host = "127.0.0.1".to_string();

        let config = Arc::new(config);
        let handler: Arc<dyn ConnectionHandler> = Arc::new(NoopHandler);
        let event_bus = Arc::new(EventBus::new(16));
        let (shutdown, _) = ShutdownSignal::new();

        let listener = MultiProtocolListener::new(config, handler, event_bus);
        let addrs = listener.start(&shutdown).await.unwrap();

        assert!(addrs.tcp.is_none());
        assert!(addrs.udp.is_none());
        assert!(addrs.ws.is_none());
        assert!(addrs.http.is_some());

        shutdown.shutdown();
    }
}
