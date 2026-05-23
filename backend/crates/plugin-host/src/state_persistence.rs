use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io;
use std::path::PathBuf;

/// Snapshot of a plugin's runtime state, safe to serialize to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedPluginState {
    /// Reverse-domain plugin identifier (e.g. "io.draox.clans").
    pub plugin_id: String,
    /// Lifecycle state name: "ActiveEnabled", "ActiveDisabled", or "Installed".
    pub state: String,
    /// Plugin-specific configuration key/value pairs.
    pub config: HashMap<String, serde_json::Value>,
    /// Wall-clock time this record was last written.
    pub saved_at: DateTime<Utc>,
}

/// Persists plugin states to a single JSON file on disk.
///
/// All operations are synchronous (`std::io`), intended to be
/// called at startup/shutdown — not on the hot path.
pub struct StatePersistence {
    /// Path to the JSON persistence file (e.g. `data/plugin_states.json`).
    path: PathBuf,
}

impl StatePersistence {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Atomically overwrite the persistence file with `states`.
    pub fn save_all(&self, states: &[PersistedPluginState]) -> io::Result<()> {
        // Ensure the parent directory exists.
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(states)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Write to a temp file then rename for atomicity.
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    /// Load all persisted states. Returns an empty vec if the file does not exist.
    pub fn load_all(&self) -> io::Result<Vec<PersistedPluginState>> {
        match std::fs::read_to_string(&self.path) {
            Ok(json) => {
                let states: Vec<PersistedPluginState> = serde_json::from_str(&json)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                Ok(states)
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(e),
        }
    }

    /// Upsert a single plugin's state into the persistence file.
    pub fn save_one(&self, state: &PersistedPluginState) -> io::Result<()> {
        let mut all = self.load_all()?;
        if let Some(existing) = all.iter_mut().find(|s| s.plugin_id == state.plugin_id) {
            *existing = state.clone();
        } else {
            all.push(state.clone());
        }
        self.save_all(&all)
    }

    /// Remove the persisted state for a plugin. No-op if not found.
    pub fn remove(&self, plugin_id: &str) -> io::Result<()> {
        let mut all = self.load_all()?;
        let before = all.len();
        all.retain(|s| s.plugin_id != plugin_id);
        if all.len() != before {
            self.save_all(&all)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_state(plugin_id: &str, state: &str) -> PersistedPluginState {
        PersistedPluginState {
            plugin_id: plugin_id.to_string(),
            state: state.to_string(),
            config: HashMap::new(),
            saved_at: Utc::now(),
        }
    }

    fn persistence_at(dir: &std::path::Path) -> StatePersistence {
        StatePersistence::new(dir.join("plugin_states.json"))
    }

    #[test]
    fn test_save_and_load_all() {
        let dir = TempDir::new().unwrap();
        let sp = persistence_at(dir.path());

        let states = vec![
            make_state("io.draox.clans", "ActiveEnabled"),
            make_state("io.draox.messaging", "Installed"),
        ];

        sp.save_all(&states).unwrap();

        let loaded = sp.load_all().unwrap();
        assert_eq!(loaded.len(), 2);

        let ids: Vec<&str> = loaded.iter().map(|s| s.plugin_id.as_str()).collect();
        assert!(ids.contains(&"io.draox.clans"));
        assert!(ids.contains(&"io.draox.messaging"));
    }

    #[test]
    fn test_load_returns_empty_when_file_missing() {
        let dir = TempDir::new().unwrap();
        let sp = persistence_at(dir.path());

        // File doesn't exist yet — should return empty vec, not an error.
        let loaded = sp.load_all().unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_save_one_inserts_new() {
        let dir = TempDir::new().unwrap();
        let sp = persistence_at(dir.path());

        sp.save_one(&make_state("io.draox.a", "Installed")).unwrap();
        sp.save_one(&make_state("io.draox.b", "ActiveEnabled")).unwrap();

        let loaded = sp.load_all().unwrap();
        assert_eq!(loaded.len(), 2);
    }

    #[test]
    fn test_save_one_updates_existing() {
        let dir = TempDir::new().unwrap();
        let sp = persistence_at(dir.path());

        sp.save_one(&make_state("io.draox.a", "Installed")).unwrap();

        // Update state to ActiveEnabled.
        sp.save_one(&make_state("io.draox.a", "ActiveEnabled")).unwrap();

        let loaded = sp.load_all().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].state, "ActiveEnabled");
    }

    #[test]
    fn test_remove_plugin_state() {
        let dir = TempDir::new().unwrap();
        let sp = persistence_at(dir.path());

        sp.save_all(&[
            make_state("io.draox.a", "ActiveEnabled"),
            make_state("io.draox.b", "Installed"),
        ])
        .unwrap();

        sp.remove("io.draox.a").unwrap();

        let loaded = sp.load_all().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].plugin_id, "io.draox.b");
    }

    #[test]
    fn test_config_roundtrip() {
        let dir = TempDir::new().unwrap();
        let sp = persistence_at(dir.path());

        let mut config = HashMap::new();
        config.insert("max_members".to_string(), serde_json::json!(500));
        config.insert("clan_prefix".to_string(), serde_json::json!("KY_"));

        let state = PersistedPluginState {
            plugin_id: "io.draox.clans".to_string(),
            state: "ActiveEnabled".to_string(),
            config,
            saved_at: Utc::now(),
        };

        sp.save_all(&[state]).unwrap();
        let loaded = sp.load_all().unwrap();

        let loaded_config = &loaded[0].config;
        assert_eq!(loaded_config["max_members"], serde_json::json!(500));
        assert_eq!(loaded_config["clan_prefix"], serde_json::json!("KY_"));
    }
}
