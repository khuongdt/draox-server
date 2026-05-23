use crate::marketplace_registry::MarketplaceRegistry;

/// Parsed semantic version (major.minor.patch).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SemVer {
    major: u64,
    minor: u64,
    patch: u64,
}

impl SemVer {
    /// Parse a version string such as "1.2.3", "1.2", or "1".
    fn parse(s: &str) -> Option<Self> {
        // Strip a leading 'v' if present.
        let s = s.trim_start_matches('v');
        let parts: Vec<&str> = s.splitn(3, '.').collect();
        let major = parts.first().and_then(|p| p.parse().ok())?;
        let minor = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(0);
        let patch = parts.get(2).and_then(|p| p.parse().ok()).unwrap_or(0);
        Some(SemVer { major, minor, patch })
    }
}

/// Semantic version resolution for plugin dependencies.
///
/// Supports the following version operators in a requirement string:
/// - `=1.2.3`  — exact match
/// - `>=1.2.3` — greater than or equal
/// - `<=1.2.3` — less than or equal
/// - `>1.2.3`  — strictly greater than
/// - `<1.2.3`  — strictly less than
/// - `^1.2.3`  — compatible with (same major, version >= given)
/// - `~1.2.3`  — patch-compatible (same major.minor, version >= given)
/// - `*`       — any version
/// - bare version (no operator) — treated as `>=version`
pub struct VersionResolver;

impl VersionResolver {
    /// Return `true` if `version` satisfies `requirement`.
    pub fn matches(version: &str, requirement: &str) -> bool {
        let req = requirement.trim();

        if req == "*" {
            return true;
        }

        // Detect operator prefix.
        let (op, ver_str) = if let Some(rest) = req.strip_prefix(">=") {
            (">=", rest)
        } else if let Some(rest) = req.strip_prefix("<=") {
            ("<=", rest)
        } else if let Some(rest) = req.strip_prefix('>') {
            (">", rest)
        } else if let Some(rest) = req.strip_prefix('<') {
            ("<", rest)
        } else if let Some(rest) = req.strip_prefix('=') {
            ("=", rest)
        } else if let Some(rest) = req.strip_prefix('^') {
            ("^", rest)
        } else if let Some(rest) = req.strip_prefix('~') {
            ("~", rest)
        } else {
            // No operator → treat as >=
            (">=", req)
        };

        let v = match SemVer::parse(version) {
            Some(v) => v,
            None => return false,
        };
        let r = match SemVer::parse(ver_str) {
            Some(r) => r,
            None => return false,
        };

        match op {
            "=" => v == r,
            ">=" => v >= r,
            "<=" => v <= r,
            ">" => v > r,
            "<" => v < r,
            "^" => {
                // Compatible: same major, v >= r
                v.major == r.major && v >= r
            }
            "~" => {
                // Patch-compatible: same major.minor, v >= r
                v.major == r.major && v.minor == r.minor && v >= r
            }
            _ => false,
        }
    }

    /// From a list of version strings, return the highest one that satisfies
    /// `requirement`, or `None` if none qualify.
    pub fn resolve_best(available: &[&str], requirement: &str) -> Option<String> {
        let mut candidates: Vec<SemVer> = available
            .iter()
            .filter(|v| Self::matches(v, requirement))
            .filter_map(|v| SemVer::parse(v).map(|sv| sv))
            .collect();

        candidates.sort();
        // Return the highest (last after sort).
        candidates.last().map(|sv| format!("{}.{}.{}", sv.major, sv.minor, sv.patch))
    }

    /// Return `true` if `server_version` satisfies the plugin's
    /// `plugin_requirement` for the minimum server version.
    pub fn is_compatible(server_version: &str, plugin_requirement: &str) -> bool {
        Self::matches(server_version, plugin_requirement)
    }

    /// Resolve all transitive dependencies for `plugin_id` from the registry
    /// and return an ordered install list (dependencies first, BFS order).
    ///
    /// Returns `Err` if a required plugin or version cannot be found, or if a
    /// circular dependency is detected.
    pub fn resolve_dependencies(
        plugin_id: &str,
        registry: &MarketplaceRegistry,
    ) -> Result<Vec<String>, String> {
        let mut ordered: Vec<String> = Vec::new();
        let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
        let _stack: Vec<String> = vec![plugin_id.to_string()];
        let mut in_stack: std::collections::HashSet<String> = std::collections::HashSet::new();

        Self::resolve_recursive(plugin_id, registry, &mut ordered, &mut visited, &mut in_stack)?;

        // Remove the root plugin itself — callers only need the dependency list.
        ordered.retain(|id| id != plugin_id);
        Ok(ordered)
    }

    fn resolve_recursive(
        plugin_id: &str,
        registry: &MarketplaceRegistry,
        ordered: &mut Vec<String>,
        visited: &mut std::collections::HashSet<String>,
        in_stack: &mut std::collections::HashSet<String>,
    ) -> Result<(), String> {
        if in_stack.contains(plugin_id) {
            return Err(format!("circular dependency detected involving '{plugin_id}'"));
        }
        if visited.contains(plugin_id) {
            return Ok(());
        }

        in_stack.insert(plugin_id.to_string());

        let latest = registry
            .get_latest_version(plugin_id)
            .ok_or_else(|| format!("no version found for plugin '{plugin_id}'"))?;

        for dep in &latest.dependencies {
            let dep_versions = registry.get_versions(&dep.plugin_id);
            if dep_versions.is_empty() {
                return Err(format!(
                    "dependency '{}' of '{}' not found in registry",
                    dep.plugin_id, plugin_id
                ));
            }
            let available: Vec<&str> = dep_versions.iter().map(|v| v.version.as_str()).collect();
            if Self::resolve_best(&available, &dep.version_requirement).is_none() {
                return Err(format!(
                    "no version of '{}' satisfies requirement '{}'",
                    dep.plugin_id, dep.version_requirement
                ));
            }
            Self::resolve_recursive(&dep.plugin_id, registry, ordered, visited, in_stack)?;
        }

        in_stack.remove(plugin_id);
        visited.insert(plugin_id.to_string());
        ordered.push(plugin_id.to_string());
        Ok(())
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::marketplace_registry::MarketplaceRegistry;
    use crate::marketplace_types::{
        MarketplacePlugin, PluginCategory, PluginDependency, PluginVersion, PublisherInfo,
    };
    use chrono::Utc;

    // ── matches() ────────────────────────────────────────────────────────────

    #[test]
    fn test_exact_match() {
        assert!(VersionResolver::matches("1.2.3", "=1.2.3"));
        assert!(!VersionResolver::matches("1.2.4", "=1.2.3"));
    }

    #[test]
    fn test_gte_lte_gt_lt() {
        assert!(VersionResolver::matches("2.0.0", ">=1.0.0"));
        assert!(VersionResolver::matches("1.0.0", ">=1.0.0"));
        assert!(!VersionResolver::matches("0.9.9", ">=1.0.0"));

        assert!(VersionResolver::matches("0.9.0", "<=1.0.0"));
        assert!(!VersionResolver::matches("1.0.1", "<=1.0.0"));

        assert!(VersionResolver::matches("1.0.1", ">1.0.0"));
        assert!(!VersionResolver::matches("1.0.0", ">1.0.0"));

        assert!(VersionResolver::matches("0.9.9", "<1.0.0"));
        assert!(!VersionResolver::matches("1.0.0", "<1.0.0"));
    }

    #[test]
    fn test_caret_compatible() {
        // ^1.2.0 means >=1.2.0 and <2.0.0 (same major)
        assert!(VersionResolver::matches("1.5.0", "^1.2.0"));
        assert!(VersionResolver::matches("1.2.0", "^1.2.0"));
        assert!(!VersionResolver::matches("2.0.0", "^1.2.0"));
        assert!(!VersionResolver::matches("1.1.9", "^1.2.0"));
    }

    #[test]
    fn test_tilde_patch_compatible() {
        // ~1.2.3 means >=1.2.3 and same major.minor
        assert!(VersionResolver::matches("1.2.5", "~1.2.3"));
        assert!(VersionResolver::matches("1.2.3", "~1.2.3"));
        assert!(!VersionResolver::matches("1.3.0", "~1.2.3"));
        assert!(!VersionResolver::matches("1.2.2", "~1.2.3"));
    }

    #[test]
    fn test_wildcard_any() {
        assert!(VersionResolver::matches("99.99.99", "*"));
        assert!(VersionResolver::matches("0.0.1", "*"));
    }

    #[test]
    fn test_bare_version_treated_as_gte() {
        assert!(VersionResolver::matches("1.5.0", "1.2.0"));
        assert!(!VersionResolver::matches("1.0.0", "1.2.0"));
    }

    // ── resolve_best() ───────────────────────────────────────────────────────

    #[test]
    fn test_resolve_best() {
        let available = &["1.0.0", "1.2.0", "1.3.5", "2.0.0"];
        assert_eq!(
            VersionResolver::resolve_best(available, "^1.0.0"),
            Some("1.3.5".to_string())
        );
        assert_eq!(
            VersionResolver::resolve_best(available, ">=2.0.0"),
            Some("2.0.0".to_string())
        );
        assert_eq!(VersionResolver::resolve_best(available, ">=3.0.0"), None);
    }

    // ── resolve_dependencies() ───────────────────────────────────────────────

    fn make_plugin_entry(id: &str) -> MarketplacePlugin {
        MarketplacePlugin {
            id: id.to_string(),
            name: id.to_string(),
            description: String::new(),
            version: "1.0.0".to_string(),
            author: PublisherInfo {
                id: "pub".to_string(),
                name: "Publisher".to_string(),
                email: None,
                verified: true,
                joined_at: Utc::now(),
            },
            category: PluginCategory::Utility,
            tags: vec![],
            downloads: 0,
            active_installs: 0,
            rating: 0.0,
            rating_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            is_paid: false,
            price_cents: None,
            featured: false,
            compatibility: ">=1.0.0".to_string(),
        }
    }

    fn add_plugin_with_deps(
        registry: &MarketplaceRegistry,
        id: &str,
        deps: Vec<(&str, &str)>,
    ) {
        registry.publish(make_plugin_entry(id)).unwrap();
        registry.add_version(
            id,
            PluginVersion {
                version: "1.0.0".to_string(),
                release_notes: String::new(),
                min_server_version: "1.0.0".to_string(),
                published_at: Utc::now(),
                download_url: String::new(),
                checksum: String::new(),
                size_bytes: 0,
                dependencies: deps
                    .into_iter()
                    .map(|(pid, req)| PluginDependency {
                        plugin_id: pid.to_string(),
                        version_requirement: req.to_string(),
                    })
                    .collect(),
            },
        );
    }

    #[test]
    fn test_resolve_no_deps() {
        let registry = MarketplaceRegistry::new();
        add_plugin_with_deps(&registry, "io.draox.a", vec![]);
        let result = VersionResolver::resolve_dependencies("io.draox.a", &registry).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_resolve_simple_chain() {
        let registry = MarketplaceRegistry::new();
        // C has no deps; B depends on C; A depends on B
        add_plugin_with_deps(&registry, "io.draox.c", vec![]);
        add_plugin_with_deps(&registry, "io.draox.b", vec![("io.draox.c", ">=1.0.0")]);
        add_plugin_with_deps(&registry, "io.draox.a", vec![("io.draox.b", ">=1.0.0")]);

        let order =
            VersionResolver::resolve_dependencies("io.draox.a", &registry).unwrap();
        assert_eq!(order, vec!["io.draox.c", "io.draox.b"]);
    }

    #[test]
    fn test_resolve_missing_dependency_fails() {
        let registry = MarketplaceRegistry::new();
        add_plugin_with_deps(
            &registry,
            "io.draox.a",
            vec![("io.draox.missing", ">=1.0.0")],
        );
        let result = VersionResolver::resolve_dependencies("io.draox.a", &registry);
        assert!(result.is_err());
    }
}
