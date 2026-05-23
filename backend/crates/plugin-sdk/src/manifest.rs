use crate::traits::{ActivationEvent, PluginContributions, PluginPermissions};
use serde::{Deserialize, Serialize};
use server_core::{Error, PluginId, Result};
use std::collections::HashMap;
use std::path::Path;

/// Plugin manifest — parsed from `plugin.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Reverse-domain plugin ID (e.g., "io.draox.clans").
    pub id: String,

    /// Human-readable name.
    pub name: String,

    /// Semver version string.
    pub version: String,

    /// Plugin author.
    pub author: String,

    /// Short description.
    #[serde(default)]
    pub description: String,

    /// Plugin type: "builtin" or "wasm".
    #[serde(rename = "type", default = "default_plugin_type")]
    pub plugin_type: String,

    /// When to activate the plugin.
    #[serde(default = "default_activation")]
    pub activation: ActivationEvent,

    /// Minimum server version required.
    #[serde(default)]
    pub min_server_version: Option<String>,

    /// Plugin dependencies (plugin_id → version requirement).
    #[serde(default)]
    pub dependencies: HashMap<String, String>,

    /// What this plugin contributes to the server.
    #[serde(default)]
    pub contributions: PluginContributions,

    /// Permissions the plugin requires.
    #[serde(default)]
    pub permissions: PluginPermissions,

    /// WASM-specific configuration.
    #[serde(default)]
    pub wasm: Option<WasmConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmConfig {
    /// Path to the .wasm file (relative to plugin root).
    pub module: String,
    /// Max memory in MB.
    #[serde(default = "default_max_memory")]
    pub max_memory_mb: u32,
    /// Max execution time per call in ms.
    #[serde(default = "default_max_exec_time")]
    pub max_execution_time_ms: u64,
}

fn default_plugin_type() -> String {
    "builtin".to_string()
}

fn default_activation() -> ActivationEvent {
    ActivationEvent::OnStartup
}

fn default_max_memory() -> u32 {
    64
}

fn default_max_exec_time() -> u64 {
    5000
}

impl PluginManifest {
    /// Parse a plugin manifest from TOML string.
    pub fn from_toml(content: &str) -> Result<Self> {
        let manifest: Self =
            toml::from_str(content).map_err(|e| Error::PluginManifestInvalid(e.to_string()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Parse a plugin manifest from a file.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref()).map_err(|e| {
            Error::PluginManifestInvalid(format!("failed to read {}: {e}", path.as_ref().display()))
        })?;
        Self::from_toml(&content)
    }

    /// Get plugin ID as a typed PluginId.
    pub fn plugin_id(&self) -> PluginId {
        PluginId::from_str(&self.id)
    }

    /// Validate the manifest fields.
    pub fn validate(&self) -> Result<()> {
        if self.id.is_empty() {
            return Err(Error::PluginManifestInvalid("id is required".to_string()));
        }
        if !self.id.contains('.') {
            return Err(Error::PluginManifestInvalid(
                "id must be reverse-domain format (e.g., io.draox.myplugin)".to_string(),
            ));
        }
        if self.name.is_empty() {
            return Err(Error::PluginManifestInvalid("name is required".to_string()));
        }
        if self.version.is_empty() {
            return Err(Error::PluginManifestInvalid(
                "version is required".to_string(),
            ));
        }
        if self.author.is_empty() {
            return Err(Error::PluginManifestInvalid(
                "author is required".to_string(),
            ));
        }
        if self.plugin_type != "builtin" && self.plugin_type != "wasm" {
            return Err(Error::PluginManifestInvalid(format!(
                "invalid type '{}', must be 'builtin' or 'wasm'",
                self.plugin_type
            )));
        }
        if self.plugin_type == "wasm" && self.wasm.is_none() {
            return Err(Error::PluginManifestInvalid(
                "wasm section required when type = 'wasm'".to_string(),
            ));
        }
        Ok(())
    }
}

// ────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_MANIFEST: &str = r#"
id = "io.draox.test-plugin"
name = "Test Plugin"
version = "1.0.0"
author = "Draox Team"
description = "A test plugin"
type = "builtin"

[contributions]
commands = [
    { name = "test.hello", description = "Say hello" }
]
routes = [
    { method = "GET", path = "/test/hello", description = "Hello endpoint" }
]
events = ["test.event.fired"]

[permissions]
storage = true
events = true
"#;

    #[test]
    fn test_parse_valid_manifest() {
        let manifest = PluginManifest::from_toml(VALID_MANIFEST).unwrap();
        assert_eq!(manifest.id, "io.draox.test-plugin");
        assert_eq!(manifest.name, "Test Plugin");
        assert_eq!(manifest.plugin_type, "builtin");
        assert!(manifest.permissions.storage);
        assert!(manifest.permissions.events);
        assert!(!manifest.permissions.cache);
        assert_eq!(manifest.contributions.commands.len(), 1);
        assert_eq!(manifest.contributions.routes.len(), 1);
    }

    #[test]
    fn test_missing_id_fails() {
        let toml = r#"
name = "Bad Plugin"
version = "1.0.0"
author = "Test"
"#;
        // TOML parsing will default id to empty → validate catches it
        let result = PluginManifest::from_toml(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_id_format() {
        let toml = r#"
id = "noreversedomain"
name = "Bad Plugin"
version = "1.0.0"
author = "Test"
"#;
        let result = PluginManifest::from_toml(toml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("reverse-domain"));
    }

    #[test]
    fn test_wasm_manifest() {
        let toml = r#"
id = "io.draox.wasm-test"
name = "WASM Plugin"
version = "0.1.0"
author = "Test"
type = "wasm"

[wasm]
module = "plugin.wasm"
max_memory_mb = 128
max_execution_time_ms = 10000
"#;
        let manifest = PluginManifest::from_toml(toml).unwrap();
        assert_eq!(manifest.plugin_type, "wasm");
        let wasm = manifest.wasm.unwrap();
        assert_eq!(wasm.module, "plugin.wasm");
        assert_eq!(wasm.max_memory_mb, 128);
    }

    #[test]
    fn test_wasm_type_without_wasm_section_fails() {
        let toml = r#"
id = "io.draox.bad-wasm"
name = "Bad WASM"
version = "1.0.0"
author = "Test"
type = "wasm"
"#;
        let result = PluginManifest::from_toml(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_plugin_id_typed() {
        let manifest = PluginManifest::from_toml(VALID_MANIFEST).unwrap();
        let id = manifest.plugin_id();
        assert_eq!(id.as_str(), "io.draox.test-plugin");
    }
}
