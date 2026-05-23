use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::clan::{ClanId, ClanRole};
use crate::divisions::DivisionId;

/// Unique channel identifier.
pub type ClanChannelId = String;

/// Channel types within a clan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClanChannelType {
    /// General-purpose channel visible to all members.
    General,
    /// Officers-only channel.
    Officers,
    /// Channel scoped to a specific division.
    Division(DivisionId),
    /// User-defined channel.
    Custom,
}

/// A communication channel within a clan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClanChannel {
    pub id: ClanChannelId,
    pub clan_id: ClanId,
    pub name: String,
    pub channel_type: ClanChannelType,
    /// Minimum role required to read/write in this channel.
    pub min_role: ClanRole,
    pub created_at: DateTime<Utc>,
}

/// Manages clan channels.
pub struct ClanChannelManager {
    channels: DashMap<ClanChannelId, ClanChannel>,
    /// clan_id -> list of channel IDs in that clan.
    clan_channels: DashMap<ClanId, Vec<ClanChannelId>>,
}

impl ClanChannelManager {
    /// Create a new, empty channel manager.
    pub fn new() -> Self {
        Self {
            channels: DashMap::new(),
            clan_channels: DashMap::new(),
        }
    }

    /// Auto-create the default `general` and `officers` channels for a new clan.
    ///
    /// Returns the two newly created channels.
    pub fn create_defaults(&self, clan_id: &ClanId) -> Vec<ClanChannel> {
        let general = self.create(
            clan_id,
            "general",
            ClanChannelType::General,
            ClanRole::Recruit,
        );
        let officers = self.create(
            clan_id,
            "officers",
            ClanChannelType::Officers,
            ClanRole::Officer,
        );
        vec![general, officers]
    }

    /// Create a new channel within a clan.
    pub fn create(
        &self,
        clan_id: &ClanId,
        name: &str,
        channel_type: ClanChannelType,
        min_role: ClanRole,
    ) -> ClanChannel {
        let id = format!("ch_{}", uuid::Uuid::new_v4().as_simple());
        let channel = ClanChannel {
            id: id.clone(),
            clan_id: clan_id.clone(),
            name: name.to_string(),
            channel_type,
            min_role,
            created_at: Utc::now(),
        };

        self.channels.insert(id.clone(), channel.clone());
        self.clan_channels
            .entry(clan_id.clone())
            .or_default()
            .push(id.clone());

        info!(channel_id = %id, clan_id = %clan_id, name = %name, "clan channel created");
        channel
    }

    /// Get a channel by ID (cloned).
    pub fn get(&self, id: &ClanChannelId) -> Option<ClanChannel> {
        self.channels.get(id).map(|r| r.value().clone())
    }

    /// Delete a channel by ID.
    ///
    /// Also removes it from the parent clan's index.
    /// Returns `true` if the channel existed and was removed.
    pub fn delete(&self, id: &ClanChannelId) -> bool {
        let removed = self.channels.remove(id);
        if let Some((_, channel)) = &removed {
            if let Some(mut ids) = self.clan_channels.get_mut(&channel.clan_id) {
                ids.retain(|cid| cid != id);
            }
            debug!(channel_id = %id, clan_id = %channel.clan_id, "clan channel deleted");
        }
        removed.is_some()
    }

    /// List all channels for a clan (cloned).
    pub fn list_by_clan(&self, clan_id: &ClanId) -> Vec<ClanChannel> {
        let ids = match self.clan_channels.get(clan_id) {
            Some(ids) => ids.value().clone(),
            None => return Vec::new(),
        };
        ids.iter()
            .filter_map(|cid| self.channels.get(cid).map(|r| r.value().clone()))
            .collect()
    }

    /// Check whether a given role can access a channel.
    ///
    /// Returns `false` if the channel does not exist.
    pub fn can_access(&self, id: &ClanChannelId, role: &ClanRole) -> bool {
        match self.channels.get(id) {
            Some(ch) => role.rank() >= ch.min_role.rank(),
            None => false,
        }
    }

    /// List all channels in a clan that a given role can access.
    pub fn list_accessible(&self, clan_id: &ClanId, role: &ClanRole) -> Vec<ClanChannel> {
        self.list_by_clan(clan_id)
            .into_iter()
            .filter(|ch| role.rank() >= ch.min_role.rank())
            .collect()
    }

    /// Total number of channels across all clans.
    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }
}

impl Default for ClanChannelManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_get_channel() {
        let mgr = ClanChannelManager::new();
        let clan_id: ClanId = "clan_alpha".to_string();

        let ch = mgr.create(
            &clan_id,
            "announcements",
            ClanChannelType::Custom,
            ClanRole::Recruit,
        );
        assert!(ch.id.starts_with("ch_"));
        assert_eq!(ch.clan_id, clan_id);
        assert_eq!(ch.name, "announcements");
        assert_eq!(ch.channel_type, ClanChannelType::Custom);
        assert_eq!(ch.min_role, ClanRole::Recruit);

        let fetched = mgr.get(&ch.id).unwrap();
        assert_eq!(fetched.id, ch.id);
    }

    #[test]
    fn test_create_defaults() {
        let mgr = ClanChannelManager::new();
        let clan_id: ClanId = "clan_defaults".to_string();

        let defaults = mgr.create_defaults(&clan_id);
        assert_eq!(defaults.len(), 2);

        let names: Vec<&str> = defaults.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"general"));
        assert!(names.contains(&"officers"));

        let general = defaults.iter().find(|c| c.name == "general").unwrap();
        assert_eq!(general.channel_type, ClanChannelType::General);
        assert_eq!(general.min_role, ClanRole::Recruit);

        let officers = defaults.iter().find(|c| c.name == "officers").unwrap();
        assert_eq!(officers.channel_type, ClanChannelType::Officers);
        assert_eq!(officers.min_role, ClanRole::Officer);
    }

    #[test]
    fn test_delete_channel() {
        let mgr = ClanChannelManager::new();
        let clan_id: ClanId = "clan_del".to_string();
        let ch = mgr.create(&clan_id, "temp", ClanChannelType::Custom, ClanRole::Recruit);

        assert_eq!(mgr.channel_count(), 1);
        assert!(mgr.delete(&ch.id));
        assert_eq!(mgr.channel_count(), 0);
        assert!(mgr.get(&ch.id).is_none());

        // Clan index cleaned up
        assert!(mgr.list_by_clan(&clan_id).is_empty());

        // Second delete returns false
        assert!(!mgr.delete(&ch.id));
    }

    #[test]
    fn test_list_by_clan() {
        let mgr = ClanChannelManager::new();
        let clan_a: ClanId = "clan_a".to_string();
        let clan_b: ClanId = "clan_b".to_string();

        mgr.create(&clan_a, "general", ClanChannelType::General, ClanRole::Recruit);
        mgr.create(&clan_a, "officers", ClanChannelType::Officers, ClanRole::Officer);
        mgr.create(&clan_b, "general", ClanChannelType::General, ClanRole::Recruit);

        let list_a = mgr.list_by_clan(&clan_a);
        assert_eq!(list_a.len(), 2);

        let list_b = mgr.list_by_clan(&clan_b);
        assert_eq!(list_b.len(), 1);

        // Non-existent clan
        assert!(mgr.list_by_clan(&"clan_nope".to_string()).is_empty());
    }

    #[test]
    fn test_can_access() {
        let mgr = ClanChannelManager::new();
        let clan_id: ClanId = "clan_access".to_string();

        let general = mgr.create(
            &clan_id,
            "general",
            ClanChannelType::General,
            ClanRole::Recruit,
        );
        let officer_ch = mgr.create(
            &clan_id,
            "officers",
            ClanChannelType::Officers,
            ClanRole::Officer,
        );

        // All roles can access general
        assert!(mgr.can_access(&general.id, &ClanRole::Recruit));
        assert!(mgr.can_access(&general.id, &ClanRole::Member));
        assert!(mgr.can_access(&general.id, &ClanRole::Officer));
        assert!(mgr.can_access(&general.id, &ClanRole::Owner));

        // Only Officer+ can access officers channel
        assert!(!mgr.can_access(&officer_ch.id, &ClanRole::Recruit));
        assert!(!mgr.can_access(&officer_ch.id, &ClanRole::Member));
        assert!(mgr.can_access(&officer_ch.id, &ClanRole::Officer));
        assert!(mgr.can_access(&officer_ch.id, &ClanRole::Owner));

        // Non-existent channel
        assert!(!mgr.can_access(&"ch_nope".to_string(), &ClanRole::Owner));
    }

    #[test]
    fn test_list_accessible() {
        let mgr = ClanChannelManager::new();
        let clan_id: ClanId = "clan_visible".to_string();

        mgr.create(&clan_id, "general", ClanChannelType::General, ClanRole::Recruit);
        mgr.create(&clan_id, "members", ClanChannelType::Custom, ClanRole::Member);
        mgr.create(&clan_id, "officers", ClanChannelType::Officers, ClanRole::Officer);
        mgr.create(&clan_id, "owner-only", ClanChannelType::Custom, ClanRole::Owner);

        // Recruit can only see general
        let recruit_channels = mgr.list_accessible(&clan_id, &ClanRole::Recruit);
        assert_eq!(recruit_channels.len(), 1);

        // Member sees general + members
        let member_channels = mgr.list_accessible(&clan_id, &ClanRole::Member);
        assert_eq!(member_channels.len(), 2);

        // Officer sees general + members + officers
        let officer_channels = mgr.list_accessible(&clan_id, &ClanRole::Officer);
        assert_eq!(officer_channels.len(), 3);

        // Owner sees all
        let owner_channels = mgr.list_accessible(&clan_id, &ClanRole::Owner);
        assert_eq!(owner_channels.len(), 4);
    }

    #[test]
    fn test_division_channel_type() {
        let mgr = ClanChannelManager::new();
        let clan_id: ClanId = "clan_div".to_string();
        let div_id = "div_alpha1234".to_string();

        let ch = mgr.create(
            &clan_id,
            "div-alpha",
            ClanChannelType::Division(div_id.clone()),
            ClanRole::Member,
        );

        assert_eq!(ch.channel_type, ClanChannelType::Division(div_id));
    }
}
