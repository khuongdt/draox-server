use dashmap::DashMap;
use std::sync::Arc;
use crate::provider::StorageError;

/// Quota registry tracking per-owner byte usage.
pub struct QuotaManager {
    usage_bytes: Arc<DashMap<String, u64>>,
    limits_bytes: Arc<DashMap<String, u64>>,
}

impl QuotaManager {
    pub fn new() -> Self {
        Self {
            usage_bytes: Arc::new(DashMap::new()),
            limits_bytes: Arc::new(DashMap::new()),
        }
    }

    pub fn set_limit(&self, owner: &str, limit_bytes: u64) {
        self.limits_bytes.insert(owner.to_string(), limit_bytes);
    }

    pub fn get_limit(&self, owner: &str) -> Option<u64> {
        self.limits_bytes.get(owner).map(|v| *v)
    }

    pub fn get_usage(&self, owner: &str) -> u64 {
        self.usage_bytes.get(owner).map(|v| *v).unwrap_or(0)
    }

    /// Check if an upload of `size_bytes` would exceed the owner's quota.
    pub fn check(&self, owner: &str, size_bytes: u64) -> Result<(), StorageError> {
        if let Some(limit) = self.get_limit(owner) {
            let current = self.get_usage(owner);
            if current + size_bytes > limit {
                return Err(StorageError::QuotaExceeded);
            }
        }
        Ok(())
    }

    pub fn add_usage(&self, owner: &str, bytes: u64) {
        let mut entry = self.usage_bytes.entry(owner.to_string()).or_insert(0);
        *entry += bytes;
    }

    pub fn remove_usage(&self, owner: &str, bytes: u64) {
        let mut entry = self.usage_bytes.entry(owner.to_string()).or_insert(0);
        *entry = entry.saturating_sub(bytes);
    }

    pub fn available_bytes(&self, owner: &str) -> Option<u64> {
        let limit = self.get_limit(owner)?;
        let used = self.get_usage(owner);
        Some(limit.saturating_sub(used))
    }
}

impl Default for QuotaManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quota_allow_within_limit() {
        let q = QuotaManager::new();
        q.set_limit("user1", 100_000);
        assert!(q.check("user1", 50_000).is_ok());
    }

    #[test]
    fn test_quota_reject_over_limit() {
        let q = QuotaManager::new();
        q.set_limit("user1", 100_000);
        q.add_usage("user1", 90_000);
        assert!(q.check("user1", 20_000).is_err());
    }

    #[test]
    fn test_quota_no_limit_set() {
        let q = QuotaManager::new();
        // Without a limit set, any size is allowed
        assert!(q.check("unlimited_user", 999_999_999).is_ok());
    }
}
