use server_config::model::TlsConfig;
use server_core::{Error, Result};
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use tokio_rustls::TlsAcceptor;
use tracing::info;

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

    let server_config = if config.mtls {
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

        let verifier = rustls::server::WebPkiClientVerifier::builder(Arc::new(root_store))
            .build()
            .map_err(|e| Error::Config(format!("failed to build client verifier: {e}")))?;

        rustls::ServerConfig::builder()
            .with_client_cert_verifier(verifier)
            .with_single_cert(certs, key)
            .map_err(|e| Error::Config(format!("TLS config error: {e}")))?
    } else {
        rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|e| Error::Config(format!("TLS config error: {e}")))?
    };

    info!("TLS configuration loaded successfully");
    Ok(Arc::new(server_config))
}

/// Create a `TlsAcceptor` from a TLS config (for use with TCP streams).
pub fn create_tls_acceptor(config: &TlsConfig) -> Result<TlsAcceptor> {
    let server_config = load_tls_config(config)?;
    Ok(TlsAcceptor::from(server_config))
}
