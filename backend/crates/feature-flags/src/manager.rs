use std::sync::Arc;
use dashmap::DashMap;
use thiserror::Error;
use tracing::{debug, info};
use crate::evaluator::EvalContext;
use crate::flag::{FeatureFlag, FlagValue};
use crate::evaluator::FlagEvaluator;

#[derive(Debug, Error)]
pub enum FlagError {
    #[error("flag not found: {0}")]
    NotFound(String),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Central feature-flag manager. Thread-safe, hot-reload capable.
///
/// Load initial flags with [`FeatureFlagManager::load`] (e.g. from `config.toml`
/// or a remote service). Flags can be toggled or updated at runtime without restart.
#[derive(Clone, Default)]
pub struct FeatureFlagManager {
    flags: Arc<DashMap<String, FeatureFlag>>,
}

impl FeatureFlagManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Bulk-load flags, replacing any existing definition for that key.
    pub fn load(&self, flags: Vec<FeatureFlag>) {
        for flag in flags {
            info!(key = %flag.key, "feature flag loaded");
            self.flags.insert(flag.key.clone(), flag);
        }
    }

    /// Register or overwrite a single flag.
    pub fn upsert(&self, flag: FeatureFlag) {
        debug!(key = %flag.key, "feature flag upserted");
        self.flags.insert(flag.key.clone(), flag);
    }

    /// Evaluate flag `key` against `ctx`. Returns `default` if flag is missing.
    pub fn evaluate(&self, key: &str, ctx: &EvalContext) -> Option<FlagValue> {
        let flag = self.flags.get(key)?;
        Some(FlagEvaluator::evaluate(&flag, ctx))
    }

    /// Convenience: evaluate and return bool (defaults to `false`).
    pub fn is_enabled(&self, key: &str, ctx: &EvalContext) -> bool {
        self.evaluate(key, ctx)
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    /// Toggle a flag on/off at runtime (hot-reload).
    pub fn set_enabled(&self, key: &str, enabled: bool) -> Result<(), FlagError> {
        let mut flag = self
            .flags
            .get_mut(key)
            .ok_or_else(|| FlagError::NotFound(key.to_string()))?;
        flag.enabled = enabled;
        info!(key, enabled, "feature flag toggled");
        Ok(())
    }

    pub fn list_keys(&self) -> Vec<String> {
        let mut keys: Vec<_> = self.flags.iter().map(|e| e.key().clone()).collect();
        keys.sort();
        keys
    }

    pub fn get(&self, key: &str) -> Option<FeatureFlag> {
        self.flags.get(key).map(|e| e.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flag::FeatureFlag;

    #[test]
    fn test_is_enabled_missing_flag() {
        let mgr = FeatureFlagManager::new();
        assert!(!mgr.is_enabled("ghost_flag", &EvalContext::new()));
    }

    #[test]
    fn test_load_and_evaluate() {
        let mgr = FeatureFlagManager::new();
        mgr.load(vec![FeatureFlag::new_bool("dark_mode", true)]);
        assert!(mgr.is_enabled("dark_mode", &EvalContext::new()));
    }

    #[test]
    fn test_toggle() {
        let mgr = FeatureFlagManager::new();
        mgr.upsert(FeatureFlag::new_bool("beta", true));
        assert!(mgr.is_enabled("beta", &EvalContext::new()));
        mgr.set_enabled("beta", false).unwrap();
        assert!(!mgr.is_enabled("beta", &EvalContext::new()));
    }
}
