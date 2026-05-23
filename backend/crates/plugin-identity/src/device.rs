use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub platform: Option<String>,
    pub app_version: Option<String>,
    pub registered_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
}

impl DeviceInfo {
    pub fn new(device_id: String) -> Self {
        let now = Utc::now();
        Self {
            device_id,
            user_agent: None,
            ip_address: None,
            platform: None,
            app_version: None,
            registered_at: now,
            last_seen_at: now,
        }
    }
}

/// Generate a device fingerprint from request attributes.
pub fn fingerprint(
    user_agent: Option<&str>,
    ip: Option<&str>,
    extra: &HashMap<String, String>,
) -> String {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;

    let mut hasher = DefaultHasher::new();
    user_agent.hash(&mut hasher);
    ip.hash(&mut hasher);
    let mut keys: Vec<_> = extra.iter().collect();
    keys.sort_by_key(|(k, _)| k.as_str());
    for (k, v) in keys {
        k.hash(&mut hasher);
        v.hash(&mut hasher);
    }
    format!("dev_{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fingerprint_deterministic() {
        let extra = HashMap::new();
        let fp1 = fingerprint(Some("Mozilla/5.0"), Some("192.168.1.1"), &extra);
        let fp2 = fingerprint(Some("Mozilla/5.0"), Some("192.168.1.1"), &extra);
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_fingerprint_differs_on_change() {
        let extra = HashMap::new();
        let fp1 = fingerprint(Some("Mozilla/5.0"), Some("192.168.1.1"), &extra);
        let fp2 = fingerprint(Some("Mozilla/5.0"), Some("10.0.0.1"), &extra);
        assert_ne!(fp1, fp2);
    }
}
