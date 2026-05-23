use activity_log::metrics::MetricsCollector;
use activity_log::{ActivityLog, AuditLog};
use admin_api::auth::JwtConfig;
use admin_api::auth_store::AdminUserStore;
use admin_api::{AdminServer, AdminServerConfig, AppState};
use billing::UsageTracker;
use cache_layer::create_cache_backend;
use connection_manager::SessionManager;
use data_store::create_storage_backend;
use grpc_api::{GrpcServer, GrpcState};
use plugin_clans::ClansPlugin;
use plugin_host::{ContextBuilder, FullMarketplaceRegistry, PluginRegistry, RouteRegistry};
use plugin_messaging::MessagingPlugin;
use server_config::ConfigLoader;
use server_core::event::EventBus;
use server_core::{ServerInfo, ShutdownSignal};
use socket_server::net_metrics::NetworkMetrics;
use socket_server::tracker::ConnectionTracker;
use socket_server::MultiProtocolListener;
use std::sync::Arc;
use traffic_guard::TrafficGuard;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,draox_server=debug".into()),
        )
        .init();

    info!("Starting Draox Server v{}", env!("CARGO_PKG_VERSION"));

    // ── Load configuration ──
    let config_path = std::env::args()
        .skip_while(|a| a != "--config")
        .nth(1)
        .unwrap_or_else(|| "config/default.toml".to_string());

    let config_loader = match ConfigLoader::from_file(&config_path) {
        Ok(loader) => {
            info!(path = %config_path, "configuration loaded");
            loader
        }
        Err(e) => {
            tracing::warn!("failed to load {config_path}: {e}, using defaults");
            ConfigLoader::default_config()
        }
    };
    let config = Arc::clone(config_loader.config());

    // Shutdown signal
    let (shutdown, _shutdown_rx) = ShutdownSignal::new();

    // Event bus
    let event_bus = Arc::new(EventBus::new(4096));

    // Connection tracker (shared between listener, session handler, and admin API)
    let connection_tracker = Arc::new(ConnectionTracker::new(
        config.server.max_connections,
        config.traffic_guard.connection_limits.max_connections_per_ip,
    ));

    // Session manager
    let session_manager = Arc::new(SessionManager::new(
        config.sessions.clone(),
        Arc::clone(&event_bus),
    ));

    // Traffic guard → Session handler pipeline
    let traffic_guard = Arc::new(TrafficGuard::new(
        config.traffic_guard.clone(),
        Arc::new(connection_manager::SessionHandler::new(
            Arc::clone(&session_manager),
            Arc::clone(&connection_tracker),
        )),
        Arc::clone(&event_bus),
    ));

    // Start traffic guard background tasks
    traffic_guard.start_background_tasks(shutdown.subscribe(), shutdown.subscribe());

    // Activity log, audit log & metrics
    let activity_log = Arc::new(ActivityLog::new(100_000));
    Arc::clone(&activity_log).start_event_listener(Arc::clone(&event_bus), shutdown.subscribe());
    let audit_log = Arc::new(AuditLog::new(100_000));
    let metrics = Arc::new(MetricsCollector::new());

    // Billing
    let usage_tracker = Arc::new(UsageTracker::new());

    // Cache backend (memory or Redis, based on config)
    let (cache, cache_backend_name) = create_cache_backend(&config.cache).await;
    info!(backend = cache_backend_name, "cache layer ready");

    // Storage backend (SQLite, PostgreSQL, MySQL, or MongoDB, based on config)
    let storage = create_storage_backend(&config.storage).await?;
    info!(backend = config.storage.backend.as_str(), "storage layer ready");

    let auth_store = std::sync::Arc::new(AdminUserStore::new(std::sync::Arc::clone(&storage)));

    // Plugin host
    let ctx_builder = ContextBuilder::new(
        ServerInfo::default(),
        Arc::clone(&event_bus),
        Arc::clone(&cache),
    );
    let plugin_registry = Arc::new(PluginRegistry::new(ctx_builder, Arc::clone(&event_bus)));

    // Register built-in plugins
    plugin_registry.register_builtin(Box::new(ClansPlugin::new()))?;
    plugin_registry.register_builtin(Box::new(MessagingPlugin::new()))?;

    info!("Registered {} built-in plugins", plugin_registry.count());

    // Marketplace registry
    let marketplace = Arc::new(FullMarketplaceRegistry::new());

    // Route registry
    let route_registry = Arc::new(RouteRegistry::new());

    // Network metrics
    let network_metrics = Arc::new(NetworkMetrics::new());

    // ── Multi-protocol listener ──
    let listener = MultiProtocolListener::with_tracker(
        Arc::clone(&config),
        Arc::clone(&connection_tracker),
        Arc::clone(&traffic_guard) as Arc<dyn socket_server::ConnectionHandler>,
        Arc::clone(&event_bus),
    );

    let addresses = listener.start(&shutdown).await?;

    if let Some(addr) = addresses.tcp {
        info!("TCP  listening on {addr}");
    }
    if let Some(addr) = addresses.udp {
        info!("UDP  listening on {addr}");
    }
    if let Some(addr) = addresses.ws {
        info!("WS   listening on {addr}");
    }
    if let Some(addr) = addresses.http {
        info!("HTTP listening on {addr}");
    }

    info!(
        "Network metrics ready (active_connections={})",
        network_metrics.snapshot().active_connections
    );

    // ── Admin API ──
    let session_manager_for_grpc = Arc::clone(&session_manager);
    let jwt_config = JwtConfig {
        secret: if config.admin_api.jwt_secret.is_empty() {
            "draox-default-jwt-secret-change-me".to_string()
        } else {
            config.admin_api.jwt_secret.clone()
        },
        expiry_secs: 3600,
    };
    let admin_state = AppState {
        connection_tracker,
        session_manager,
        traffic_guard,
        plugin_registry: Arc::clone(&plugin_registry),
        activity_log,
        metrics,
        usage_tracker,
        audit_log,
        event_bus: Arc::clone(&event_bus),
        marketplace,
        route_registry,
        cache,
        storage,
        jwt_config,
        auth_store,
    };

    let bind_addr: std::net::SocketAddr = format!("{}:{}", config.admin_api.host, config.admin_api.port)
        .parse()
        .unwrap_or_else(|_| "0.0.0.0:9100".parse().unwrap());

    let admin_addr = AdminServer::start(
        AdminServerConfig { bind_addr },
        admin_state,
        shutdown.subscribe(),
    )
    .await?;

    info!("Admin API listening on http://{admin_addr}");

    // ── gRPC Server (if enabled) ──
    if config.grpc.enabled {
        let grpc_state = GrpcState {
            session_manager: session_manager_for_grpc,
            event_bus:       Arc::clone(&event_bus),
            plugin_registry: Arc::clone(&plugin_registry),
        };
        let grpc_addr: std::net::SocketAddr =
            format!("{}:{}", config.server.host, config.grpc.port)
                .parse()
                .unwrap_or_else(|_| "0.0.0.0:9004".parse().unwrap());

        match GrpcServer::start(grpc_addr, grpc_state, shutdown.subscribe()).await {
            Ok(bound) => info!("gRPC listening on {bound}"),
            Err(e)    => tracing::warn!("gRPC server failed to start: {e}"),
        }
    }

    // Publish server started event
    event_bus.publish(server_core::event::ServerEvent::ServerStarted {
        timestamp: chrono::Utc::now(),
    });

    info!("Draox Server is ready");

    // Wait for shutdown signal (Ctrl+C)
    tokio::signal::ctrl_c().await?;
    info!("Shutdown signal received");

    // Graceful shutdown
    event_bus.publish(server_core::event::ServerEvent::ServerShuttingDown {
        reason: "SIGINT received".to_string(),
    });

    // Deactivate all plugins
    plugin_registry.deactivate_all().await;

    // Signal shutdown to all tasks
    shutdown.shutdown();

    info!("Draox Server stopped");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_info() {
        let info = ServerInfo::default();
        assert_eq!(info.name, "Draox Server");
        assert!(!info.protocols.is_empty());
    }

    #[test]
    fn test_shutdown_signal() {
        let (signal, rx) = ShutdownSignal::new();
        signal.shutdown();
        // The recv would succeed in an async context
        // Just test that it compiles and creates correctly
        drop(rx);
    }
}
