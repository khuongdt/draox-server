use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use server_core::ClientId;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPreferences {
    pub client_id: ClientId,
    pub enabled: bool,
    pub muted_topics: Vec<String>,
    pub quiet_hours_start: Option<u8>,
    pub quiet_hours_end: Option<u8>,
    pub badge_count: u32,
}

impl NotificationPreferences {
    pub fn new(client_id: ClientId) -> Self {
        Self {
            client_id,
            enabled: true,
            muted_topics: Vec::new(),
            quiet_hours_start: None,
            quiet_hours_end: None,
            badge_count: 0,
        }
    }

    pub fn is_topic_muted(&self, topic: &str) -> bool {
        self.muted_topics.iter().any(|t| t == topic)
    }

    pub fn is_quiet_hours(&self, current_hour: u8) -> bool {
        match (self.quiet_hours_start, self.quiet_hours_end) {
            (Some(start), Some(end)) => {
                if start <= end {
                    current_hour >= start && current_hour < end
                } else {
                    current_hour >= start || current_hour < end
                }
            }
            _ => false,
        }
    }

    pub fn should_notify(&self, topic: Option<&str>) -> bool {
        if !self.enabled {
            return false;
        }
        if let Some(t) = topic {
            if self.is_topic_muted(t) {
                return false;
            }
        }
        true
    }
}

pub struct PreferencesStore {
    data: Arc<DashMap<String, NotificationPreferences>>,
}

impl PreferencesStore {
    pub fn new() -> Self {
        Self { data: Arc::new(DashMap::new()) }
    }

    pub fn get_or_default(&self, client_id: &ClientId) -> NotificationPreferences {
        self.data
            .get(client_id.as_str())
            .map(|p| p.clone())
            .unwrap_or_else(|| NotificationPreferences::new(client_id.clone()))
    }

    pub fn update(&self, prefs: NotificationPreferences) {
        self.data.insert(prefs.client_id.as_str().to_string(), prefs);
    }

    pub fn increment_badge(&self, client_id: &ClientId) {
        let mut prefs = self.get_or_default(client_id);
        prefs.badge_count += 1;
        self.update(prefs);
    }

    pub fn reset_badge(&self, client_id: &ClientId) {
        if let Some(mut entry) = self.data.get_mut(client_id.as_str()) {
            entry.badge_count = 0;
        }
    }
}

impl Default for PreferencesStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quiet_hours_overnight() {
        let mut prefs = NotificationPreferences::new(ClientId::from_str("x"));
        prefs.quiet_hours_start = Some(22);
        prefs.quiet_hours_end = Some(7);
        assert!(prefs.is_quiet_hours(23));
        assert!(prefs.is_quiet_hours(0));
        assert!(prefs.is_quiet_hours(6));
        assert!(!prefs.is_quiet_hours(8));
    }

    #[test]
    fn test_muted_topic() {
        let mut prefs = NotificationPreferences::new(ClientId::from_str("y"));
        prefs.muted_topics.push("clan.chat".to_string());
        assert!(!prefs.should_notify(Some("clan.chat")));
        assert!(prefs.should_notify(Some("dm")));
    }
}
