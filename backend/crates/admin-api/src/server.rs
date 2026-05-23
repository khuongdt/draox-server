use crate::routes::build_router;
use crate::seed::seed_default_users;
use crate::state::AppState;
use crate::trace_context::trace_middleware;
use axum::middleware;
use server_core::ShutdownReceiver;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;

/// Admin API server configuration.
pub struct AdminServerConfig {
    pub bind_addr: SocketAddr,
}

impl Default for AdminServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:9100".parse().unwrap(),
        }
    }
}

/// The admin API server — runs on a separate port from the main server.
pub struct AdminServer;

impl AdminServer {
    /// Start the admin API server.
    ///
    /// Returns the actual bound address (useful when binding to port 0).
    pub async fn start(
        config: AdminServerConfig,
        state: AppState,
        mut shutdown: ShutdownReceiver,
    ) -> std::io::Result<SocketAddr> {
        seed_default_users(&state.auth_store).await;

        let router = build_router(state)
            .layer(middleware::from_fn(trace_middleware))
            .layer(CorsLayer::permissive())
            .layer(TraceLayer::new_for_http());

        let listener = TcpListener::bind(config.bind_addr).await?;
        let local_addr = listener.local_addr()?;

        info!(addr = %local_addr, "admin API server started");

        tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    shutdown.recv().await;
                    info!("admin API server shutting down");
                })
                .await
                .ok();
        });

        Ok(local_addr)
    }
}
