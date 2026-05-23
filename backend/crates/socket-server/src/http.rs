use axum::http::StatusCode;
use axum::http::header::HeaderName;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use server_config::model::HttpConfig;
use server_core::types::ShutdownReceiver;
use server_core::Error;
use std::net::SocketAddr;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;

pub struct HttpServer {
    config: HttpConfig,
    bind_addr: SocketAddr,
}

impl HttpServer {
    pub fn new(config: HttpConfig, host: &str) -> Self {
        let bind_addr: SocketAddr = format!("{host}:{}", config.port)
            .parse()
            .unwrap_or_else(|_| SocketAddr::from(([0, 0, 0, 0], config.port)));
        Self { config, bind_addr }
    }

    /// Build the axum Router with all middleware configured.
    pub fn router(&self) -> Router {
        let mut app = Router::new().route("/health", get(health_handler));

        // Static file serving
        if let Some(ref dir) = self.config.static_files {
            app = app.nest_service("/static", tower_http::services::ServeDir::new(dir));
        }

        // CORS
        if self.config.cors.enabled {
            let cors = CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
                .max_age(std::time::Duration::from_secs(self.config.cors.max_age_secs));
            app = app.layer(cors);
        }

        // Response compression (gzip, brotli)
        app = app.layer(CompressionLayer::new());

        // Request tracing
        app = app.layer(TraceLayer::new_for_http());

        // Body size limit
        app = app.layer(axum::extract::DefaultBodyLimit::max(
            self.config.request_body_limit,
        ));

        app
    }

    /// Bind the HTTP server and start serving in a background task.
    /// Returns the local address.
    pub async fn start(self, shutdown: ShutdownReceiver) -> server_core::Result<SocketAddr> {
        let router = self.router();

        let listener = tokio::net::TcpListener::bind(self.bind_addr)
            .await
            .map_err(|e| Error::Transport(format!("HTTP bind {}: {e}", self.bind_addr)))?;
        let addr = listener
            .local_addr()
            .map_err(|e| Error::Transport(e.to_string()))?;
        info!(addr = %addr, "HTTP server listening");

        let mut shutdown = shutdown;
        tokio::spawn(async move {
            axum::serve(listener, router.into_make_service())
                .with_graceful_shutdown(async move {
                    shutdown.recv().await;
                })
                .await
                .ok();
        });

        Ok(addr)
    }
}

async fn health_handler() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

// ─── Keep-Alive Configuration ─────────────────────────────────────────────────

/// HTTP keep-alive settings.
///
/// These values are surfaced to the client via the `Keep-Alive` response
/// header so it knows how long and how often it may reuse the TCP connection.
#[derive(Debug, Clone)]
pub struct KeepAliveConfig {
    /// Whether keep-alive connections are enabled at all.
    pub enabled: bool,
    /// How many seconds the server will keep an idle connection open.
    pub timeout_secs: u64,
    /// Maximum number of requests allowed on a single connection.
    pub max_requests: u64,
}

impl Default for KeepAliveConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            timeout_secs: 75,
            max_requests: 1000,
        }
    }
}

/// Append `Connection` and `Keep-Alive` headers to `response` according to
/// `config`.
///
/// - When `enabled` is `true`:
///   * `Connection: keep-alive`
///   * `Keep-Alive: timeout=<timeout_secs>, max=<max_requests>`
/// - When `enabled` is `false`:
///   * `Connection: close`
pub fn apply_keep_alive_headers(response: &mut Response, config: &KeepAliveConfig) {
    if config.enabled {
        response.headers_mut().insert(
            HeaderName::from_static("connection"),
            "keep-alive".parse().unwrap(),
        );
        let value = format!("timeout={}, max={}", config.timeout_secs, config.max_requests);
        response.headers_mut().insert(
            HeaderName::from_static("keep-alive"),
            value.parse().unwrap(),
        );
    } else {
        response.headers_mut().insert(
            HeaderName::from_static("connection"),
            "close".parse().unwrap(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use server_core::types::ShutdownSignal;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    // ── KeepAliveConfig tests ─────────────────────────────────────────────────

    #[test]
    fn test_keep_alive_default_values() {
        let config = KeepAliveConfig::default();
        assert!(config.enabled);
        assert_eq!(config.timeout_secs, 75);
        assert_eq!(config.max_requests, 1000);
    }

    #[test]
    fn test_apply_keep_alive_headers_enabled() {
        let config = KeepAliveConfig::default();
        let mut response = Response::new(Body::empty());
        apply_keep_alive_headers(&mut response, &config);

        let headers = response.headers();
        assert_eq!(headers["connection"], "keep-alive");
        let ka = headers["keep-alive"].to_str().unwrap();
        assert!(ka.contains("timeout=75"));
        assert!(ka.contains("max=1000"));
    }

    #[test]
    fn test_apply_keep_alive_headers_disabled() {
        let config = KeepAliveConfig {
            enabled: false,
            ..Default::default()
        };
        let mut response = Response::new(Body::empty());
        apply_keep_alive_headers(&mut response, &config);

        assert_eq!(response.headers()["connection"], "close");
        assert!(!response.headers().contains_key("keep-alive"));
    }

    // ── HTTP server tests ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_http_health_endpoint() {
        let mut config = HttpConfig::default();
        config.port = 0;

        let (shutdown, shutdown_rx) = ShutdownSignal::new();
        let server = HttpServer::new(config, "127.0.0.1");
        let addr = server.start(shutdown_rx).await.unwrap();

        // Send a raw HTTP request
        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        stream
            .write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();

        let mut response = String::new();
        stream.read_to_string(&mut response).await.unwrap();

        assert!(response.contains("200 OK"));
        assert!(response.contains("OK"));

        shutdown.shutdown();
    }
}
