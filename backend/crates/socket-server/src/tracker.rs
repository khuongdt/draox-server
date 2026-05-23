use crate::handler::{OutgoingMessage, WriteSender};
use dashmap::DashMap;
use server_core::types::*;
use server_core::{Error, Result};
use std::net::{IpAddr, SocketAddr};
use tokio::sync::mpsc;
use tracing::debug;

const WRITE_CHANNEL_SIZE: usize = 256;

/// Internal entry stored per connection.
struct ConnectionEntry {
    info: ConnectionInfo,
    writer: WriteSender,
}

/// Tracks all active connections across all protocols.
///
/// Provides per-IP limits, global limits, and a write channel per connection
/// so higher layers can send data via `tracker.send(conn_id, msg)`.
pub struct ConnectionTracker {
    connections: DashMap<ConnectionId, ConnectionEntry>,
    addr_to_id: DashMap<SocketAddr, ConnectionId>,
    ip_counts: DashMap<IpAddr, usize>,
    max_connections: usize,
    max_per_ip: u32,
}

impl ConnectionTracker {
    pub fn new(max_connections: usize, max_per_ip: u32) -> Self {
        Self {
            connections: DashMap::new(),
            addr_to_id: DashMap::new(),
            ip_counts: DashMap::new(),
            max_connections,
            max_per_ip,
        }
    }

    /// Register a new connection.
    ///
    /// Returns an `mpsc::Receiver<OutgoingMessage>` that the connection task
    /// should read from to send data back to the client. The corresponding
    /// sender is stored internally and used by `send()`.
    pub fn register(&self, info: ConnectionInfo) -> Result<mpsc::Receiver<OutgoingMessage>> {
        // Check global limit
        if self.connections.len() >= self.max_connections {
            return Err(Error::MaxConnectionsReached {
                max: self.max_connections,
            });
        }

        // Check per-IP limit
        let ip = info.remote_addr.ip();
        let current = self.ip_counts.get(&ip).map(|r| *r.value()).unwrap_or(0);
        if current >= self.max_per_ip as usize {
            return Err(Error::ConnectionRefused {
                addr: info.remote_addr.to_string(),
                reason: format!("per-IP limit exceeded ({current}/{})", self.max_per_ip),
            });
        }

        let (tx, rx) = mpsc::channel(WRITE_CHANNEL_SIZE);
        let conn_id = info.id.clone();
        let addr = info.remote_addr;

        self.connections
            .insert(conn_id.clone(), ConnectionEntry { info, writer: tx });
        self.addr_to_id.insert(addr, conn_id);
        self.ip_counts
            .entry(ip)
            .and_modify(|c| *c += 1)
            .or_insert(1);

        debug!(ip = %ip, count = current + 1, "connection registered");
        Ok(rx)
    }

    /// Unregister a connection. Returns its info if it existed.
    pub fn unregister(&self, id: &ConnectionId) -> Option<ConnectionInfo> {
        if let Some((_, entry)) = self.connections.remove(id) {
            let ip = entry.info.remote_addr.ip();
            self.addr_to_id.remove(&entry.info.remote_addr);

            // Decrement IP count, remove entry if zero
            let remove_ip = if let Some(mut count) = self.ip_counts.get_mut(&ip) {
                *count = count.saturating_sub(1);
                *count == 0
            } else {
                false
            };
            if remove_ip {
                self.ip_counts.remove(&ip);
            }

            debug!(id = %id, ip = %ip, "connection unregistered");
            Some(entry.info)
        } else {
            None
        }
    }

    /// Get connection info by ID (cloned snapshot).
    pub fn get(&self, id: &ConnectionId) -> Option<ConnectionInfo> {
        self.connections.get(id).map(|e| e.info.clone())
    }

    /// Get connection ID by socket address.
    pub fn get_by_addr(&self, addr: &SocketAddr) -> Option<ConnectionId> {
        self.addr_to_id.get(addr).map(|r| r.value().clone())
    }

    /// Send an outgoing message to a connection.
    pub async fn send(&self, id: &ConnectionId, msg: OutgoingMessage) -> Result<()> {
        if let Some(entry) = self.connections.get(id) {
            entry
                .writer
                .send(msg)
                .await
                .map_err(|_| Error::Connection(format!("send failed for connection {id}")))
        } else {
            Err(Error::Connection(format!("connection not found: {id}")))
        }
    }

    /// Update a connection's state.
    pub fn update_state(&self, id: &ConnectionId, state: ConnectionState) {
        if let Some(mut entry) = self.connections.get_mut(id) {
            entry.info.state = state;
        }
    }

    /// Record bytes received on a connection.
    pub fn record_received(&self, id: &ConnectionId, bytes: u64) {
        if let Some(mut entry) = self.connections.get_mut(id) {
            entry.info.bytes_received += bytes;
            entry.info.last_activity = chrono::Utc::now();
        }
    }

    /// Record bytes sent on a connection.
    pub fn record_sent(&self, id: &ConnectionId, bytes: u64) {
        if let Some(mut entry) = self.connections.get_mut(id) {
            entry.info.bytes_sent += bytes;
        }
    }

    /// Total number of active connections.
    pub fn count(&self) -> usize {
        self.connections.len()
    }

    /// Number of connections from a specific IP.
    pub fn count_by_ip(&self, ip: IpAddr) -> usize {
        self.ip_counts.get(&ip).map(|r| *r.value()).unwrap_or(0)
    }

    /// Snapshot of all active connection infos.
    pub fn connections(&self) -> Vec<ConnectionInfo> {
        self.connections.iter().map(|r| r.info.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use server_core::Protocol;
    use std::sync::Arc;

    fn make_info(port: u16) -> ConnectionInfo {
        let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
        ConnectionInfo::new(ConnectionId::new(), Protocol::Tcp, addr)
    }

    #[test]
    fn test_register_and_count() {
        let tracker = ConnectionTracker::new(100, 10);
        let info = make_info(5000);
        let id = info.id.clone();
        let _rx = tracker.register(info).unwrap();
        assert_eq!(tracker.count(), 1);
        assert_eq!(tracker.count_by_ip("127.0.0.1".parse().unwrap()), 1);

        tracker.unregister(&id);
        assert_eq!(tracker.count(), 0);
        assert_eq!(tracker.count_by_ip("127.0.0.1".parse().unwrap()), 0);
    }

    #[test]
    fn test_global_limit() {
        let tracker = ConnectionTracker::new(2, 10);
        let _rx1 = tracker.register(make_info(5001)).unwrap();
        let _rx2 = tracker.register(make_info(5002)).unwrap();
        let result = tracker.register(make_info(5003));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            Error::MaxConnectionsReached { max: 2 }
        ));
    }

    #[test]
    fn test_per_ip_limit() {
        let tracker = ConnectionTracker::new(100, 2);
        let _rx1 = tracker.register(make_info(5001)).unwrap();
        let _rx2 = tracker.register(make_info(5002)).unwrap();
        let result = tracker.register(make_info(5003));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            Error::ConnectionRefused { .. }
        ));
    }

    #[test]
    fn test_get_by_addr() {
        let tracker = ConnectionTracker::new(100, 10);
        let info = make_info(5000);
        let id = info.id.clone();
        let addr = info.remote_addr;
        let _rx = tracker.register(info).unwrap();

        assert_eq!(tracker.get_by_addr(&addr), Some(id));
    }

    #[test]
    fn test_update_state() {
        let tracker = ConnectionTracker::new(100, 10);
        let info = make_info(5000);
        let id = info.id.clone();
        let _rx = tracker.register(info).unwrap();

        tracker.update_state(&id, ConnectionState::Established);
        let info = tracker.get(&id).unwrap();
        assert_eq!(info.state, ConnectionState::Established);
    }

    #[test]
    fn test_record_bytes() {
        let tracker = ConnectionTracker::new(100, 10);
        let info = make_info(5000);
        let id = info.id.clone();
        let _rx = tracker.register(info).unwrap();

        tracker.record_received(&id, 100);
        tracker.record_sent(&id, 50);

        let info = tracker.get(&id).unwrap();
        assert_eq!(info.bytes_received, 100);
        assert_eq!(info.bytes_sent, 50);
    }

    #[tokio::test]
    async fn test_send_message() {
        let tracker = Arc::new(ConnectionTracker::new(100, 10));
        let info = make_info(5000);
        let id = info.id.clone();
        let mut rx = tracker.register(info).unwrap();

        tracker
            .send(&id, OutgoingMessage::Text("hello".to_string()))
            .await
            .unwrap();

        let msg = rx.recv().await.unwrap();
        assert!(matches!(msg, OutgoingMessage::Text(t) if t == "hello"));
    }
}
