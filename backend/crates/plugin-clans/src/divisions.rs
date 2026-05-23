use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::clan::ClanId;

/// Unique division identifier.
pub type DivisionId = String;

/// A sub-group within a clan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Division {
    pub id: DivisionId,
    pub clan_id: ClanId,
    pub name: String,
    pub description: Option<String>,
    /// client_id of the division leader (if any).
    pub leader: Option<String>,
    /// client_ids of division members.
    pub members: Vec<String>,
    pub created_at: DateTime<Utc>,
}

/// Manages clan divisions.
pub struct DivisionManager {
    divisions: DashMap<DivisionId, Division>,
    /// clan_id -> list of division IDs in that clan.
    clan_divisions: DashMap<ClanId, Vec<DivisionId>>,
}

impl DivisionManager {
    /// Create a new, empty division manager.
    pub fn new() -> Self {
        Self {
            divisions: DashMap::new(),
            clan_divisions: DashMap::new(),
        }
    }

    /// Create a new division within a clan.
    ///
    /// Returns the newly created [`Division`].
    pub fn create(
        &self,
        clan_id: &ClanId,
        name: &str,
        description: Option<&str>,
    ) -> Division {
        let id = format!("div_{}", uuid::Uuid::new_v4().as_simple());
        let division = Division {
            id: id.clone(),
            clan_id: clan_id.clone(),
            name: name.to_string(),
            description: description.map(|d| d.to_string()),
            leader: None,
            members: Vec::new(),
            created_at: Utc::now(),
        };

        self.divisions.insert(id.clone(), division.clone());
        self.clan_divisions
            .entry(clan_id.clone())
            .or_default()
            .push(id.clone());

        info!(division_id = %id, clan_id = %clan_id, name = %name, "division created");
        division
    }

    /// Get a division by ID (cloned).
    pub fn get(&self, id: &DivisionId) -> Option<Division> {
        self.divisions.get(id).map(|r| r.value().clone())
    }

    /// Update a division's name and/or description.
    ///
    /// Returns `true` if the division was found and updated; `false` if not found.
    pub fn update(
        &self,
        id: &DivisionId,
        name: Option<&str>,
        description: Option<&str>,
    ) -> bool {
        let mut entry = match self.divisions.get_mut(id) {
            Some(e) => e,
            None => return false,
        };

        if let Some(n) = name {
            entry.name = n.to_string();
        }
        if let Some(d) = description {
            entry.description = Some(d.to_string());
        }

        debug!(division_id = %id, "division updated");
        true
    }

    /// Delete a division by ID.
    ///
    /// Also removes its entry from the parent clan's index.
    /// Returns `true` if the division existed and was removed.
    pub fn delete(&self, id: &DivisionId) -> bool {
        let removed = self.divisions.remove(id);
        if let Some((_, division)) = &removed {
            if let Some(mut ids) = self.clan_divisions.get_mut(&division.clan_id) {
                ids.retain(|did| did != id);
            }
            debug!(division_id = %id, clan_id = %division.clan_id, "division deleted");
        }
        removed.is_some()
    }

    /// List all divisions belonging to a clan.
    pub fn list_by_clan(&self, clan_id: &ClanId) -> Vec<Division> {
        let ids = match self.clan_divisions.get(clan_id) {
            Some(ids) => ids.value().clone(),
            None => return Vec::new(),
        };
        ids.iter()
            .filter_map(|did| self.divisions.get(did).map(|r| r.value().clone()))
            .collect()
    }

    /// Set the leader of a division.
    ///
    /// Returns `true` if the division was found.
    pub fn set_leader(&self, id: &DivisionId, leader: &str) -> bool {
        match self.divisions.get_mut(id) {
            Some(mut entry) => {
                entry.leader = Some(leader.to_string());
                debug!(division_id = %id, leader = %leader, "division leader set");
                true
            }
            None => false,
        }
    }

    /// Add a member to a division.
    ///
    /// Does nothing if the member is already in the division.
    /// Returns `true` if the division was found.
    pub fn add_member(&self, id: &DivisionId, member: &str) -> bool {
        match self.divisions.get_mut(id) {
            Some(mut entry) => {
                if !entry.members.iter().any(|m| m == member) {
                    entry.members.push(member.to_string());
                    debug!(division_id = %id, member = %member, "member added to division");
                }
                true
            }
            None => false,
        }
    }

    /// Remove a member from a division.
    ///
    /// Also clears the leader field if the removed member was the leader.
    /// Returns `true` if the division was found.
    pub fn remove_member(&self, id: &DivisionId, member: &str) -> bool {
        match self.divisions.get_mut(id) {
            Some(mut entry) => {
                entry.members.retain(|m| m != member);
                // Clear leader if they were the one removed
                if entry.leader.as_deref() == Some(member) {
                    entry.leader = None;
                }
                debug!(division_id = %id, member = %member, "member removed from division");
                true
            }
            None => false,
        }
    }

    /// Total number of divisions across all clans.
    pub fn division_count(&self) -> usize {
        self.divisions.len()
    }
}

impl Default for DivisionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_get_division() {
        let mgr = DivisionManager::new();
        let clan_id: ClanId = "clan_alpha".to_string();

        let div = mgr.create(&clan_id, "Vanguard", Some("Front-line fighters"));
        assert!(div.id.starts_with("div_"));
        assert_eq!(div.clan_id, clan_id);
        assert_eq!(div.name, "Vanguard");
        assert_eq!(div.description.as_deref(), Some("Front-line fighters"));
        assert!(div.leader.is_none());
        assert!(div.members.is_empty());

        let fetched = mgr.get(&div.id).unwrap();
        assert_eq!(fetched.id, div.id);
        assert_eq!(fetched.name, "Vanguard");
    }

    #[test]
    fn test_update_division() {
        let mgr = DivisionManager::new();
        let clan_id: ClanId = "clan_beta".to_string();
        let div = mgr.create(&clan_id, "Old Name", None);

        // Update name only
        assert!(mgr.update(&div.id, Some("New Name"), None));
        let updated = mgr.get(&div.id).unwrap();
        assert_eq!(updated.name, "New Name");
        assert!(updated.description.is_none());

        // Update description only
        assert!(mgr.update(&div.id, None, Some("Now has a description")));
        let updated2 = mgr.get(&div.id).unwrap();
        assert_eq!(updated2.name, "New Name");
        assert_eq!(updated2.description.as_deref(), Some("Now has a description"));

        // Non-existent division returns false
        assert!(!mgr.update(&"div_nope".to_string(), Some("X"), None));
    }

    #[test]
    fn test_delete_division() {
        let mgr = DivisionManager::new();
        let clan_id: ClanId = "clan_gamma".to_string();
        let div = mgr.create(&clan_id, "Scouts", None);

        assert_eq!(mgr.division_count(), 1);
        assert!(mgr.delete(&div.id));
        assert_eq!(mgr.division_count(), 0);
        assert!(mgr.get(&div.id).is_none());

        // Clan index should be empty too
        assert!(mgr.list_by_clan(&clan_id).is_empty());

        // Deleting again returns false
        assert!(!mgr.delete(&div.id));
    }

    #[test]
    fn test_list_by_clan() {
        let mgr = DivisionManager::new();
        let clan_a: ClanId = "clan_a".to_string();
        let clan_b: ClanId = "clan_b".to_string();

        mgr.create(&clan_a, "Alpha-1", None);
        mgr.create(&clan_a, "Alpha-2", Some("Second division"));
        mgr.create(&clan_b, "Beta-1", None);

        let divs_a = mgr.list_by_clan(&clan_a);
        assert_eq!(divs_a.len(), 2);
        assert!(divs_a.iter().all(|d| d.clan_id == clan_a));

        let divs_b = mgr.list_by_clan(&clan_b);
        assert_eq!(divs_b.len(), 1);
        assert_eq!(divs_b[0].name, "Beta-1");

        // Non-existent clan returns empty
        assert!(mgr.list_by_clan(&"clan_nope".to_string()).is_empty());
    }

    #[test]
    fn test_set_leader() {
        let mgr = DivisionManager::new();
        let clan_id: ClanId = "clan_lead".to_string();
        let div = mgr.create(&clan_id, "Elites", None);

        assert!(mgr.set_leader(&div.id, "cli_commander"));
        let updated = mgr.get(&div.id).unwrap();
        assert_eq!(updated.leader.as_deref(), Some("cli_commander"));

        // Non-existent division returns false
        assert!(!mgr.set_leader(&"div_nope".to_string(), "cli_x"));
    }

    #[test]
    fn test_add_and_remove_member() {
        let mgr = DivisionManager::new();
        let clan_id: ClanId = "clan_members".to_string();
        let div = mgr.create(&clan_id, "Recon", None);

        assert!(mgr.add_member(&div.id, "cli_alice"));
        assert!(mgr.add_member(&div.id, "cli_bob"));

        // Duplicate add is a no-op
        assert!(mgr.add_member(&div.id, "cli_alice"));

        let updated = mgr.get(&div.id).unwrap();
        assert_eq!(updated.members.len(), 2);
        assert!(updated.members.contains(&"cli_alice".to_string()));

        // Remove one member
        assert!(mgr.remove_member(&div.id, "cli_alice"));
        let updated2 = mgr.get(&div.id).unwrap();
        assert_eq!(updated2.members.len(), 1);
        assert!(!updated2.members.contains(&"cli_alice".to_string()));

        // Non-existent division returns false
        assert!(!mgr.remove_member(&"div_nope".to_string(), "cli_x"));
    }

    #[test]
    fn test_remove_leader_clears_leader_field() {
        let mgr = DivisionManager::new();
        let clan_id: ClanId = "clan_clear".to_string();
        let div = mgr.create(&clan_id, "Ops", None);

        mgr.add_member(&div.id, "cli_leader");
        mgr.set_leader(&div.id, "cli_leader");

        // Removing the leader should clear the leader field
        mgr.remove_member(&div.id, "cli_leader");
        let updated = mgr.get(&div.id).unwrap();
        assert!(updated.leader.is_none());
    }

    #[test]
    fn test_division_count() {
        let mgr = DivisionManager::new();
        assert_eq!(mgr.division_count(), 0);

        let clan_id: ClanId = "clan_count".to_string();
        let d1 = mgr.create(&clan_id, "D1", None);
        let _d2 = mgr.create(&clan_id, "D2", None);
        assert_eq!(mgr.division_count(), 2);

        mgr.delete(&d1.id);
        assert_eq!(mgr.division_count(), 1);
    }
}
