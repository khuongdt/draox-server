//! Well-known cache key patterns for the Draox server.
//!
//! [`CacheKeys`] centralises key construction so all subsystems use a
//! consistent naming scheme. Using structured keys prevents collisions between
//! subsystems and makes it easy to reason about the cache contents.
//!
//! # Format convention
//!
//! Keys follow the pattern `<domain>:<sub-domain>:<id>`, e.g.
//! - `session:abc123`
//! - `auth:token:sha256_hash`
//! - `billing:quota:client_99`

/// Factory for well-known cache keys used across the Draox server.
///
/// All methods are pure functions that return owned [`String`] values.  They
/// are intentionally free of any external I/O so they can be used everywhere
/// without async overhead.
pub struct CacheKeys;

impl CacheKeys {
    // ── Session ───────────────────────────────────────────────────────────────

    /// Key for a client session record.
    ///
    /// Example: `session:s_abc123`
    pub fn session(id: &str) -> String {
        format!("session:{id}")
    }

    // ── Plugin ────────────────────────────────────────────────────────────────

    /// Key for a plugin's persisted runtime state.
    ///
    /// Example: `plugin:com.example.myplugin:state`
    pub fn plugin_state(id: &str) -> String {
        format!("plugin:{id}:state")
    }

    // ── Auth ──────────────────────────────────────────────────────────────────

    /// Key for a validated authentication token (stored by its hash).
    ///
    /// Example: `auth:token:sha256abc…`
    pub fn auth_token(token_hash: &str) -> String {
        format!("auth:token:{token_hash}")
    }

    // ── Rate limiting ─────────────────────────────────────────────────────────

    /// Key for per-IP rate-limit counters.
    ///
    /// Example: `rate:192.168.1.1`
    pub fn rate_limit(ip: &str) -> String {
        format!("rate:{ip}")
    }

    // ── Connection ────────────────────────────────────────────────────────────

    /// Key for an active connection record.
    ///
    /// Example: `conn:conn_xyz789`
    pub fn connection(id: &str) -> String {
        format!("conn:{id}")
    }

    // ── Health ────────────────────────────────────────────────────────────────

    /// Key for a component's last-known health status.
    ///
    /// Example: `health:database`
    pub fn health(component: &str) -> String {
        format!("health:{component}")
    }

    // ── Billing ───────────────────────────────────────────────────────────────

    /// Key for a client's current billing quota snapshot.
    ///
    /// Example: `billing:quota:client_42`
    pub fn billing_quota(client_id: &str) -> String {
        format!("billing:quota:{client_id}")
    }

    // ── Clans ─────────────────────────────────────────────────────────────────

    /// Key for a clan record.
    ///
    /// Example: `clan:clan_01`
    pub fn clan(id: &str) -> String {
        format!("clan:{id}")
    }

    // ── Message queue ─────────────────────────────────────────────────────────

    /// Key for a user's pending message queue.
    ///
    /// Example: `mq:user_999`
    pub fn message_queue(user_id: &str) -> String {
        format!("mq:{user_id}")
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_key_formats() {
        assert_eq!(CacheKeys::session("s1"), "session:s1");
        assert_eq!(CacheKeys::plugin_state("com.acme.plugin"), "plugin:com.acme.plugin:state");
        assert_eq!(CacheKeys::auth_token("abc123"), "auth:token:abc123");
        assert_eq!(CacheKeys::rate_limit("10.0.0.1"), "rate:10.0.0.1");
        assert_eq!(CacheKeys::connection("c99"), "conn:c99");
        assert_eq!(CacheKeys::health("database"), "health:database");
        assert_eq!(CacheKeys::billing_quota("client_7"), "billing:quota:client_7");
        assert_eq!(CacheKeys::clan("clan_1"), "clan:clan_1");
        assert_eq!(CacheKeys::message_queue("user_42"), "mq:user_42");
    }

    #[test]
    fn test_all_keys_are_unique_for_same_id() {
        // All key builders for the same ID should produce distinct strings,
        // ensuring there is no accidental namespace collision.
        let id = "same_id";
        let keys = vec![
            CacheKeys::session(id),
            CacheKeys::plugin_state(id),
            CacheKeys::auth_token(id),
            CacheKeys::rate_limit(id),
            CacheKeys::connection(id),
            CacheKeys::health(id),
            CacheKeys::billing_quota(id),
            CacheKeys::clan(id),
            CacheKeys::message_queue(id),
        ];

        let unique: HashSet<_> = keys.iter().collect();
        assert_eq!(
            unique.len(),
            keys.len(),
            "two or more key builders produce the same key for id={id:?}: {keys:?}"
        );
    }

    #[test]
    fn test_keys_contain_id() {
        let id = "test_entity";
        assert!(CacheKeys::session(id).contains(id));
        assert!(CacheKeys::clan(id).contains(id));
        assert!(CacheKeys::message_queue(id).contains(id));
    }
}
