use dashmap::DashMap;
use serde::{Deserialize, Serialize};

/// Describes a single HTTP route contributed by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDefinition {
    /// HTTP method (e.g. "GET", "POST", "PUT", "DELETE").
    pub method: String,
    /// Route path, e.g. "/api/clans/{id}".
    pub path: String,
    /// ID of the plugin that owns this route.
    pub plugin_id: String,
    /// Human-readable description shown in documentation.
    pub description: Option<String>,
}

/// Registry of routes contributed by plugins.
///
/// Supports conflict detection, per-plugin bulk removal, and
/// O(1) look-up by method + path.
pub struct RouteRegistry {
    /// key = "METHOD /path"
    routes: DashMap<String, RouteDefinition>,
    /// plugin_id -> list of route keys registered by that plugin
    plugin_routes: DashMap<String, Vec<String>>,
}

impl RouteRegistry {
    pub fn new() -> Self {
        Self {
            routes: DashMap::new(),
            plugin_routes: DashMap::new(),
        }
    }

    /// Canonical key for a route.
    fn key(method: &str, path: &str) -> String {
        format!("{} {}", method.to_uppercase(), path)
    }

    /// Register a route for a plugin.
    ///
    /// Returns `Err(String)` if another plugin already owns the same method+path.
    pub fn register(&self, plugin_id: &str, definition: RouteDefinition) -> Result<(), String> {
        let k = Self::key(&definition.method, &definition.path);

        if let Some(existing) = self.routes.get(&k) {
            if existing.plugin_id != plugin_id {
                return Err(format!(
                    "route conflict: {} {} is already owned by plugin '{}'",
                    definition.method, definition.path, existing.plugin_id
                ));
            }
            // Same plugin registering the same route again is a no-op / update.
        }

        self.routes.insert(k.clone(), definition);
        self.plugin_routes
            .entry(plugin_id.to_string())
            .or_default()
            .push(k);

        Ok(())
    }

    /// Unregister a single route owned by `plugin_id`.
    ///
    /// Returns `true` if the route was found and removed.
    pub fn unregister(&self, plugin_id: &str, method: &str, path: &str) -> bool {
        let k = Self::key(method, path);

        // Only remove if owned by this plugin.
        let removed = self
            .routes
            .remove_if(&k, |_, def| def.plugin_id == plugin_id)
            .is_some();

        if removed {
            if let Some(mut keys) = self.plugin_routes.get_mut(plugin_id) {
                keys.retain(|existing_key| existing_key != &k);
            }
        }

        removed
    }

    /// Remove all routes registered by `plugin_id`.
    ///
    /// Returns the number of routes that were removed.
    pub fn unregister_all(&self, plugin_id: &str) -> usize {
        let keys = match self.plugin_routes.remove(plugin_id) {
            Some((_, keys)) => keys,
            None => return 0,
        };

        let count = keys.len();
        for k in &keys {
            self.routes.remove(k);
        }
        count
    }

    /// All routes registered by `plugin_id`.
    pub fn get_routes(&self, plugin_id: &str) -> Vec<RouteDefinition> {
        let keys = match self.plugin_routes.get(plugin_id) {
            Some(k) => k.clone(),
            None => return Vec::new(),
        };

        keys.iter()
            .filter_map(|k| self.routes.get(k).map(|r| r.clone()))
            .collect()
    }

    /// All routes across every plugin.
    pub fn all_routes(&self) -> Vec<RouteDefinition> {
        self.routes.iter().map(|r| r.value().clone()).collect()
    }

    /// Find a specific route by method and path.
    pub fn find_route(&self, method: &str, path: &str) -> Option<RouteDefinition> {
        let k = Self::key(method, path);
        self.routes.get(&k).map(|r| r.clone())
    }

    /// Returns `true` if the given method+path is already occupied.
    pub fn has_conflict(&self, method: &str, path: &str) -> bool {
        let k = Self::key(method, path);
        self.routes.contains_key(&k)
    }
}

impl Default for RouteRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_route(method: &str, path: &str, plugin_id: &str) -> RouteDefinition {
        RouteDefinition {
            method: method.to_string(),
            path: path.to_string(),
            plugin_id: plugin_id.to_string(),
            description: None,
        }
    }

    #[test]
    fn test_register_and_find_route() {
        let rr = RouteRegistry::new();
        let route = make_route("GET", "/api/clans", "io.draox.clans");

        rr.register("io.draox.clans", route).unwrap();

        let found = rr.find_route("GET", "/api/clans").unwrap();
        assert_eq!(found.plugin_id, "io.draox.clans");
        assert_eq!(found.path, "/api/clans");
    }

    #[test]
    fn test_conflict_detected_on_different_plugin() {
        let rr = RouteRegistry::new();
        rr.register("io.draox.a", make_route("POST", "/api/data", "io.draox.a"))
            .unwrap();

        let result =
            rr.register("io.draox.b", make_route("POST", "/api/data", "io.draox.b"));
        assert!(result.is_err(), "should report conflict");
        assert!(result.unwrap_err().contains("conflict"));
    }

    #[test]
    fn test_unregister_route() {
        let rr = RouteRegistry::new();
        rr.register("io.draox.clans", make_route("GET", "/api/clans/{id}", "io.draox.clans"))
            .unwrap();

        assert!(rr.has_conflict("GET", "/api/clans/{id}"));

        let removed = rr.unregister("io.draox.clans", "GET", "/api/clans/{id}");
        assert!(removed);
        assert!(!rr.has_conflict("GET", "/api/clans/{id}"));
    }

    #[test]
    fn test_unregister_all_returns_count() {
        let rr = RouteRegistry::new();
        let pid = "io.draox.messaging";
        rr.register(pid, make_route("GET", "/api/messages", pid)).unwrap();
        rr.register(pid, make_route("POST", "/api/messages", pid)).unwrap();
        rr.register(pid, make_route("DELETE", "/api/messages/{id}", pid))
            .unwrap();

        let count = rr.unregister_all(pid);
        assert_eq!(count, 3);
        assert_eq!(rr.get_routes(pid).len(), 0);
        assert_eq!(rr.all_routes().len(), 0);
    }

    #[test]
    fn test_get_routes_per_plugin() {
        let rr = RouteRegistry::new();
        rr.register("io.draox.a", make_route("GET", "/a/1", "io.draox.a")).unwrap();
        rr.register("io.draox.a", make_route("GET", "/a/2", "io.draox.a")).unwrap();
        rr.register("io.draox.b", make_route("GET", "/b/1", "io.draox.b")).unwrap();

        let a_routes = rr.get_routes("io.draox.a");
        assert_eq!(a_routes.len(), 2);

        let b_routes = rr.get_routes("io.draox.b");
        assert_eq!(b_routes.len(), 1);

        assert_eq!(rr.all_routes().len(), 3);
    }

    #[test]
    fn test_method_case_normalization() {
        let rr = RouteRegistry::new();
        rr.register("io.draox.a", make_route("get", "/api/items", "io.draox.a"))
            .unwrap();

        // Should find it with uppercase GET as well.
        assert!(rr.find_route("GET", "/api/items").is_some());
        assert!(rr.has_conflict("GET", "/api/items"));
    }

    #[test]
    fn test_same_plugin_re_register_is_ok() {
        let rr = RouteRegistry::new();
        let pid = "io.draox.a";
        rr.register(pid, make_route("GET", "/api/x", pid)).unwrap();
        // Same plugin registering the same route again should not error.
        let result = rr.register(pid, make_route("GET", "/api/x", pid));
        assert!(result.is_ok());
    }
}
