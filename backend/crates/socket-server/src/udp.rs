use crate::handler::{ConnectionHandler, OutgoingMessage};
use crate::tracker::ConnectionTracker;
use server_config::model::UdpConfig;
use server_core::event::{EventBus, ServerEvent};
use server_core::types::*;
use server_core::Error;
use dashmap::DashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::net::UdpSocket;
use tokio::time::{Duration, Instant};
use tracing::{debug, error, info};

struct UdpSession {
    conn_id: ConnectionId,
    last_activity: Instant,
}

pub struct UdpServer {
    config: UdpConfig,
    bind_addr: SocketAddr,
    tracker: Arc<ConnectionTracker>,
    handler: Arc<dyn ConnectionHandler>,
    event_bus: Arc<EventBus>,
}

impl UdpServer {
    pub fn new(
        config: UdpConfig,
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

    /// Bind the UDP socket and start the receive loop in a background task.
    /// Returns the local address.
    pub async fn start(self, shutdown: ShutdownReceiver) -> server_core::Result<SocketAddr> {
        let socket = self.bind_socket()?;
        let addr = socket
            .local_addr()
            .map_err(|e| Error::Transport(e.to_string()))?;
        info!(addr = %addr, "UDP server listening");

        let tracker = self.tracker;
        let handler = self.handler;
        let event_bus = self.event_bus;
        let session_timeout = Duration::from_secs(self.config.session_timeout_secs);
        let max_packet_size = self.config.max_packet_size;

        tokio::spawn(async move {
            recv_loop(
                socket,
                tracker,
                handler,
                event_bus,
                session_timeout,
                max_packet_size,
                shutdown,
            )
            .await;
        });

        Ok(addr)
    }

    fn bind_socket(&self) -> server_core::Result<UdpSocket> {
        use socket2::{Domain, SockAddr, Socket, Type};

        let domain = if self.bind_addr.is_ipv4() {
            Domain::IPV4
        } else {
            Domain::IPV6
        };
        let socket = Socket::new(domain, Type::DGRAM, None)?;

        socket.set_reuse_address(true)?;
        socket.set_nonblocking(true)?;
        socket.set_recv_buffer_size(self.config.recv_buffer_size)?;
        socket.set_send_buffer_size(self.config.send_buffer_size)?;

        if self.config.broadcast_enabled {
            socket.set_broadcast(true)?;
        }

        socket.bind(&SockAddr::from(self.bind_addr))?;

        let std_socket = socket2_to_std_udp(socket);
        UdpSocket::from_std(std_socket).map_err(|e| Error::Transport(e.to_string()))
    }
}

/// Convert a socket2::Socket to std::net::UdpSocket via platform-specific owned handle.
fn socket2_to_std_udp(socket: socket2::Socket) -> std::net::UdpSocket {
    #[cfg(unix)]
    {
        use std::os::fd::OwnedFd;
        let fd: OwnedFd = socket.into();
        std::net::UdpSocket::from(fd)
    }
    #[cfg(windows)]
    {
        use std::os::windows::io::OwnedSocket;
        let s: OwnedSocket = socket.into();
        std::net::UdpSocket::from(s)
    }
}

async fn recv_loop(
    socket: UdpSocket,
    tracker: Arc<ConnectionTracker>,
    handler: Arc<dyn ConnectionHandler>,
    event_bus: Arc<EventBus>,
    session_timeout: Duration,
    max_packet_size: usize,
    mut shutdown: ShutdownReceiver,
) {
    let socket = Arc::new(socket);
    let sessions: DashMap<SocketAddr, UdpSession> = DashMap::new();
    let mut buf = vec![0u8; max_packet_size];
    let mut cleanup_interval = tokio::time::interval(Duration::from_secs(10));

    loop {
        tokio::select! {
            result = socket.recv_from(&mut buf) => {
                match result {
                    Ok((len, addr)) => {
                        handle_packet(
                            &buf[..len], addr, &socket, &sessions,
                            &tracker, &handler, &event_bus,
                        ).await;
                    }
                    Err(e) => {
                        error!(error = %e, "UDP recv error");
                    }
                }
            }
            _ = cleanup_interval.tick() => {
                cleanup_expired(
                    &sessions, &tracker, &handler, &event_bus, session_timeout,
                ).await;
            }
            _ = shutdown.recv() => {
                info!("UDP server shutting down");
                break;
            }
        }
    }
}

async fn handle_packet(
    data: &[u8],
    addr: SocketAddr,
    socket: &Arc<UdpSocket>,
    sessions: &DashMap<SocketAddr, UdpSession>,
    tracker: &Arc<ConnectionTracker>,
    handler: &Arc<dyn ConnectionHandler>,
    event_bus: &Arc<EventBus>,
) {
    // Update existing session or create new one
    let conn_id = if let Some(mut session) = sessions.get_mut(&addr) {
        session.last_activity = Instant::now();
        session.conn_id.clone()
    } else {
        // New virtual session
        let conn_id = ConnectionId::new();
        let info = ConnectionInfo::new(conn_id.clone(), Protocol::Udp, addr);

        let rx = match tracker.register(info.clone()) {
            Ok(rx) => rx,
            Err(e) => {
                debug!(addr = %addr, error = %e, "UDP session rejected");
                return;
            }
        };

        if let Err(e) = handler.on_connect(&info).await {
            debug!(addr = %addr, error = %e, "UDP session rejected by handler");
            tracker.unregister(&conn_id);
            return;
        }

        event_bus.publish(ServerEvent::ConnectionAccepted {
            connection_id: conn_id.clone(),
            protocol: Protocol::Udp,
            remote_addr: addr.to_string(),
        });

        tracker.update_state(&conn_id, ConnectionState::Established);

        // Spawn writer task: reads from the write channel, sends to client via socket
        let write_socket = Arc::clone(socket);
        tokio::spawn(async move {
            udp_write_task(rx, write_socket, addr).await;
        });

        sessions.insert(
            addr,
            UdpSession {
                conn_id: conn_id.clone(),
                last_activity: Instant::now(),
            },
        );

        conn_id
    };

    tracker.record_received(&conn_id, data.len() as u64);
    handler.on_data(&conn_id, data).await;
}

/// Writer task for a UDP session. Reads outgoing messages from the channel
/// and sends them to the client address via the shared UDP socket.
/// Exits when the sender is dropped (connection unregistered).
async fn udp_write_task(
    mut rx: tokio::sync::mpsc::Receiver<OutgoingMessage>,
    socket: Arc<UdpSocket>,
    addr: SocketAddr,
) {
    while let Some(msg) = rx.recv().await {
        let data = match msg {
            OutgoingMessage::Binary(d) => d,
            OutgoingMessage::Text(t) => t.into_bytes(),
            OutgoingMessage::Close => break,
            OutgoingMessage::Ping => continue,
        };
        if let Err(e) = socket.send_to(&data, addr).await {
            debug!(addr = %addr, error = %e, "UDP send error");
            break;
        }
    }
}

async fn cleanup_expired(
    sessions: &DashMap<SocketAddr, UdpSession>,
    tracker: &Arc<ConnectionTracker>,
    handler: &Arc<dyn ConnectionHandler>,
    event_bus: &Arc<EventBus>,
    timeout: Duration,
) {
    let expired: Vec<_> = sessions
        .iter()
        .filter(|entry| entry.last_activity.elapsed() > timeout)
        .map(|entry| (*entry.key(), entry.conn_id.clone()))
        .collect();

    for (addr, conn_id) in expired {
        sessions.remove(&addr);
        handler.on_disconnect(&conn_id, "session timeout").await;
        tracker.unregister(&conn_id);
        event_bus.publish(ServerEvent::ConnectionClosed {
            connection_id: conn_id,
            reason: "UDP session timeout".to_string(),
        });
        debug!(addr = %addr, "UDP session expired");
    }
}

// ─── Multicast ───────────────────────────────────────────────────────────────

/// Join an IPv4 multicast group on the given local interface.
///
/// The `socket` must have been created with `SO_REUSEADDR` and bound to the
/// multicast port before calling this function.
pub fn join_multicast(
    socket: &UdpSocket,
    multicast_addr: &Ipv4Addr,
    interface: &Ipv4Addr,
) -> server_core::Result<()> {
    multicast_op(socket, multicast_addr, interface, true)
}

/// Leave an IPv4 multicast group on the given local interface.
pub fn leave_multicast(
    socket: &UdpSocket,
    multicast_addr: &Ipv4Addr,
    interface: &Ipv4Addr,
) -> server_core::Result<()> {
    multicast_op(socket, multicast_addr, interface, false)
}

/// Internal helper that borrows the raw socket handle, calls the appropriate
/// `socket2` multicast method, then `mem::forget`s the wrapper so the fd is
/// not closed.
fn multicast_op(
    socket: &UdpSocket,
    multicast_addr: &Ipv4Addr,
    interface: &Ipv4Addr,
    join: bool,
) -> server_core::Result<()> {
    use socket2::Socket;

    // Build a temporary socket2::Socket from the raw handle without taking
    // ownership (we call mem::forget before it drops).
    #[cfg(unix)]
    let sock = {
        use std::os::fd::{AsRawFd, FromRawFd};
        // SAFETY: fd is valid for the lifetime of `socket`.
        unsafe { Socket::from_raw_fd(socket.as_raw_fd()) }
    };
    #[cfg(windows)]
    let sock = {
        use std::os::windows::io::{AsRawSocket, FromRawSocket};
        // SAFETY: socket handle is valid for the lifetime of `socket`.
        unsafe { Socket::from_raw_socket(socket.as_raw_socket()) }
    };

    let result = if join {
        sock.join_multicast_v4(multicast_addr, interface)
            .map_err(|e| Error::Transport(format!("join_multicast: {e}")))
    } else {
        sock.leave_multicast_v4(multicast_addr, interface)
            .map_err(|e| Error::Transport(format!("leave_multicast: {e}")))
    };

    // Do NOT let sock drop — that would close the fd we don't own.
    std::mem::forget(sock);
    result
}

// ─── Per-Source Rate Limiter ──────────────────────────────────────────────────

/// Per-source-address UDP packet rate limiter using a sliding-window approach.
///
/// Each unique remote address gets its own `(packet_count, window_start)` entry.
/// `check_rate` returns `true` if the packet should be allowed, `false` if it
/// exceeds `max_packets_per_sec`.  Call `cleanup` periodically to remove stale
/// entries for sources that have gone silent.
pub struct UdpRateLimiter {
    /// map: remote addr → (packets in current window, window start timestamp)
    state: DashMap<SocketAddr, (AtomicU64, std::time::Instant)>,
    max_packets_per_sec: u64,
}

impl UdpRateLimiter {
    pub fn new(max_packets_per_sec: u64) -> Self {
        Self {
            state: DashMap::new(),
            max_packets_per_sec,
        }
    }

    /// Returns `true` if the packet from `addr` is within the allowed rate,
    /// `false` if it should be dropped.
    pub fn check_rate(&self, addr: SocketAddr) -> bool {
        let now = std::time::Instant::now();

        // Fast path: entry already exists.
        if let Some(entry) = self.state.get(&addr) {
            let (counter, window_start) = &*entry;
            if now.duration_since(*window_start) < std::time::Duration::from_secs(1) {
                // Still within the same 1-second window.
                let prev = counter.fetch_add(1, Ordering::Relaxed);
                return prev < self.max_packets_per_sec;
            }
            // Window expired — reset below (needs write access).
            drop(entry);
        }

        // Insert or reset the entry for this address.
        self.state
            .entry(addr)
            .and_modify(|e| {
                // Reset if the window has expired.
                if now.duration_since(e.1) >= std::time::Duration::from_secs(1) {
                    e.0.store(1, Ordering::Relaxed);
                    e.1 = now;
                } else {
                    e.0.fetch_add(1, Ordering::Relaxed);
                }
            })
            .or_insert_with(|| (AtomicU64::new(1), now));

        // After insert/modify, re-read the counter to decide.
        if let Some(entry) = self.state.get(&addr) {
            return entry.0.load(Ordering::Relaxed) <= self.max_packets_per_sec;
        }
        true
    }

    /// Remove entries whose windows are older than `max_age`.  Call this on a
    /// periodic timer (e.g. every 30 seconds) to reclaim memory for silent
    /// sources.
    pub fn cleanup(&self) {
        let now = std::time::Instant::now();
        self.state
            .retain(|_, (_, window_start)| {
                now.duration_since(*window_start) < std::time::Duration::from_secs(60)
            });
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::tests::NoopHandler;

    #[tokio::test]
    async fn test_udp_server_start_and_send() {
        let tracker = Arc::new(ConnectionTracker::new(100, 10));
        let handler: Arc<dyn ConnectionHandler> = Arc::new(NoopHandler);
        let event_bus = Arc::new(EventBus::new(16));
        let (shutdown, shutdown_rx) = ShutdownSignal::new();

        let mut config = UdpConfig::default();
        config.port = 0;

        let server = UdpServer::new(
            config,
            "127.0.0.1",
            Arc::clone(&tracker),
            handler,
            event_bus,
        );
        let addr = server.start(shutdown_rx).await.unwrap();

        // Send a packet from a client
        let client = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        client.send_to(b"hello UDP", addr).await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Virtual session should be created
        assert_eq!(tracker.count(), 1);

        // Send another packet from the same source — should reuse session
        client.send_to(b"again", addr).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(tracker.count(), 1);

        shutdown.shutdown();
    }

    #[tokio::test]
    async fn test_udp_send_response() {
        let tracker = Arc::new(ConnectionTracker::new(100, 10));
        let handler: Arc<dyn ConnectionHandler> = Arc::new(NoopHandler);
        let event_bus = Arc::new(EventBus::new(16));
        let (shutdown, shutdown_rx) = ShutdownSignal::new();

        let mut config = UdpConfig::default();
        config.port = 0;

        let server = UdpServer::new(
            config,
            "127.0.0.1",
            Arc::clone(&tracker),
            handler,
            event_bus,
        );
        let addr = server.start(shutdown_rx).await.unwrap();

        // Send a packet to create the session
        let client = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        client.send_to(b"ping", addr).await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Send response via tracker
        let conns = tracker.connections();
        assert_eq!(conns.len(), 1);
        let conn_id = conns[0].id.clone();

        tracker
            .send(&conn_id, OutgoingMessage::Binary(b"pong".to_vec()))
            .await
            .unwrap();

        // Read response on the client
        let mut buf = vec![0u8; 64];
        let (n, _) = tokio::time::timeout(Duration::from_secs(2), client.recv_from(&mut buf))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(&buf[..n], b"pong");

        shutdown.shutdown();
    }

    // ── Rate limiter tests ────────────────────────────────────────────────────

    #[test]
    fn test_rate_limiter_allows_within_limit() {
        let limiter = UdpRateLimiter::new(5);
        let addr: SocketAddr = "127.0.0.1:9000".parse().unwrap();

        for _ in 0..5 {
            assert!(limiter.check_rate(addr), "should be allowed within limit");
        }
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let limiter = UdpRateLimiter::new(3);
        let addr: SocketAddr = "127.0.0.1:9001".parse().unwrap();

        for _ in 0..3 {
            assert!(limiter.check_rate(addr));
        }
        // The 4th packet in the same window should be blocked.
        assert!(!limiter.check_rate(addr), "should be blocked over limit");
    }

    #[test]
    fn test_rate_limiter_independent_sources() {
        let limiter = UdpRateLimiter::new(2);
        let a: SocketAddr = "10.0.0.1:100".parse().unwrap();
        let b: SocketAddr = "10.0.0.2:100".parse().unwrap();

        // Both sources are independent; each gets its own window.
        assert!(limiter.check_rate(a));
        assert!(limiter.check_rate(b));
        assert!(limiter.check_rate(a));
        assert!(limiter.check_rate(b));

        // Third packet from each should be blocked.
        assert!(!limiter.check_rate(a));
        assert!(!limiter.check_rate(b));
    }

    #[test]
    fn test_rate_limiter_cleanup_does_not_panic() {
        let limiter = UdpRateLimiter::new(10);
        let addr: SocketAddr = "127.0.0.1:9002".parse().unwrap();
        limiter.check_rate(addr);
        // Should not panic even when the map has entries.
        limiter.cleanup();
    }

    // ── Multicast helper smoke tests ─────────────────────────────────────────

    #[test]
    fn test_multicast_addr_is_multicast() {
        let addr: Ipv4Addr = "224.0.0.1".parse().unwrap();
        assert!(addr.is_multicast(), "224.0.0.1 must be a multicast address");
    }

    #[test]
    fn test_multicast_non_multicast_addr() {
        let addr: Ipv4Addr = "192.168.1.1".parse().unwrap();
        assert!(!addr.is_multicast(), "192.168.1.1 is not multicast");
    }
}
