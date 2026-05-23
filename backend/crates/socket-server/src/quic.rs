use std::{net::SocketAddr, sync::Arc, time::Duration};
use quinn::{Connection, Endpoint, RecvStream, SendStream, ServerConfig};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

#[derive(Debug, Error)]
pub enum QuicError {
    #[error("TLS config error: {0}")]
    Tls(String),
    #[error("endpoint bind error: {0}")]
    Bind(#[from] std::io::Error),
    #[error("quinn connection error: {0}")]
    Connection(#[from] quinn::ConnectionError),
    #[error("quinn stream error: {0}")]
    Stream(String),
}

/// A received QUIC datagram or stream payload.
pub struct QuicMessage {
    pub connection_id: String,
    pub remote_addr: SocketAddr,
    pub data: bytes::Bytes,
}

/// Configuration for the QUIC server.
#[derive(Clone)]
pub struct QuicConfig {
    /// Address to bind the QUIC endpoint on (UDP).
    pub bind_addr: SocketAddr,
    /// Maximum idle timeout before a connection is closed.
    pub idle_timeout: Duration,
    /// Maximum number of concurrent bidirectional streams per connection.
    pub max_concurrent_bidi: u64,
    /// Maximum concurrent connections.
    pub max_connections: usize,
}

impl Default for QuicConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:9004".parse().unwrap(),
            idle_timeout: Duration::from_secs(30),
            max_concurrent_bidi: 100,
            max_connections: 10_000,
        }
    }
}

/// Build a `quinn::ServerConfig` from DER-encoded certificate + key.
pub fn build_server_config(
    cert_der: CertificateDer<'static>,
    key_der: PrivateKeyDer<'static>,
) -> Result<ServerConfig, QuicError> {
    let mut tls_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_der], key_der)
        .map_err(|e| QuicError::Tls(e.to_string()))?;

    tls_config.alpn_protocols = vec![b"draox/1".to_vec()];
    tls_config.max_early_data_size = u32::MAX; // enable 0-RTT

    let transport = Arc::new(quinn::TransportConfig::default());
    let mut server_config = ServerConfig::with_crypto(Arc::new(
        quinn::crypto::rustls::QuicServerConfig::try_from(tls_config)
            .map_err(|e| QuicError::Tls(e.to_string()))?,
    ));
    server_config.transport_config(transport);
    Ok(server_config)
}

/// QUIC server that accepts connections and spawns a task per connection.
pub struct QuicServer {
    endpoint: Endpoint,
    cfg: QuicConfig,
}

impl QuicServer {
    pub fn bind(cfg: QuicConfig, server_config: ServerConfig) -> Result<Self, QuicError> {
        let endpoint = Endpoint::server(server_config, cfg.bind_addr)?;
        info!(addr = %cfg.bind_addr, "QUIC endpoint bound");
        Ok(Self { endpoint, cfg })
    }

    /// Accept connections until shutdown. Incoming messages are sent on `tx`.
    pub async fn serve(self, tx: mpsc::Sender<QuicMessage>) {
        let Self { endpoint, cfg } = self;
        info!(addr = %cfg.bind_addr, "QUIC server listening");

        while let Some(incoming) = endpoint.accept().await {
            let tx = tx.clone();
            tokio::spawn(async move {
                match incoming.await {
                    Ok(conn) => {
                        handle_connection(conn, tx).await;
                    }
                    Err(e) => warn!(error = %e, "QUIC connection handshake failed"),
                }
            });
        }
    }

    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.endpoint.local_addr().ok()
    }
}

/// Handle one QUIC connection: accept bidirectional streams indefinitely.
async fn handle_connection(conn: Connection, tx: mpsc::Sender<QuicMessage>) {
    let remote = conn.remote_address();
    let conn_id = conn.stable_id().to_string();
    debug!(conn_id, %remote, "QUIC connection established");

    loop {
        match conn.accept_bi().await {
            Ok((send, recv)) => {
                let tx = tx.clone();
                let conn_id = conn_id.clone();
                tokio::spawn(handle_stream(send, recv, remote, conn_id, tx));
            }
            Err(quinn::ConnectionError::ApplicationClosed(_))
            | Err(quinn::ConnectionError::LocallyClosed) => {
                debug!(conn_id, "QUIC connection closed");
                break;
            }
            Err(e) => {
                error!(conn_id, error = %e, "QUIC connection error");
                break;
            }
        }
    }
}

/// Read a full bidirectional stream into a `QuicMessage` and send it upstream.
async fn handle_stream(
    mut send: SendStream,
    mut recv: RecvStream,
    remote: SocketAddr,
    conn_id: String,
    tx: mpsc::Sender<QuicMessage>,
) {
    match recv.read_to_end(64 * 1024).await {
        Ok(data) => {
            let msg = QuicMessage {
                connection_id: conn_id.clone(),
                remote_addr: remote,
                data: data.into(),
            };
            if tx.send(msg).await.is_err() {
                warn!(conn_id, "QUIC message channel closed");
            }
            // Echo ACK (protocol-level ack; application can override)
            let _ = send.write_all(b"OK").await;
            let _ = send.finish();
        }
        Err(e) => {
            warn!(conn_id, error = %e, "QUIC stream read error");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = QuicConfig::default();
        assert_eq!(cfg.bind_addr.port(), 9004);
        assert_eq!(cfg.max_concurrent_bidi, 100);
    }
}
