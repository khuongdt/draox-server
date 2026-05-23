use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// All permissions a plugin may request in its manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PluginPermission {
    /// Read/write to the server's data store (SQL/NoSQL).
    Storage,
    /// Access the shared cache layer (Redis / in-memory).
    Cache,
    /// Inspect or manage client connections.
    Connections,
    /// Subscribe to and emit server events via the event bus.
    Events,
    /// Make outbound network requests.
    Network,
    /// Read or write files on the host filesystem.
    FileSystem,
    /// Schedule recurring or deferred tasks.
    Scheduler,
    /// Call administrative endpoints or modify server settings.
    Admin,
}

/// Enforces plugin permissions declared in manifests at runtime.
///
/// Internally uses a `DashMap` so all operations are lock-free and
/// safe to call concurrently from multiple Tokio tasks.
pub struct PermissionEnforcer {
    /// plugin_id -> set of granted permissions
    granted: DashMap<String, HashSet<PluginPermission>>,
}

impl PermissionEnforcer {
    pub fn new() -> Self {
        Self {
            granted: DashMap::new(),
        }
    }

    /// Replace the full permission set for `plugin_id`.
    pub fn grant(&self, plugin_id: &str, permissions: HashSet<PluginPermission>) {
        self.granted.insert(plugin_id.to_string(), permissions);
    }

    /// Returns `true` if `plugin_id` has been granted `permission`.
    pub fn check(&self, plugin_id: &str, permission: &PluginPermission) -> bool {
        self.granted
            .get(plugin_id)
            .map(|perms| perms.contains(permission))
            .unwrap_or(false)
    }

    /// Remove a single permission from `plugin_id`'s grant set.
    /// No-op if the plugin has no grants or doesn't hold the permission.
    pub fn revoke(&self, plugin_id: &str, permission: &PluginPermission) {
        if let Some(mut perms) = self.granted.get_mut(plugin_id) {
            perms.remove(permission);
        }
    }

    /// Remove all permissions for `plugin_id` (e.g. on plugin deactivation).
    pub fn revoke_all(&self, plugin_id: &str) {
        self.granted.remove(plugin_id);
    }

    /// Return a copy of the current permission set for `plugin_id`.
    /// Returns an empty set if the plugin has not been granted any permissions.
    pub fn get_permissions(&self, plugin_id: &str) -> HashSet<PluginPermission> {
        self.granted
            .get(plugin_id)
            .map(|p| p.clone())
            .unwrap_or_default()
    }
}

impl Default for PermissionEnforcer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enforcer_with_plugin(plugin_id: &str, perms: &[PluginPermission]) -> PermissionEnforcer {
        let enforcer = PermissionEnforcer::new();
        enforcer.grant(plugin_id, perms.iter().cloned().collect());
        enforcer
    }

    #[test]
    fn test_grant_and_check_permission() {
        let enforcer =
            enforcer_with_plugin("io.draox.clans", &[PluginPermission::Storage, PluginPermission::Events]);

        assert!(enforcer.check("io.draox.clans", &PluginPermission::Storage));
        assert!(enforcer.check("io.draox.clans", &PluginPermission::Events));
        assert!(!enforcer.check("io.draox.clans", &PluginPermission::Admin));
    }

    #[test]
    fn test_check_unknown_plugin_returns_false() {
        let enforcer = PermissionEnforcer::new();
        assert!(!enforcer.check("unknown.plugin", &PluginPermission::Network));
    }

    #[test]
    fn test_revoke_single_permission() {
        let enforcer = enforcer_with_plugin(
            "io.draox.messaging",
            &[PluginPermission::Cache, PluginPermission::Network],
        );

        enforcer.revoke("io.draox.messaging", &PluginPermission::Network);

        assert!(enforcer.check("io.draox.messaging", &PluginPermission::Cache));
        assert!(!enforcer.check("io.draox.messaging", &PluginPermission::Network));
    }

    #[test]
    fn test_revoke_all_removes_every_permission() {
        let enforcer = enforcer_with_plugin(
            "io.draox.clans",
            &[
                PluginPermission::Storage,
                PluginPermission::Cache,
                PluginPermission::Admin,
            ],
        );

        enforcer.revoke_all("io.draox.clans");

        assert!(!enforcer.check("io.draox.clans", &PluginPermission::Storage));
        assert!(!enforcer.check("io.draox.clans", &PluginPermission::Cache));
        assert!(!enforcer.check("io.draox.clans", &PluginPermission::Admin));
        assert!(enforcer.get_permissions("io.draox.clans").is_empty());
    }

    #[test]
    fn test_get_permissions_returns_full_set() {
        let expected: HashSet<PluginPermission> = [
            PluginPermission::Storage,
            PluginPermission::Scheduler,
            PluginPermission::Connections,
        ]
        .iter()
        .cloned()
        .collect();

        let enforcer = PermissionEnforcer::new();
        enforcer.grant("io.draox.a", expected.clone());

        assert_eq!(enforcer.get_permissions("io.draox.a"), expected);
    }

    #[test]
    fn test_multiple_plugins_isolated() {
        let enforcer = PermissionEnforcer::new();
        enforcer.grant(
            "io.draox.a",
            [PluginPermission::Admin].iter().cloned().collect(),
        );
        enforcer.grant(
            "io.draox.b",
            [PluginPermission::FileSystem].iter().cloned().collect(),
        );

        // Plugin A should not see plugin B's permissions and vice-versa.
        assert!(!enforcer.check("io.draox.a", &PluginPermission::FileSystem));
        assert!(!enforcer.check("io.draox.b", &PluginPermission::Admin));

        assert!(enforcer.check("io.draox.a", &PluginPermission::Admin));
        assert!(enforcer.check("io.draox.b", &PluginPermission::FileSystem));
    }
}
