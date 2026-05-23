use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Root configuration for Draox Server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DraoxConfig {
    pub server: ServerConfig,
    pub tcp: TcpConfig,
    pub udp: UdpConfig,
    pub websocket: WebSocketConfig,
    pub http: HttpConfig,
    pub grpc: GrpcConfig,
    pub tls: TlsConfig,
    pub traffic_guard: TrafficGuardConfig,
    pub sessions: SessionConfig,
    pub storage: StorageConfig,
    pub cache: CacheConfig,
    pub billing: BillingConfig,
    pub admin_api: AdminApiConfig,
    pub logging: LoggingConfig,
    pub metrics: MetricsConfig,
    pub marketplace: MarketplaceConfig,
    pub plugins: HashMap<String, toml::Value>,
}

impl Default for DraoxConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            tcp: TcpConfig::default(),
            udp: UdpConfig::default(),
            websocket: WebSocketConfig::default(),
            http: HttpConfig::default(),
            grpc: GrpcConfig::default(),
            tls: TlsConfig::default(),
            traffic_guard: TrafficGuardConfig::default(),
            sessions: SessionConfig::default(),
            storage: StorageConfig::default(),
            cache: CacheConfig::default(),
            billing: BillingConfig::default(),
            admin_api: AdminApiConfig::default(),
            logging: LoggingConfig::default(),
            metrics: MetricsConfig::default(),
            marketplace: MarketplaceConfig::default(),
            plugins: HashMap::new(),
        }
    }
}

// ────────────────────────────────────────────────────────
// Server
// ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub name: String,
    pub host: String,
    pub workers: usize,
    pub max_connections: usize,
    pub shutdown_timeout_secs: u64,
    pub plugin_dir: PathBuf,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            name: "Draox Server".to_string(),
            host: "0.0.0.0".to_string(),
            workers: num_cpus(),
            max_connections: 10_000,
            shutdown_timeout_secs: 30,
            plugin_dir: PathBuf::from("plugins"),
        }
    }
}

// ────────────────────────────────────────────────────────
// TCP
// ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TcpConfig {
    pub enabled: bool,
    pub port: u16,
    pub backlog: u32,
    pub nodelay: bool,
    pub keepalive_secs: Option<u64>,
    pub recv_buffer_size: usize,
    pub send_buffer_size: usize,
    pub idle_timeout_secs: u64,
}

impl Default for TcpConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            port: 9000,
            backlog: 1024,
            nodelay: true,
            keepalive_secs: Some(60),
            recv_buffer_size: 65_536,
            send_buffer_size: 65_536,
            idle_timeout_secs: 300,
        }
    }
}

// ────────────────────────────────────────────────────────
// UDP
// ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UdpConfig {
    pub enabled: bool,
    pub port: u16,
    pub recv_buffer_size: usize,
    pub send_buffer_size: usize,
    pub max_packet_size: usize,
    pub session_timeout_secs: u64,
    pub multicast_enabled: bool,
    pub broadcast_enabled: bool,
}

impl Default for UdpConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            port: 9001,
            recv_buffer_size: 65_536,
            send_buffer_size: 65_536,
            max_packet_size: 65_507,
            session_timeout_secs: 60,
            multicast_enabled: false,
            broadcast_enabled: false,
        }
    }
}

// ────────────────────────────────────────────────────────
// WebSocket
// ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WebSocketConfig {
    pub enabled: bool,
    pub port: u16,
    pub path: String,
    pub max_frame_size: usize,
    pub max_message_size: usize,
    pub ping_interval_secs: u64,
    pub pong_timeout_secs: u64,
    pub compression: bool,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            port: 9002,
            path: "/ws".to_string(),
            max_frame_size: 65_536,
            max_message_size: 1_048_576,
            ping_interval_secs: 30,
            pong_timeout_secs: 10,
            compression: false,
        }
    }
}

// ────────────────────────────────────────────────────────
// HTTP
// ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HttpConfig {
    pub enabled: bool,
    pub port: u16,
    pub request_body_limit: usize,
    pub request_timeout_secs: u64,
    pub keepalive: bool,
    pub cors: CorsConfig,
    pub sse_enabled: bool,
    pub static_files: Option<PathBuf>,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            port: 9003,
            request_body_limit: 10_485_760, // 10 MB
            request_timeout_secs: 30,
            keepalive: true,
            cors: CorsConfig::default(),
            sse_enabled: true,
            static_files: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CorsConfig {
    pub enabled: bool,
    pub allowed_origins: Vec<String>,
    pub allowed_methods: Vec<String>,
    pub allowed_headers: Vec<String>,
    pub max_age_secs: u64,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec!["GET".into(), "POST".into(), "PUT".into(), "DELETE".into(), "PATCH".into()],
            allowed_headers: vec!["Content-Type".into(), "Authorization".into()],
            max_age_secs: 86400,
        }
    }
}

// ────────────────────────────────────────────────────────
// gRPC
// ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GrpcConfig {
    pub enabled: bool,
    pub port: u16,
    pub tls_enabled: bool,
    pub max_frame_size_bytes: u32,
    pub reflection_enabled: bool,
    pub max_concurrent_streams: Option<u32>,
}

impl Default for GrpcConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 9004,
            tls_enabled: false,
            max_frame_size_bytes: 4 * 1024 * 1024,
            reflection_enabled: true,
            max_concurrent_streams: None,
        }
    }
}

// ────────────────────────────────────────────────────────
// TLS
// ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TlsConfig {
    pub enabled: bool,
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
    pub ca_path: Option<PathBuf>,
    pub mtls: bool,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            cert_path: PathBuf::from("certs/server.crt"),
            key_path: PathBuf::from("certs/server.key"),
            ca_path: None,
            mtls: false,
        }
    }
}

// ────────────────────────────────────────────────────────
// Traffic Guard
// ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TrafficGuardConfig {
    pub enabled: bool,
    pub connection_limits: ConnectionLimitsConfig,
    pub rate_limiting: RateLimitingConfig,
    pub banning: BanningConfig,
    pub ip_reputation: IpReputationConfig,
    pub blacklist: IpListConfig,
    pub whitelist: IpListConfig,
    pub slowloris: SlowlorisConfig,
    pub adaptive: AdaptiveConfig,
}

impl Default for TrafficGuardConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            connection_limits: ConnectionLimitsConfig::default(),
            rate_limiting: RateLimitingConfig::default(),
            banning: BanningConfig::default(),
            ip_reputation: IpReputationConfig::default(),
            blacklist: IpListConfig::default(),
            whitelist: IpListConfig {
                ips: vec!["127.0.0.1".to_string(), "::1".to_string()],
                cidrs: vec![],
            },
            slowloris: SlowlorisConfig::default(),
            adaptive: AdaptiveConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ConnectionLimitsConfig {
    pub max_connections_per_ip: u32,
    pub max_new_connections_per_sec_per_ip: u32,
    pub max_new_connections_per_sec_global: u32,
    pub max_half_open_connections: u32,
    pub connection_timeout_secs: u64,
}

impl Default for ConnectionLimitsConfig {
    fn default() -> Self {
        Self {
            max_connections_per_ip: 50,
            max_new_connections_per_sec_per_ip: 10,
            max_new_connections_per_sec_global: 1000,
            max_half_open_connections: 500,
            connection_timeout_secs: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RateLimitingConfig {
    pub algorithm: String,
    pub default_requests_per_sec: u32,
    pub burst_size: u32,
    pub http_rate_per_sec: u32,
    pub ws_messages_per_sec: u32,
    pub udp_packets_per_sec: u32,
}

impl Default for RateLimitingConfig {
    fn default() -> Self {
        Self {
            algorithm: "token_bucket".to_string(),
            default_requests_per_sec: 100,
            burst_size: 50,
            http_rate_per_sec: 200,
            ws_messages_per_sec: 60,
            udp_packets_per_sec: 500,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BanningConfig {
    pub enabled: bool,
    pub max_violations_before_ban: u32,
    pub initial_ban_duration_secs: u64,
    pub ban_escalation_multiplier: u32,
    pub max_ban_duration_secs: u64,
    pub auth_failure_threshold: u32,
    pub auth_failure_window_secs: u64,
}

impl Default for BanningConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_violations_before_ban: 5,
            initial_ban_duration_secs: 300,
            ban_escalation_multiplier: 6,
            max_ban_duration_secs: 86_400,
            auth_failure_threshold: 10,
            auth_failure_window_secs: 300,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct IpReputationConfig {
    pub enabled: bool,
    pub initial_score: u32,
    pub min_score_to_connect: u32,
    pub violation_penalty: u32,
    pub recovery_rate_per_hour: u32,
    pub score_persistence: String,
}

impl Default for IpReputationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            initial_score: 100,
            min_score_to_connect: 20,
            violation_penalty: 10,
            recovery_rate_per_hour: 5,
            score_persistence: "memory".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct IpListConfig {
    pub ips: Vec<String>,
    pub cidrs: Vec<String>,
}

impl Default for IpListConfig {
    fn default() -> Self {
        Self {
            ips: vec![],
            cidrs: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SlowlorisConfig {
    pub enabled: bool,
    pub min_data_rate_bytes_sec: u64,
    pub header_timeout_secs: u64,
    pub body_timeout_secs: u64,
}

impl Default for SlowlorisConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_data_rate_bytes_sec: 100,
            header_timeout_secs: 30,
            body_timeout_secs: 60,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AdaptiveConfig {
    pub enabled: bool,
    pub cpu_threshold_percent: u32,
    pub memory_threshold_percent: u32,
    pub throttle_factor: f64,
    pub recovery_cooldown_secs: u64,
}

impl Default for AdaptiveConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cpu_threshold_percent: 80,
            memory_threshold_percent: 85,
            throttle_factor: 0.5,
            recovery_cooldown_secs: 30,
        }
    }
}

// ────────────────────────────────────────────────────────
// Sessions
// ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SessionConfig {
    pub max_connections_per_session: usize,
    pub session_timeout_secs: u64,
    pub grace_period_secs: u64,
    pub heartbeat_interval_secs: u64,
    pub heartbeat_timeout_secs: u64,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_connections_per_session: 5,
            session_timeout_secs: 3600,
            grace_period_secs: 30,
            heartbeat_interval_secs: 30,
            heartbeat_timeout_secs: 10,
        }
    }
}

// ────────────────────────────────────────────────────────
// Storage
// ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StorageConfig {
    pub backend: String,
    pub sql: SqlConfig,
    pub mongodb: MongoConfig,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            backend: "sqlite".to_string(),
            sql: SqlConfig::default(),
            mongodb: MongoConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SqlConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub idle_timeout_secs: u64,
    pub max_lifetime_secs: u64,
    pub run_migrations: bool,
}

impl Default for SqlConfig {
    fn default() -> Self {
        Self {
            url: "sqlite://data/draox.db?mode=rwc".to_string(),
            max_connections: 10,
            min_connections: 1,
            idle_timeout_secs: 300,
            max_lifetime_secs: 3600,
            run_migrations: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MongoConfig {
    pub enabled: bool,
    pub url: String,
    pub database: String,
    pub max_pool_size: u32,
}

impl Default for MongoConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: "mongodb://localhost:27017".to_string(),
            database: "draox".to_string(),
            max_pool_size: 10,
        }
    }
}

// ────────────────────────────────────────────────────────
// Cache
// ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CacheConfig {
    pub redis: RedisConfig,
    pub memory: MemoryCacheConfig,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            redis: RedisConfig::default(),
            memory: MemoryCacheConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RedisConfig {
    pub enabled: bool,
    pub url: String,
    pub pool_size: u32,
    pub default_ttl_secs: u64,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: "redis://localhost:6379".to_string(),
            pool_size: 5,
            default_ttl_secs: 3600,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MemoryCacheConfig {
    pub max_capacity: u64,
    pub ttl_secs: u64,
}

impl Default for MemoryCacheConfig {
    fn default() -> Self {
        Self {
            max_capacity: 10_000,
            ttl_secs: 300,
        }
    }
}

// ────────────────────────────────────────────────────────
// Billing
// ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BillingConfig {
    pub enabled: bool,
    pub provider: String,
    pub free_tier_requests_per_day: u64,
    pub free_tier_connections: u32,
}

impl Default for BillingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: "stripe".to_string(),
            free_tier_requests_per_day: 10_000,
            free_tier_connections: 100,
        }
    }
}

// ────────────────────────────────────────────────────────
// Admin API
// ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AdminApiConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub auth_method: String,
    pub jwt_secret: String,
    pub api_keys: Vec<String>,
    pub cors: CorsConfig,
    pub swagger_enabled: bool,
}

impl Default for AdminApiConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            host: "127.0.0.1".to_string(),
            port: 9100,
            auth_method: "jwt".to_string(),
            jwt_secret: String::new(),
            api_keys: vec![],
            cors: CorsConfig::default(),
            swagger_enabled: true,
        }
    }
}

// ────────────────────────────────────────────────────────
// Logging
// ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub file: Option<PathBuf>,
    pub rotation: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "pretty".to_string(),
            file: None,
            rotation: "daily".to_string(),
        }
    }
}

// ────────────────────────────────────────────────────────
// Metrics
// ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub endpoint: String,
    pub port: u16,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            endpoint: "/metrics".to_string(),
            port: 9090,
        }
    }
}

// ────────────────────────────────────────────────────────
// Marketplace
// ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MarketplaceConfig {
    pub enabled: bool,
    pub registry_url: String,
    pub auto_update_check: bool,
    pub update_check_interval_secs: u64,
    pub verify_signatures: bool,
}

impl Default for MarketplaceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            registry_url: "https://marketplace.draox-server.io/api/v1".to_string(),
            auto_update_check: true,
            update_check_interval_secs: 86_400,
            verify_signatures: true,
        }
    }
}

// ────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

// ────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DraoxConfig::default();
        assert_eq!(config.server.name, "Draox Server");
        assert!(config.tcp.enabled);
        assert_eq!(config.tcp.port, 9000);
        assert_eq!(config.udp.port, 9001);
        assert_eq!(config.websocket.port, 9002);
        assert_eq!(config.http.port, 9003);
        assert_eq!(config.admin_api.port, 9100);
    }

    #[test]
    fn test_traffic_guard_defaults() {
        let config = TrafficGuardConfig::default();
        assert!(config.enabled);
        assert_eq!(config.connection_limits.max_connections_per_ip, 50);
        assert_eq!(config.banning.initial_ban_duration_secs, 300);
        assert_eq!(config.banning.ban_escalation_multiplier, 6);
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let config = DraoxConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: DraoxConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.server.name, config.server.name);
        assert_eq!(parsed.tcp.port, config.tcp.port);
    }
}
