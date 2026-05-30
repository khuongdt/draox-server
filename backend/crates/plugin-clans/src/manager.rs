use crate::clan::{Clan, ClanId, ClanMember, ClanRole};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use server_core::{ClientId, Error, Result};
use std::collections::HashMap;
use tracing::{debug, info};

/// Statistics for a clan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClanStats {
    pub member_count: usize,
    pub max_members: usize,
    pub role_distribution: HashMap<String, usize>,
    pub created_at: DateTime<Utc>,
    pub age_days: i64,
}

/// Manages all clans: CRUD, membership, roles.
pub struct ClanManager {
    clans: DashMap<ClanId, Clan>,
    client_to_clan: DashMap<ClientId, ClanId>,
    banned_clients: DashMap<ClanId, dashmap::DashSet<String>>,
    default_max_members: usize,
}

impl ClanManager {
    pub fn new(default_max_members: usize) -> Self {
        Self {
            clans: DashMap::new(),
            client_to_clan: DashMap::new(),
            banned_clients: DashMap::new(),
            default_max_members,
        }
    }

    /// Create a new clan. The creator becomes the owner.
    pub fn create_clan(
        &self,
        name: String,
        tag: String,
        owner_id: ClientId,
    ) -> Result<ClanId> {
        // Check if owner already in a clan
        if self.client_to_clan.contains_key(&owner_id) {
            return Err(Error::Plugin {
                plugin_id: "io.draox.clans".to_string(),
                message: "client already in a clan".to_string(),
            });
        }

        let id = format!("clan_{}", uuid::Uuid::new_v4().as_simple());
        let clan = Clan::new(
            id.clone(),
            name.clone(),
            tag,
            owner_id.clone(),
            self.default_max_members,
        );

        self.client_to_clan.insert(owner_id, id.clone());
        self.clans.insert(id.clone(), clan);
        info!(clan_id = %id, name = %name, "clan created");
        Ok(id)
    }

    /// Create a clan with a caller-supplied id. Used by the seed path to
    /// install the system "Draox" clan with a stable id. Idempotent —
    /// returns `Err` when the id is already present so reactivation is a
    /// no-op.
    ///
    /// Unlike `create_clan`, this skips the "owner already in a clan"
    /// guard because the seed uses a synthetic `system` owner.
    pub fn create_clan_with_id(
        &self,
        id: ClanId,
        name: String,
        tag: String,
        owner_id: ClientId,
        is_system: bool,
    ) -> Result<()> {
        if self.clans.contains_key(&id) {
            return Err(Error::Plugin {
                plugin_id: "io.draox.clans".to_string(),
                message:   format!("clan already exists: {id}"),
            });
        }
        let mut clan = Clan::new(
            id.clone(),
            name.clone(),
            tag,
            owner_id.clone(),
            self.default_max_members,
        );
        clan.is_system = is_system;
        self.client_to_clan.insert(owner_id, id.clone());
        self.clans.insert(id.clone(), clan);
        info!(clan_id = %id, name = %name, is_system, "clan created with stable id");
        Ok(())
    }

    /// Freeze or unfreeze a clan. Frozen clans reject new join requests;
    /// existing members remain.
    pub fn set_clan_frozen(&self, clan_id: &ClanId, frozen: bool) -> Result<()> {
        let mut clan = self
            .clans
            .get_mut(clan_id)
            .ok_or_else(|| Error::Plugin {
                plugin_id: "io.draox.clans".to_string(),
                message:   format!("clan not found: {clan_id}"),
            })?;
        clan.frozen = frozen;
        Ok(())
    }

    /// Delete a clan. Only the owner can delete.
    pub fn delete_clan(&self, clan_id: &ClanId, requester: &ClientId) -> Result<()> {
        let clan = self.get_clan(clan_id)?;

        if &clan.owner_id != requester {
            return Err(Error::Forbidden("only the owner can delete the clan".to_string()));
        }

        // Remove all member mappings
        for member in &clan.members {
            self.client_to_clan.remove(&member.client_id);
        }
        self.clans.remove(clan_id);

        info!(clan_id = %clan_id, "clan deleted");
        Ok(())
    }

    /// Get a clan by ID (cloned).
    pub fn get_clan(&self, clan_id: &ClanId) -> Result<Clan> {
        self.clans
            .get(clan_id)
            .map(|r| r.value().clone())
            .ok_or_else(|| Error::Plugin {
                plugin_id: "io.draox.clans".to_string(),
                message: format!("clan not found: {clan_id}"),
            })
    }

    /// Get a client's clan ID.
    pub fn get_client_clan(&self, client_id: &ClientId) -> Option<ClanId> {
        self.client_to_clan.get(client_id).map(|r| r.value().clone())
    }

    /// Add a member to a clan.
    pub fn join_clan(
        &self,
        clan_id: &ClanId,
        client_id: ClientId,
    ) -> Result<()> {
        // Check if already in a clan
        if self.client_to_clan.contains_key(&client_id) {
            return Err(Error::Plugin {
                plugin_id: "io.draox.clans".to_string(),
                message: "client already in a clan".to_string(),
            });
        }

        // Check if banned from this clan
        if self.is_banned(clan_id, &client_id) {
            return Err(Error::Forbidden(
                "client is banned from this clan".to_string(),
            ));
        }

        let mut clan = self.clans.get_mut(clan_id).ok_or_else(|| Error::Plugin {
            plugin_id: "io.draox.clans".to_string(),
            message: format!("clan not found: {clan_id}"),
        })?;

        if clan.is_full() {
            return Err(Error::Plugin {
                plugin_id: "io.draox.clans".to_string(),
                message: "clan is full".to_string(),
            });
        }

        clan.members.push(ClanMember {
            client_id: client_id.clone(),
            role: ClanRole::Recruit,
            joined_at: Utc::now(),
        });

        self.client_to_clan.insert(client_id.clone(), clan_id.clone());
        debug!(clan_id = %clan_id, client_id = %client_id, "member joined clan");
        Ok(())
    }

    /// Remove a member from a clan.
    pub fn leave_clan(&self, clan_id: &ClanId, client_id: &ClientId) -> Result<()> {
        let mut clan = self.clans.get_mut(clan_id).ok_or_else(|| Error::Plugin {
            plugin_id: "io.draox.clans".to_string(),
            message: format!("clan not found: {clan_id}"),
        })?;

        // Owner can't leave (must transfer or delete)
        if &clan.owner_id == client_id {
            return Err(Error::Plugin {
                plugin_id: "io.draox.clans".to_string(),
                message: "owner cannot leave the clan, transfer ownership or delete".to_string(),
            });
        }

        let before = clan.members.len();
        clan.members.retain(|m| &m.client_id != client_id);
        if clan.members.len() == before {
            return Err(Error::Plugin {
                plugin_id: "io.draox.clans".to_string(),
                message: "client is not a member of this clan".to_string(),
            });
        }

        self.client_to_clan.remove(client_id);
        debug!(clan_id = %clan_id, client_id = %client_id, "member left clan");
        Ok(())
    }

    /// Update a member's role.
    pub fn set_role(
        &self,
        clan_id: &ClanId,
        target_id: &ClientId,
        new_role: ClanRole,
        requester: &ClientId,
    ) -> Result<()> {
        let mut clan = self.clans.get_mut(clan_id).ok_or_else(|| Error::Plugin {
            plugin_id: "io.draox.clans".to_string(),
            message: format!("clan not found: {clan_id}"),
        })?;

        // Check requester's permission
        let requester_role = clan.get_role(requester).ok_or_else(|| Error::Forbidden(
            "requester is not a member of this clan".to_string(),
        ))?;

        if !requester_role.can_manage_members() {
            return Err(Error::Forbidden(
                "insufficient permissions to manage members".to_string(),
            ));
        }

        // Can't set someone to Owner (ownership transfer is separate)
        if new_role == ClanRole::Owner {
            return Err(Error::Plugin {
                plugin_id: "io.draox.clans".to_string(),
                message: "use transfer_ownership to change owner".to_string(),
            });
        }

        // Find and update the target member
        let member = clan
            .members
            .iter_mut()
            .find(|m| &m.client_id == target_id)
            .ok_or_else(|| Error::Plugin {
                plugin_id: "io.draox.clans".to_string(),
                message: "target is not a member".to_string(),
            })?;

        member.role = new_role;
        debug!(clan_id = %clan_id, target = %target_id, role = ?new_role, "role updated");
        Ok(())
    }

    /// List all clans (summary).
    pub fn list_clans(&self) -> Vec<Clan> {
        self.clans.iter().map(|r| r.value().clone()).collect()
    }

    /// Total number of clans.
    pub fn clan_count(&self) -> usize {
        self.clans.len()
    }

    /// Transfer clan ownership to another member.
    pub fn transfer_ownership(
        &self,
        clan_id: &ClanId,
        new_owner_id: &ClientId,
        requester: &ClientId,
    ) -> Result<()> {
        let mut clan = self.clans.get_mut(clan_id).ok_or_else(|| Error::Plugin {
            plugin_id: "io.draox.clans".to_string(),
            message: format!("clan not found: {clan_id}"),
        })?;

        // Verify requester is the current owner
        if &clan.owner_id != requester {
            return Err(Error::Forbidden(
                "only the owner can transfer ownership".to_string(),
            ));
        }

        // Verify new owner is a member
        if !clan.is_member(new_owner_id) {
            return Err(Error::Plugin {
                plugin_id: "io.draox.clans".to_string(),
                message: "target is not a member of the clan".to_string(),
            });
        }

        // Change old owner's role to Officer
        if let Some(old_owner) = clan.members.iter_mut().find(|m| &m.client_id == requester) {
            old_owner.role = ClanRole::Officer;
        }

        // Change new owner's role to Owner
        if let Some(new_owner) = clan.members.iter_mut().find(|m| &m.client_id == new_owner_id) {
            new_owner.role = ClanRole::Owner;
        }

        // Update clan owner_id
        clan.owner_id = new_owner_id.clone();

        info!(
            clan_id = %clan_id,
            old_owner = %requester,
            new_owner = %new_owner_id,
            "clan ownership transferred"
        );
        Ok(())
    }

    /// Kick a member from the clan. Requires Officer or Owner role.
    pub fn kick_member(
        &self,
        clan_id: &ClanId,
        target_id: &ClientId,
        requester: &ClientId,
    ) -> Result<()> {
        let mut clan = self.clans.get_mut(clan_id).ok_or_else(|| Error::Plugin {
            plugin_id: "io.draox.clans".to_string(),
            message: format!("clan not found: {clan_id}"),
        })?;

        // Verify requester has manage_members permission
        let requester_role = clan.get_role(requester).ok_or_else(|| Error::Forbidden(
            "requester is not a member of this clan".to_string(),
        ))?;

        if !requester_role.can_manage_members() {
            return Err(Error::Forbidden(
                "insufficient permissions to kick members".to_string(),
            ));
        }

        // Can't kick the owner
        if &clan.owner_id == target_id {
            return Err(Error::Forbidden(
                "cannot kick the clan owner".to_string(),
            ));
        }

        // Can't kick someone of equal or higher rank
        let target_role = clan.get_role(target_id).ok_or_else(|| Error::Plugin {
            plugin_id: "io.draox.clans".to_string(),
            message: "target is not a member of this clan".to_string(),
        })?;

        if target_role.rank() >= requester_role.rank() {
            return Err(Error::Forbidden(
                "cannot kick a member of equal or higher rank".to_string(),
            ));
        }

        // Remove from members vec
        clan.members.retain(|m| &m.client_id != target_id);

        // Drop the mutable borrow before modifying client_to_clan
        drop(clan);

        // Remove from client_to_clan map
        self.client_to_clan.remove(target_id);

        info!(clan_id = %clan_id, target = %target_id, kicked_by = %requester, "member kicked from clan");
        Ok(())
    }

    /// Ban a member from the clan (kick + prevent rejoin).
    pub fn ban_member(
        &self,
        clan_id: &ClanId,
        target_id: &ClientId,
        requester: &ClientId,
    ) -> Result<()> {
        // Kick first (reuse kick_member logic)
        self.kick_member(clan_id, target_id, requester)?;

        // Add to banned set
        self.banned_clients
            .entry(clan_id.clone())
            .or_insert_with(dashmap::DashSet::new)
            .insert(target_id.as_str().to_string());

        info!(clan_id = %clan_id, target = %target_id, banned_by = %requester, "member banned from clan");
        Ok(())
    }

    /// Check if a client is banned from a clan.
    pub fn is_banned(&self, clan_id: &ClanId, client_id: &ClientId) -> bool {
        self.banned_clients
            .get(clan_id)
            .map(|set| set.contains(client_id.as_str()))
            .unwrap_or(false)
    }

    /// Search clans by name or tag (case-insensitive substring match).
    pub fn search_clans(&self, query: &str) -> Vec<Clan> {
        let q = query.to_lowercase();
        self.clans
            .iter()
            .filter(|r| {
                let clan = r.value();
                clan.name.to_lowercase().contains(&q) || clan.tag.to_lowercase().contains(&q)
            })
            .map(|r| r.value().clone())
            .collect()
    }

    /// Get clan statistics.
    pub fn get_stats(&self, clan_id: &ClanId) -> Result<ClanStats> {
        let clan = self.get_clan(clan_id)?;
        let now = Utc::now();

        let mut role_distribution = HashMap::new();
        for member in &clan.members {
            let role_name = format!("{:?}", member.role);
            *role_distribution.entry(role_name).or_insert(0) += 1;
        }

        let age_days = (now - clan.created_at).num_days();

        debug!(clan_id = %clan_id, members = clan.members.len(), age_days, "clan stats retrieved");

        Ok(ClanStats {
            member_count: clan.members.len(),
            max_members: clan.max_members,
            role_distribution,
            created_at: clan.created_at,
            age_days,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_clan() {
        let mgr = ClanManager::new(50);
        let owner = ClientId::from_str("cli_owner");
        let id = mgr
            .create_clan("My Clan".to_string(), "MC".to_string(), owner.clone())
            .unwrap();

        assert_eq!(mgr.clan_count(), 1);
        let clan = mgr.get_clan(&id).unwrap();
        assert_eq!(clan.name, "My Clan");
        assert_eq!(clan.member_count(), 1);
    }

    #[test]
    fn test_duplicate_clan_membership() {
        let mgr = ClanManager::new(50);
        let owner = ClientId::from_str("cli_owner");
        mgr.create_clan("Clan 1".to_string(), "C1".to_string(), owner.clone())
            .unwrap();

        // Trying to create another clan while in one should fail
        assert!(mgr
            .create_clan("Clan 2".to_string(), "C2".to_string(), owner)
            .is_err());
    }

    #[test]
    fn test_join_and_leave() {
        let mgr = ClanManager::new(50);
        let owner = ClientId::from_str("cli_owner");
        let member = ClientId::from_str("cli_member");
        let clan_id = mgr
            .create_clan("Test".to_string(), "T".to_string(), owner.clone())
            .unwrap();

        mgr.join_clan(&clan_id, member.clone()).unwrap();
        let clan = mgr.get_clan(&clan_id).unwrap();
        assert_eq!(clan.member_count(), 2);
        assert!(clan.is_member(&member));

        mgr.leave_clan(&clan_id, &member).unwrap();
        let clan = mgr.get_clan(&clan_id).unwrap();
        assert_eq!(clan.member_count(), 1);
        assert!(!clan.is_member(&member));
    }

    #[test]
    fn test_owner_cannot_leave() {
        let mgr = ClanManager::new(50);
        let owner = ClientId::from_str("cli_owner");
        let clan_id = mgr
            .create_clan("Test".to_string(), "T".to_string(), owner.clone())
            .unwrap();

        assert!(mgr.leave_clan(&clan_id, &owner).is_err());
    }

    #[test]
    fn test_clan_full() {
        let mgr = ClanManager::new(2); // Max 2 members
        let owner = ClientId::from_str("cli_owner");
        let m1 = ClientId::from_str("cli_m1");
        let m2 = ClientId::from_str("cli_m2");

        let clan_id = mgr
            .create_clan("Small".to_string(), "S".to_string(), owner)
            .unwrap();
        mgr.join_clan(&clan_id, m1).unwrap();
        assert!(mgr.join_clan(&clan_id, m2).is_err());
    }

    #[test]
    fn test_set_role() {
        let mgr = ClanManager::new(50);
        let owner = ClientId::from_str("cli_owner");
        let member = ClientId::from_str("cli_member");
        let clan_id = mgr
            .create_clan("Test".to_string(), "T".to_string(), owner.clone())
            .unwrap();

        mgr.join_clan(&clan_id, member.clone()).unwrap();
        mgr.set_role(&clan_id, &member, ClanRole::Officer, &owner)
            .unwrap();

        let clan = mgr.get_clan(&clan_id).unwrap();
        assert_eq!(clan.get_role(&member), Some(ClanRole::Officer));
    }

    #[test]
    fn test_delete_clan() {
        let mgr = ClanManager::new(50);
        let owner = ClientId::from_str("cli_owner");
        let member = ClientId::from_str("cli_member");
        let clan_id = mgr
            .create_clan("Test".to_string(), "T".to_string(), owner.clone())
            .unwrap();

        mgr.join_clan(&clan_id, member.clone()).unwrap();
        mgr.delete_clan(&clan_id, &owner).unwrap();

        assert_eq!(mgr.clan_count(), 0);
        assert!(mgr.get_client_clan(&owner).is_none());
        assert!(mgr.get_client_clan(&member).is_none());
    }

    #[test]
    fn test_transfer_ownership() {
        let mgr = ClanManager::new(50);
        let owner = ClientId::from_str("cli_owner");
        let member = ClientId::from_str("cli_member");
        let clan_id = mgr
            .create_clan("Test".to_string(), "T".to_string(), owner.clone())
            .unwrap();

        mgr.join_clan(&clan_id, member.clone()).unwrap();
        mgr.transfer_ownership(&clan_id, &member, &owner).unwrap();

        let clan = mgr.get_clan(&clan_id).unwrap();
        assert_eq!(clan.owner_id, member);
        assert_eq!(clan.get_role(&member), Some(ClanRole::Owner));
        assert_eq!(clan.get_role(&owner), Some(ClanRole::Officer));

        // Non-owner cannot transfer
        let other = ClientId::from_str("cli_other");
        mgr.join_clan(&clan_id, other.clone()).unwrap();
        assert!(mgr.transfer_ownership(&clan_id, &other, &owner).is_err());
    }

    #[test]
    fn test_kick_member() {
        let mgr = ClanManager::new(50);
        let owner = ClientId::from_str("cli_owner");
        let member = ClientId::from_str("cli_member");
        let clan_id = mgr
            .create_clan("Test".to_string(), "T".to_string(), owner.clone())
            .unwrap();

        mgr.join_clan(&clan_id, member.clone()).unwrap();
        assert_eq!(mgr.get_clan(&clan_id).unwrap().member_count(), 2);

        mgr.kick_member(&clan_id, &member, &owner).unwrap();

        let clan = mgr.get_clan(&clan_id).unwrap();
        assert_eq!(clan.member_count(), 1);
        assert!(!clan.is_member(&member));
        assert!(mgr.get_client_clan(&member).is_none());
    }

    #[test]
    fn test_kick_higher_rank_fails() {
        let mgr = ClanManager::new(50);
        let owner = ClientId::from_str("cli_owner");
        let officer = ClientId::from_str("cli_officer");
        let recruit = ClientId::from_str("cli_recruit");
        let clan_id = mgr
            .create_clan("Test".to_string(), "T".to_string(), owner.clone())
            .unwrap();

        mgr.join_clan(&clan_id, officer.clone()).unwrap();
        mgr.set_role(&clan_id, &officer, ClanRole::Officer, &owner)
            .unwrap();
        mgr.join_clan(&clan_id, recruit.clone()).unwrap();

        // Officer cannot kick owner
        assert!(mgr.kick_member(&clan_id, &owner, &officer).is_err());

        // Recruit cannot kick anyone (no manage_members permission)
        assert!(mgr.kick_member(&clan_id, &officer, &recruit).is_err());

        // Officer can kick recruit (lower rank)
        mgr.kick_member(&clan_id, &recruit, &officer).unwrap();
        assert!(!mgr.get_clan(&clan_id).unwrap().is_member(&recruit));
    }

    #[test]
    fn test_ban_member_prevents_rejoin() {
        let mgr = ClanManager::new(50);
        let owner = ClientId::from_str("cli_owner");
        let member = ClientId::from_str("cli_member");
        let clan_id = mgr
            .create_clan("Test".to_string(), "T".to_string(), owner.clone())
            .unwrap();

        mgr.join_clan(&clan_id, member.clone()).unwrap();

        // Ban the member
        mgr.ban_member(&clan_id, &member, &owner).unwrap();

        // Verify kicked
        assert!(!mgr.get_clan(&clan_id).unwrap().is_member(&member));
        assert!(mgr.get_client_clan(&member).is_none());

        // Verify banned
        assert!(mgr.is_banned(&clan_id, &member));

        // Attempting to rejoin should fail
        assert!(mgr.join_clan(&clan_id, member).is_err());
    }

    #[test]
    fn test_search_clans() {
        let mgr = ClanManager::new(50);
        let o1 = ClientId::from_str("cli_o1");
        let o2 = ClientId::from_str("cli_o2");
        let o3 = ClientId::from_str("cli_o3");

        mgr.create_clan("Alpha Warriors".to_string(), "AW".to_string(), o1)
            .unwrap();
        mgr.create_clan("Beta Knights".to_string(), "BK".to_string(), o2)
            .unwrap();
        mgr.create_clan("Gamma Warriors".to_string(), "GW".to_string(), o3)
            .unwrap();

        // Search by name substring (case-insensitive)
        let results = mgr.search_clans("warriors");
        assert_eq!(results.len(), 2);

        // Search by tag
        let results = mgr.search_clans("bk");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Beta Knights");

        // No match
        let results = mgr.search_clans("zzzzz");
        assert!(results.is_empty());
    }

    #[test]
    fn test_get_stats() {
        let mgr = ClanManager::new(50);
        let owner = ClientId::from_str("cli_owner");
        let m1 = ClientId::from_str("cli_m1");
        let m2 = ClientId::from_str("cli_m2");
        let clan_id = mgr
            .create_clan("Stats Clan".to_string(), "SC".to_string(), owner.clone())
            .unwrap();

        mgr.join_clan(&clan_id, m1.clone()).unwrap();
        mgr.join_clan(&clan_id, m2.clone()).unwrap();
        mgr.set_role(&clan_id, &m1, ClanRole::Officer, &owner)
            .unwrap();

        let stats = mgr.get_stats(&clan_id).unwrap();
        assert_eq!(stats.member_count, 3);
        assert_eq!(stats.max_members, 50);
        assert_eq!(stats.role_distribution.get("Owner"), Some(&1));
        assert_eq!(stats.role_distribution.get("Officer"), Some(&1));
        assert_eq!(stats.role_distribution.get("Recruit"), Some(&1));
        assert!(stats.age_days >= 0);
    }

    #[test]
    fn test_clan_metadata_fields() {
        let mgr = ClanManager::new(50);
        let owner = ClientId::from_str("cli_owner");
        let clan_id = mgr
            .create_clan("Meta Clan".to_string(), "MT".to_string(), owner)
            .unwrap();

        let clan = mgr.get_clan(&clan_id).unwrap();
        // New metadata fields should be empty by default
        assert_eq!(clan.icon_url, "");
        assert!(clan.tags.is_empty());
        assert!(clan.settings.is_empty());
    }
}
