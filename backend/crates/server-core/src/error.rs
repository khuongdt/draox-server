use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    // Config
    #[error("configuration error: {0}")]
    Config(String),

    #[error("config file not found: {path}")]
    ConfigNotFound { path: String },

    #[error("config validation failed: {field}: {reason}")]
    ConfigValidation { field: String, reason: String },

    // Networking
    #[error("connection error: {0}")]
    Connection(String),

    #[error("connection refused for {addr}: {reason}")]
    ConnectionRefused { addr: String, reason: String },

    #[error("connection timeout for {addr} after {timeout_ms}ms")]
    ConnectionTimeout { addr: String, timeout_ms: u64 },

    #[error("transport error: {0}")]
    Transport(String),

    // Protocol
    #[error("protocol error: {0}")]
    Protocol(String),

    #[error("invalid message: {0}")]
    InvalidMessage(String),

    #[error("message too large: {size} bytes (max: {max} bytes)")]
    MessageTooLarge { size: usize, max: usize },

    // Authentication
    #[error("authentication failed: {0}")]
    AuthFailed(String),

    #[error("unauthorized: {0}")]
    Unauthorized(String),

    #[error("forbidden: {0}")]
    Forbidden(String),

    // Session
    #[error("session not found: {0}")]
    SessionNotFound(String),

    #[error("session expired: {0}")]
    SessionExpired(String),

    #[error("max connections per session reached: {max}")]
    MaxConnectionsReached { max: usize },

    // Plugin
    #[error("plugin error [{plugin_id}]: {message}")]
    Plugin { plugin_id: String, message: String },

    #[error("plugin not found: {0}")]
    PluginNotFound(String),

    #[error("plugin manifest invalid: {0}")]
    PluginManifestInvalid(String),

    #[error("plugin activation failed [{plugin_id}]: {reason}")]
    PluginActivation { plugin_id: String, reason: String },

    // Storage
    #[error("storage error: {0}")]
    Storage(String),

    // Cache
    #[error("cache error: {0}")]
    Cache(String),

    // Rate limiting / traffic guard
    #[error("rate limited: {0}")]
    RateLimited(String),

    #[error("IP banned: {addr} until {until}")]
    IpBanned { addr: String, until: String },

    // Internal
    #[error("internal error: {0}")]
    Internal(String),

    #[error("not implemented: {0}")]
    NotImplemented(String),

    #[error("shutdown in progress")]
    ShuttingDown,

    // IO
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
}

impl Error {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Error::Connection(_)
                | Error::ConnectionTimeout { .. }
                | Error::Transport(_)
                | Error::Storage(_)
                | Error::Cache(_)
                | Error::Io(_)
        )
    }

    pub fn status_code(&self) -> u16 {
        match self {
            Error::AuthFailed(_) => 401,
            Error::Unauthorized(_) => 401,
            Error::Forbidden(_) => 403,
            Error::SessionNotFound(_) | Error::PluginNotFound(_) => 404,
            Error::RateLimited(_) | Error::IpBanned { .. } => 429,
            Error::ConfigValidation { .. } | Error::InvalidMessage(_) => 400,
            Error::MessageTooLarge { .. } => 413,
            Error::MaxConnectionsReached { .. } => 503,
            Error::ShuttingDown => 503,
            _ => 500,
        }
    }
}
