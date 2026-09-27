use server_config::model::TlsConfig;
use server_core::{Error, Result};
use std::fs::File;
use std::io::BufReader;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Semaphore};
use tokio::task::JoinHandle;
use tokio_rustls::server::TlsStream;
use tokio_rustls::TlsAcceptor;
use tracing::{debug, info, warn};

/// Handshakes slower than this are dropped so idle sockets cannot pin resources.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
/// Upper bound on concurrent in-flight TLS handshakes per listener.
const MAX_PENDING_HANDSHAKES: usize = 1024;

// K.D 2026-09-27 P0 rustls is compiled with both `ring` and `aws_lc_rs` (tokio-rustls
// default features), so `ServerConfig::builder()` cannot pick a process default and
// panics. Pin the provider explicitly instead of relying on a global install.
fn crypto_provider() -> Arc<rustls::crypto::CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

fn server_config_builder(
) -> Result<rustls::ConfigBuilder<rustls::ServerConfig, rustls::WantsVerifier>> {
    rustls::ServerConfig::builder_with_provider(crypto_provider())
        .with_safe_default_protocol_versions()
        .map_err(|e| Error::Config(format!("TLS protocol config error: {e}")))
}

/// Load TLS server configuration from PEM certificate and key files.
///
/// Returns an `Arc<rustls::ServerConfig>` suitable for creating a
/// `TlsAcceptor` (TCP) or passing to axum/hyper (HTTP).
pub fn load_tls_config(config: &TlsConfig) -> Result<Arc<rustls::ServerConfig>> {
    info!(
        cert = %config.cert_path.display(),
        key = %config.key_path.display(),
        mtls = config.mtls,
        "loading TLS configuration"
    );

    let cert_file = File::open(&config.cert_path).map_err(|e| {
        Error::Config(format!(
            "failed to open TLS cert {}: {e}",
            config.cert_path.display()
        ))
    })?;
    let key_file = File::open(&config.key_path).map_err(|e| {
        Error::Config(format!(
            "failed to open TLS key {}: {e}",
            config.key_path.display()
        ))
    })?;

    let certs: Vec<_> = rustls_pemfile::certs(&mut BufReader::new(cert_file))
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| Error::Config(format!("failed to parse TLS certs: {e}")))?;

    let key = rustls_pemfile::private_key(&mut BufReader::new(key_file))
        .map_err(|e| Error::Config(format!("failed to parse TLS key: {e}")))?
        .ok_or_else(|| Error::Config("no private key found in TLS key file".to_string()))?;

    let mut server_config = if config.mtls {
        let ca_path = config
            .ca_path
            .as_ref()
            .ok_or_else(|| Error::Config("mTLS requires ca_path to be set".to_string()))?;

        let ca_file = File::open(ca_path).map_err(|e| {
            Error::Config(format!("failed to open CA cert {}: {e}", ca_path.display()))
        })?;
        let ca_certs: Vec<_> = rustls_pemfile::certs(&mut BufReader::new(ca_file))
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| Error::Config(format!("failed to parse CA certs: {e}")))?;

        let mut root_store = rustls::RootCertStore::empty();
        for cert in ca_certs {
            root_store
                .add(cert)
                .map_err(|e| Error::Config(format!("failed to add CA cert: {e}")))?;
        }

        let verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
            Arc::new(root_store),
            crypto_provider(),
        )
        .build()
        .map_err(|e| Error::Config(format!("failed to build client verifier: {e}")))?;

        server_config_builder()?
            .with_client_cert_verifier(verifier)
            .with_single_cert(certs, key)
            .map_err(|e| Error::Config(format!("TLS config error: {e}")))?
    } else {
        server_config_builder()?
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|e| Error::Config(format!("TLS config error: {e}")))?
    };

    // Advertise HTTP/1.1 so HTTPS clients and WebSocket upgrades negotiate a protocol
    // hyper serves; raw TCP clients that send no ALPN are unaffected.
    server_config.alpn_protocols = vec![b"http/1.1".to_vec()];

    info!("TLS configuration loaded successfully");
    Ok(Arc::new(server_config))
}

/// Create a `TlsAcceptor` from a TLS config (for use with TCP streams).
pub fn create_tls_acceptor(config: &TlsConfig) -> Result<TlsAcceptor> {
    let server_config = load_tls_config(config)?;
    Ok(TlsAcceptor::from(server_config))
}

// ────────────────────────────────────────────────────────
// TLS listener for axum (HTTPS / WSS)
// ────────────────────────────────────────────────────────

/// A TCP listener that terminates TLS before handing streams to `axum::serve`.
///
/// Handshakes run in their own tasks: `axum::serve` accepts sequentially, so an
/// inline handshake would let a single slow client stall every new connection.
pub struct TlsListener {
    local_addr: SocketAddr,
    rx: mpsc::Receiver<(TlsStream<TcpStream>, SocketAddr)>,
    accept_task: JoinHandle<()>,
}

impl TlsListener {
    pub fn new(listener: TcpListener, acceptor: TlsAcceptor) -> std::io::Result<Self> {
        let local_addr = listener.local_addr()?;
        let (tx, rx) = mpsc::channel(128);
        let accept_task = tokio::spawn(accept_loop(listener, acceptor, tx));
        Ok(Self {
            local_addr,
            rx,
            accept_task,
        })
    }
}

impl Drop for TlsListener {
    fn drop(&mut self) {
        // axum drops the listener on graceful shutdown; release the port with it.
        self.accept_task.abort();
    }
}

async fn accept_loop(
    listener: TcpListener,
    acceptor: TlsAcceptor,
    tx: mpsc::Sender<(TlsStream<TcpStream>, SocketAddr)>,
) {
    let permits = Arc::new(Semaphore::new(MAX_PENDING_HANDSHAKES));
    loop {
        let (tcp, addr) = match listener.accept().await {
            Ok(pair) => pair,
            Err(e) => {
                // Same policy as axum's TcpListener: transient errors (e.g. EMFILE)
                // must not terminate the accept loop.
                warn!(error = %e, "TLS listener accept error");
                tokio::time::sleep(Duration::from_millis(50)).await;
                continue;
            }
        };
        let Ok(permit) = Arc::clone(&permits).acquire_owned().await else {
            return;
        };
        let acceptor = acceptor.clone();
        let tx = tx.clone();
        tokio::spawn(async move {
            let _permit = permit;
            match tokio::time::timeout(HANDSHAKE_TIMEOUT, acceptor.accept(tcp)).await {
                Ok(Ok(stream)) => {
                    let _ = tx.send((stream, addr)).await;
                }
                Ok(Err(e)) => debug!(peer = %addr, error = %e, "TLS handshake failed"),
                Err(_) => debug!(peer = %addr, "TLS handshake timed out"),
            }
        });
    }
}

impl axum::serve::Listener for TlsListener {
    type Io = TlsStream<TcpStream>;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        match self.rx.recv().await {
            Some(conn) => conn,
            // Accept loop ended; never resolve so axum just idles until shutdown.
            None => std::future::pending().await,
        }
    }

    fn local_addr(&self) -> std::io::Result<Self::Addr> {
        Ok(self.local_addr)
    }
}

/// Peer address of a connection accepted by [`TlsListener`].
///
/// axum only implements `Connected` for `SocketAddr` on its own listeners and the
/// orphan rule forbids adding one here, so TLS servers use this newtype and copy it
/// into `ConnectInfo<SocketAddr>` (see [`tls_connect_info_to_socket_addr`]).
#[derive(Debug, Clone, Copy)]
pub struct TlsConnectAddr(pub SocketAddr);

impl axum::extract::connect_info::Connected<axum::serve::IncomingStream<'_, TlsListener>>
    for TlsConnectAddr
{
    fn connect_info(stream: axum::serve::IncomingStream<'_, TlsListener>) -> Self {
        Self(*stream.remote_addr())
    }
}

/// Middleware that exposes a TLS peer address as `ConnectInfo<SocketAddr>`, so handlers
/// work unchanged whether the listener is plain or TLS.
pub async fn tls_connect_info_to_socket_addr(
    mut request: axum::extract::Request,
) -> axum::extract::Request {
    use axum::extract::ConnectInfo;
    if let Some(ConnectInfo(TlsConnectAddr(addr))) =
        request.extensions().get::<ConnectInfo<TlsConnectAddr>>().copied()
    {
        request.extensions_mut().insert(ConnectInfo(addr));
    }
    request
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
    use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
    use rustls::{DigitallySignedStruct, SignatureScheme};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// TLS config pointing at the repo's self-signed dev certs.
    pub(crate) fn dev_tls_config() -> TlsConfig {
        TlsConfig {
            enabled: true,
            cert_path: "../../certs/server.crt".into(),
            key_path: "../../certs/server.key".into(),
            ..TlsConfig::default()
        }
    }

    /// The dev cert is self-signed with CA:TRUE, which webpki rejects as an end-entity
    /// cert, so tests verify that a TLS session is established, not the chain.
    #[derive(Debug)]
    pub(crate) struct AcceptAnyCert;

    impl ServerCertVerifier for AcceptAnyCert {
        fn verify_server_cert(
            &self,
            _: &CertificateDer<'_>,
            _: &[CertificateDer<'_>],
            _: &ServerName<'_>,
            _: &[u8],
            _: UnixTime,
        ) -> std::result::Result<ServerCertVerified, rustls::Error> {
            Ok(ServerCertVerified::assertion())
        }

        fn verify_tls12_signature(
            &self,
            _: &[u8],
            _: &CertificateDer<'_>,
            _: &DigitallySignedStruct,
        ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
            Ok(HandshakeSignatureValid::assertion())
        }

        fn verify_tls13_signature(
            &self,
            _: &[u8],
            _: &CertificateDer<'_>,
            _: &DigitallySignedStruct,
        ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
            Ok(HandshakeSignatureValid::assertion())
        }

        fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
            crypto_provider()
                .signature_verification_algorithms
                .supported_schemes()
        }
    }

    pub(crate) fn insecure_client_config() -> Arc<rustls::ClientConfig> {
        let mut config = rustls::ClientConfig::builder_with_provider(crypto_provider())
            .with_safe_default_protocol_versions()
            .unwrap()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(AcceptAnyCert))
            .with_no_client_auth();
        config.alpn_protocols = vec![b"http/1.1".to_vec()];
        Arc::new(config)
    }

    /// Connect over TLS, send `request`, and return everything the server sends back.
    pub(crate) async fn tls_roundtrip(addr: SocketAddr, request: &[u8]) -> String {
        let connector = tokio_rustls::TlsConnector::from(insecure_client_config());
        let tcp = TcpStream::connect(addr).await.unwrap();
        let mut tls = connector
            .connect(ServerName::try_from("localhost").unwrap(), tcp)
            .await
            .expect("TLS handshake should succeed");
        tls.write_all(request).await.unwrap();
        let mut buf = Vec::new();
        let _ = tls.read_to_end(&mut buf).await;
        String::from_utf8_lossy(&buf).into_owned()
    }

    #[test]
    fn test_load_dev_tls_config() {
        // Regression: must not panic on the ambiguous ring + aws-lc-rs provider setup.
        assert!(load_tls_config(&dev_tls_config()).is_ok());
    }

    #[test]
    fn test_missing_cert_is_error() {
        let mut cfg = dev_tls_config();
        cfg.cert_path = "does/not/exist.crt".into();
        assert!(create_tls_acceptor(&cfg).is_err());
    }

    #[tokio::test]
    async fn test_plaintext_client_never_reaches_axum() {
        let acceptor = create_tls_acceptor(&dev_tls_config()).unwrap();
        let tcp = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut listener = TlsListener::new(tcp, acceptor).unwrap();
        let addr = axum::serve::Listener::local_addr(&listener).unwrap();

        let mut plain = TcpStream::connect(addr).await.unwrap();
        plain
            .write_all(b"GET / HTTP/1.1\r\nHost: x\r\n\r\n")
            .await
            .unwrap();

        // A plaintext request fails the handshake, so no stream is handed to axum.
        let accepted = tokio::time::timeout(
            Duration::from_millis(300),
            axum::serve::Listener::accept(&mut listener),
        )
        .await;
        assert!(accepted.is_err());
    }
}
