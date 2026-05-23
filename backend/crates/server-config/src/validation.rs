use crate::model::DraoxConfig;
use server_core::Error;

/// Validate a DraoxConfig, returning all validation errors at once.
pub fn validate(config: &DraoxConfig) -> server_core::Result<()> {
    let mut errors = Vec::new();

    // Server
    if config.server.max_connections == 0 {
        errors.push(("server.max_connections", "must be > 0"));
    }
    if config.server.shutdown_timeout_secs == 0 {
        errors.push(("server.shutdown_timeout_secs", "must be > 0"));
    }

    // TCP
    if config.tcp.enabled && config.tcp.port == 0 {
        errors.push(("tcp.port", "must be > 0 when TCP is enabled"));
    }

    // UDP
    if config.udp.enabled && config.udp.port == 0 {
        errors.push(("udp.port", "must be > 0 when UDP is enabled"));
    }

    // WebSocket
    if config.websocket.enabled {
        if config.websocket.port == 0 {
            errors.push(("websocket.port", "must be > 0 when WebSocket is enabled"));
        }
        if config.websocket.max_frame_size == 0 {
            errors.push(("websocket.max_frame_size", "must be > 0"));
        }
    }

    // HTTP
    if config.http.enabled && config.http.port == 0 {
        errors.push(("http.port", "must be > 0 when HTTP is enabled"));
    }

    // TLS
    if config.tls.enabled {
        if config.tls.cert_path.as_os_str().is_empty() {
            errors.push(("tls.cert_path", "required when TLS is enabled"));
        }
        if config.tls.key_path.as_os_str().is_empty() {
            errors.push(("tls.key_path", "required when TLS is enabled"));
        }
    }

    // Traffic guard
    if config.traffic_guard.enabled {
        let tg = &config.traffic_guard;
        if tg.connection_limits.max_connections_per_ip == 0 {
            errors.push(("traffic_guard.connection_limits.max_connections_per_ip", "must be > 0"));
        }
        if tg.banning.enabled && tg.banning.initial_ban_duration_secs == 0 {
            errors.push(("traffic_guard.banning.initial_ban_duration_secs", "must be > 0"));
        }
        if tg.ip_reputation.enabled && tg.ip_reputation.initial_score == 0 {
            errors.push(("traffic_guard.ip_reputation.initial_score", "must be > 0"));
        }
        if tg.adaptive.throttle_factor <= 0.0 || tg.adaptive.throttle_factor > 1.0 {
            errors.push(("traffic_guard.adaptive.throttle_factor", "must be in (0.0, 1.0]"));
        }
    }

    // Sessions
    if config.sessions.max_connections_per_session == 0 {
        errors.push(("sessions.max_connections_per_session", "must be > 0"));
    }

    // Admin API
    if config.admin_api.enabled {
        if config.admin_api.port == 0 {
            errors.push(("admin_api.port", "must be > 0 when admin API is enabled"));
        }
        if config.admin_api.auth_method == "jwt" && config.admin_api.jwt_secret.is_empty() {
            errors.push(("admin_api.jwt_secret", "required when auth_method is 'jwt'"));
        }
    }

    // Port collision check
    let mut ports: Vec<(&str, u16, bool)> = vec![
        ("tcp", config.tcp.port, config.tcp.enabled),
        ("udp", config.udp.port, config.udp.enabled),
        ("websocket", config.websocket.port, config.websocket.enabled),
        ("http", config.http.port, config.http.enabled),
        ("admin_api", config.admin_api.port, config.admin_api.enabled),
    ];
    ports.retain(|(_, _, enabled)| *enabled);
    for i in 0..ports.len() {
        for j in (i + 1)..ports.len() {
            if ports[i].1 == ports[j].1 {
                errors.push(("ports", "port collision detected between services"));
                break;
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        let msg = errors
            .iter()
            .map(|(field, reason)| format!("  - {field}: {reason}"))
            .collect::<Vec<_>>()
            .join("\n");
        Err(Error::Config(format!("configuration validation failed:\n{msg}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_passes_validation() {
        let mut config = DraoxConfig::default();
        // jwt_secret is required for default config
        config.admin_api.jwt_secret = "test-secret-key".to_string();
        assert!(validate(&config).is_ok());
    }

    #[test]
    fn test_port_collision_detected() {
        let mut config = DraoxConfig::default();
        config.admin_api.jwt_secret = "test-secret-key".to_string();
        config.tcp.port = 9000;
        config.udp.port = 9000; // collision
        assert!(validate(&config).is_err());
    }

    #[test]
    fn test_zero_max_connections_fails() {
        let mut config = DraoxConfig::default();
        config.admin_api.jwt_secret = "test-secret-key".to_string();
        config.server.max_connections = 0;
        let err = validate(&config).unwrap_err();
        assert!(err.to_string().contains("max_connections"));
    }
}
