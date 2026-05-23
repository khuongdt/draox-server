use crate::model::DraoxConfig;
use crate::validation;
use server_core::{Error, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::watch;
use tracing::{debug, info};

/// Loads and manages configuration with environment variable overrides.
#[derive(Debug)]
pub struct ConfigLoader {
    path: PathBuf,
    config: Arc<DraoxConfig>,
    tx: watch::Sender<Arc<DraoxConfig>>,
    rx: watch::Receiver<Arc<DraoxConfig>>,
}

impl ConfigLoader {
    /// Load config from a TOML file, applying env var overrides.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        info!(path = %path.display(), "loading configuration");

        let content = std::fs::read_to_string(&path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Error::ConfigNotFound {
                    path: path.display().to_string(),
                }
            } else {
                Error::Config(format!("failed to read {}: {e}", path.display()))
            }
        })?;

        let mut config: DraoxConfig = toml::from_str(&content)
            .map_err(|e| Error::Config(format!("TOML parse error in {}: {e}", path.display())))?;

        apply_env_overrides(&mut config);
        validation::validate(&config)?;

        let config = Arc::new(config);
        let (tx, rx) = watch::channel(Arc::clone(&config));

        info!("configuration loaded successfully");
        Ok(Self { path, config, tx, rx })
    }

    /// Load from a TOML string (useful for tests).
    pub fn from_str(toml_content: &str) -> Result<Self> {
        let mut config: DraoxConfig = toml::from_str(toml_content)
            .map_err(|e| Error::Config(format!("TOML parse error: {e}")))?;

        apply_env_overrides(&mut config);
        validation::validate(&config)?;

        let config = Arc::new(config);
        let (tx, rx) = watch::channel(Arc::clone(&config));

        Ok(Self {
            path: PathBuf::new(),
            config,
            tx,
            rx,
        })
    }

    /// Load default configuration (all defaults).
    pub fn default_config() -> Self {
        let config = Arc::new(DraoxConfig::default());
        let (tx, rx) = watch::channel(Arc::clone(&config));
        Self {
            path: PathBuf::new(),
            config,
            tx,
            rx,
        }
    }

    /// Get current config.
    pub fn config(&self) -> &Arc<DraoxConfig> {
        &self.config
    }

    /// Subscribe to config changes (for hot-reload).
    pub fn subscribe(&self) -> watch::Receiver<Arc<DraoxConfig>> {
        self.rx.clone()
    }

    /// Reload config from the original file.
    pub fn reload(&mut self) -> Result<()> {
        if self.path.as_os_str().is_empty() {
            return Err(Error::Config("no config file path set".to_string()));
        }

        info!(path = %self.path.display(), "reloading configuration");

        let content = std::fs::read_to_string(&self.path)
            .map_err(|e| Error::Config(format!("failed to read {}: {e}", self.path.display())))?;

        let mut new_config: DraoxConfig = toml::from_str(&content)
            .map_err(|e| Error::Config(format!("TOML parse error: {e}")))?;

        apply_env_overrides(&mut new_config);
        validation::validate(&new_config)?;

        let new_config = Arc::new(new_config);
        self.config = Arc::clone(&new_config);
        let _ = self.tx.send(new_config);

        info!("configuration reloaded successfully");
        Ok(())
    }

    /// Get the config file path.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Apply environment variable overrides with DRAOX_ prefix.
/// Convention: DRAOX_SECTION_KEY → config.section.key
fn apply_env_overrides(config: &mut DraoxConfig) {
    if let Ok(val) = std::env::var("DRAOX_SERVER_HOST") {
        debug!(key = "DRAOX_SERVER_HOST", value = %val, "env override");
        config.server.host = val;
    }
    if let Ok(val) = std::env::var("DRAOX_SERVER_NAME") {
        config.server.name = val;
    }
    if let Ok(val) = std::env::var("DRAOX_SERVER_MAX_CONNECTIONS") {
        if let Ok(v) = val.parse() {
            config.server.max_connections = v;
        }
    }

    // TCP
    if let Ok(val) = std::env::var("DRAOX_TCP_PORT") {
        if let Ok(v) = val.parse() {
            config.tcp.port = v;
        }
    }
    if let Ok(val) = std::env::var("DRAOX_TCP_ENABLED") {
        config.tcp.enabled = val == "true" || val == "1";
    }

    // UDP
    if let Ok(val) = std::env::var("DRAOX_UDP_PORT") {
        if let Ok(v) = val.parse() {
            config.udp.port = v;
        }
    }
    if let Ok(val) = std::env::var("DRAOX_UDP_ENABLED") {
        config.udp.enabled = val == "true" || val == "1";
    }

    // WebSocket
    if let Ok(val) = std::env::var("DRAOX_WEBSOCKET_PORT") {
        if let Ok(v) = val.parse() {
            config.websocket.port = v;
        }
    }

    // HTTP
    if let Ok(val) = std::env::var("DRAOX_HTTP_PORT") {
        if let Ok(v) = val.parse() {
            config.http.port = v;
        }
    }

    // Admin API
    if let Ok(val) = std::env::var("DRAOX_ADMIN_PORT") {
        if let Ok(v) = val.parse() {
            config.admin_api.port = v;
        }
    }
    if let Ok(val) = std::env::var("DRAOX_ADMIN_JWT_SECRET") {
        config.admin_api.jwt_secret = val;
    }

    // Storage
    if let Ok(val) = std::env::var("DRAOX_DATABASE_URL") {
        config.storage.sql.url = val;
    }
    if let Ok(val) = std::env::var("DRAOX_MONGODB_URL") {
        config.storage.mongodb.url = val;
    }

    // Cache
    if let Ok(val) = std::env::var("DRAOX_REDIS_URL") {
        config.cache.redis.url = val;
    }

    // Logging
    if let Ok(val) = std::env::var("DRAOX_LOG_LEVEL") {
        config.logging.level = val;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_load_from_string() {
        let toml = r#"
[server]
name = "Test Server"

[admin_api]
jwt_secret = "test-secret"
"#;
        let loader = ConfigLoader::from_str(toml).unwrap();
        assert_eq!(loader.config().server.name, "Test Server");
    }

    #[test]
    fn test_load_from_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        write!(
            f,
            r#"
[server]
name = "File Test"

[admin_api]
jwt_secret = "test-secret"
"#
        )
        .unwrap();

        let loader = ConfigLoader::from_file(&path).unwrap();
        assert_eq!(loader.config().server.name, "File Test");
    }

    #[test]
    fn test_file_not_found() {
        let result = ConfigLoader::from_file("/nonexistent/path.toml");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::ConfigNotFound { .. }));
    }

    #[test]
    fn test_invalid_toml() {
        let result = ConfigLoader::from_str("this is not [valid toml");
        assert!(result.is_err());
    }

    #[test]
    fn test_default_config() {
        let loader = ConfigLoader::default_config();
        assert_eq!(loader.config().server.name, "Draox Server");
    }

    #[test]
    fn test_config_subscribe() {
        let loader = ConfigLoader::default_config();
        let rx = loader.subscribe();
        assert_eq!(rx.borrow().server.name, "Draox Server");
    }
}
