use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::clan::ClanId;

/// Alliance status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AllianceStatus {
    Proposed,
    Active,
    Rejected,
    Dissolved,
}

/// An alliance between two clans.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alliance {
    pub id: String,
    pub clan_a: ClanId,
    pub clan_b: ClanId,
    pub status: AllianceStatus,
    pub proposed_by: ClanId,
    pub proposed_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub dissolved_at: Option<DateTime<Utc>>,
}

/// Manages clan alliances.
pub struct AllianceManager {
    alliances: DashMap<String, Alliance>,
    /// clan_id -> list of alliance IDs
    clan_alliances: DashMap<ClanId, Vec<String>>,
}

impl AllianceManager {
    /// Create a new, empty alliance manager.
    pub fn new() -> Self {
        Self {
            alliances: DashMap::new(),
            clan_alliances: DashMap::new(),
        }
    }

    /// Propose an alliance between two clans.
    ///
    /// Returns the alliance ID on success, or an error string if an active
    /// alliance already exists or a proposal is already pending between these
    /// clans (in either direction).
    pub fn propose(&self, from_clan: ClanId, to_clan: ClanId) -> Result<String, String> {
        // Check if alliance already exists between these clans
        if self.are_allied(&from_clan, &to_clan) {
            return Err("alliance already exists".to_string());
        }
        // Check for pending proposal
        if self.has_pending_proposal(&from_clan, &to_clan) {
            return Err("proposal already pending".to_string());
        }

        let id = format!("alliance_{}", uuid::Uuid::new_v4().as_simple());
        let alliance = Alliance {
            id: id.clone(),
            clan_a: from_clan.clone(),
            clan_b: to_clan.clone(),
            status: AllianceStatus::Proposed,
            proposed_by: from_clan.clone(),
            proposed_at: Utc::now(),
            accepted_at: None,
            dissolved_at: None,
        };

        self.alliances.insert(id.clone(), alliance);
        self.clan_alliances
            .entry(from_clan)
            .or_default()
            .push(id.clone());
        self.clan_alliances
            .entry(to_clan)
            .or_default()
            .push(id.clone());

        info!(alliance_id = %id, "alliance proposed");
        Ok(id)
    }

    /// Accept a proposed alliance. Transitions it to `Active`.
    pub fn accept(&self, alliance_id: &str) -> Result<(), String> {
        let mut entry = self
            .alliances
            .get_mut(alliance_id)
            .ok_or_else(|| "alliance not found".to_string())?;

        if entry.status != AllianceStatus::Proposed {
            return Err(format!(
                "alliance cannot be accepted from {:?} status",
                entry.status
            ));
        }

        entry.status = AllianceStatus::Active;
        entry.accepted_at = Some(Utc::now());
        debug!(alliance_id = %alliance_id, "alliance accepted");
        Ok(())
    }

    /// Reject a proposed alliance. Transitions it to `Rejected`.
    pub fn reject(&self, alliance_id: &str) -> Result<(), String> {
        let mut entry = self
            .alliances
            .get_mut(alliance_id)
            .ok_or_else(|| "alliance not found".to_string())?;

        if entry.status != AllianceStatus::Proposed {
            return Err(format!(
                "alliance cannot be rejected from {:?} status",
                entry.status
            ));
        }

        entry.status = AllianceStatus::Rejected;
        debug!(alliance_id = %alliance_id, "alliance rejected");
        Ok(())
    }

    /// Dissolve an active alliance. Transitions it to `Dissolved`.
    pub fn dissolve(&self, alliance_id: &str) -> Result<(), String> {
        let mut entry = self
            .alliances
            .get_mut(alliance_id)
            .ok_or_else(|| "alliance not found".to_string())?;

        if entry.status != AllianceStatus::Active {
            return Err(format!(
                "alliance cannot be dissolved from {:?} status",
                entry.status
            ));
        }

        entry.status = AllianceStatus::Dissolved;
        entry.dissolved_at = Some(Utc::now());
        debug!(alliance_id = %alliance_id, "alliance dissolved");
        Ok(())
    }

    /// Check if two clans are currently allied (status `Active`).
    ///
    /// Order-independent: `are_allied(a, b)` == `are_allied(b, a)`.
    pub fn are_allied(&self, clan_a: &ClanId, clan_b: &ClanId) -> bool {
        let ids = match self.clan_alliances.get(clan_a) {
            Some(ids) => ids.value().clone(),
            None => return false,
        };
        for aid in &ids {
            if let Some(alliance) = self.alliances.get(aid) {
                let a = &alliance.clan_a;
                let b = &alliance.clan_b;
                let involves_both =
                    (a == clan_a && b == clan_b) || (a == clan_b && b == clan_a);
                if involves_both && alliance.status == AllianceStatus::Active {
                    return true;
                }
            }
        }
        false
    }

    /// Check if there's a pending proposal between two clans (in either direction).
    fn has_pending_proposal(&self, clan_a: &ClanId, clan_b: &ClanId) -> bool {
        let ids = match self.clan_alliances.get(clan_a) {
            Some(ids) => ids.value().clone(),
            None => return false,
        };
        for aid in &ids {
            if let Some(alliance) = self.alliances.get(aid) {
                let a = &alliance.clan_a;
                let b = &alliance.clan_b;
                let involves_both =
                    (a == clan_a && b == clan_b) || (a == clan_b && b == clan_a);
                if involves_both && alliance.status == AllianceStatus::Proposed {
                    return true;
                }
            }
        }
        false
    }

    /// Get all alliances for a clan (cloned, any status).
    pub fn get_clan_alliances(&self, clan_id: &ClanId) -> Vec<Alliance> {
        let ids = match self.clan_alliances.get(clan_id) {
            Some(ids) => ids.value().clone(),
            None => return Vec::new(),
        };
        ids.iter()
            .filter_map(|aid| self.alliances.get(aid).map(|r| r.value().clone()))
            .collect()
    }

    /// Get active allies of a clan (returns their clan IDs).
    pub fn get_allies(&self, clan_id: &ClanId) -> Vec<ClanId> {
        let ids = match self.clan_alliances.get(clan_id) {
            Some(ids) => ids.value().clone(),
            None => return Vec::new(),
        };
        ids.iter()
            .filter_map(|aid| {
                let alliance = self.alliances.get(aid)?;
                if alliance.status != AllianceStatus::Active {
                    return None;
                }
                // Return the *other* clan in the alliance
                if &alliance.clan_a == clan_id {
                    Some(alliance.clan_b.clone())
                } else {
                    Some(alliance.clan_a.clone())
                }
            })
            .collect()
    }

    /// Total number of alliances (all statuses).
    pub fn alliance_count(&self) -> usize {
        self.alliances.len()
    }
}

impl Default for AllianceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_propose_and_accept() {
        let mgr = AllianceManager::new();
        let clan_a: ClanId = "clan_alpha".to_string();
        let clan_b: ClanId = "clan_beta".to_string();

        let id = mgr.propose(clan_a.clone(), clan_b.clone()).unwrap();
        assert!(id.starts_with("alliance_"));
        assert_eq!(mgr.alliance_count(), 1);

        // Before acceptance, should not be allied
        assert!(!mgr.are_allied(&clan_a, &clan_b));

        // Accept the proposal
        mgr.accept(&id).unwrap();
        assert!(mgr.are_allied(&clan_a, &clan_b));
        // Order-independent
        assert!(mgr.are_allied(&clan_b, &clan_a));

        // Verify accepted_at is set
        let alliance = mgr.alliances.get(&id).unwrap();
        assert_eq!(alliance.status, AllianceStatus::Active);
        assert!(alliance.accepted_at.is_some());
    }

    #[test]
    fn test_reject_proposal() {
        let mgr = AllianceManager::new();
        let clan_a: ClanId = "clan_alpha".to_string();
        let clan_b: ClanId = "clan_beta".to_string();

        let id = mgr.propose(clan_a.clone(), clan_b.clone()).unwrap();

        // Reject
        mgr.reject(&id).unwrap();

        assert!(!mgr.are_allied(&clan_a, &clan_b));

        // Verify status in a scoped block so the Ref guard is dropped
        // before we call accept (which requires a write lock).
        {
            let alliance = mgr.alliances.get(&id).unwrap();
            assert_eq!(alliance.status, AllianceStatus::Rejected);
        }

        // Cannot accept a rejected alliance
        assert!(mgr.accept(&id).is_err());
    }

    #[test]
    fn test_dissolve_alliance() {
        let mgr = AllianceManager::new();
        let clan_a: ClanId = "clan_alpha".to_string();
        let clan_b: ClanId = "clan_beta".to_string();

        let id = mgr.propose(clan_a.clone(), clan_b.clone()).unwrap();
        mgr.accept(&id).unwrap();
        assert!(mgr.are_allied(&clan_a, &clan_b));

        // Dissolve
        mgr.dissolve(&id).unwrap();
        assert!(!mgr.are_allied(&clan_a, &clan_b));

        // Verify status in a scoped block so the Ref guard is dropped
        // before we call dissolve again (which requires a write lock).
        {
            let alliance = mgr.alliances.get(&id).unwrap();
            assert_eq!(alliance.status, AllianceStatus::Dissolved);
            assert!(alliance.dissolved_at.is_some());
        }

        // Cannot dissolve again
        assert!(mgr.dissolve(&id).is_err());
    }

    #[test]
    fn test_duplicate_prevention() {
        let mgr = AllianceManager::new();
        let clan_a: ClanId = "clan_alpha".to_string();
        let clan_b: ClanId = "clan_beta".to_string();

        let id = mgr.propose(clan_a.clone(), clan_b.clone()).unwrap();

        // Duplicate proposal (same direction) should fail
        let dup = mgr.propose(clan_a.clone(), clan_b.clone());
        assert!(dup.is_err());
        assert_eq!(dup.unwrap_err(), "proposal already pending");

        // Duplicate proposal (reverse direction) should also fail
        let dup_rev = mgr.propose(clan_b.clone(), clan_a.clone());
        assert!(dup_rev.is_err());
        assert_eq!(dup_rev.unwrap_err(), "proposal already pending");

        // After accepting, proposing again should fail with "already exists"
        mgr.accept(&id).unwrap();
        let dup_active = mgr.propose(clan_a.clone(), clan_b.clone());
        assert!(dup_active.is_err());
        assert_eq!(dup_active.unwrap_err(), "alliance already exists");
    }

    #[test]
    fn test_get_allies_list() {
        let mgr = AllianceManager::new();
        let clan_a: ClanId = "clan_alpha".to_string();
        let clan_b: ClanId = "clan_beta".to_string();
        let clan_c: ClanId = "clan_gamma".to_string();

        // A <-> B active alliance
        let id_ab = mgr.propose(clan_a.clone(), clan_b.clone()).unwrap();
        mgr.accept(&id_ab).unwrap();

        // A <-> C proposed only (not accepted)
        let _id_ac = mgr.propose(clan_a.clone(), clan_c.clone()).unwrap();

        // A's active allies should only include B
        let allies_a = mgr.get_allies(&clan_a);
        assert_eq!(allies_a.len(), 1);
        assert_eq!(allies_a[0], clan_b);

        // B's active allies should include A
        let allies_b = mgr.get_allies(&clan_b);
        assert_eq!(allies_b.len(), 1);
        assert_eq!(allies_b[0], clan_a);

        // C has no active allies yet
        let allies_c = mgr.get_allies(&clan_c);
        assert!(allies_c.is_empty());

        // All alliances for A (any status) should be 2
        let all_a = mgr.get_clan_alliances(&clan_a);
        assert_eq!(all_a.len(), 2);

        // Non-existent clan returns empty
        let no_clan = mgr.get_allies(&"clan_nope".to_string());
        assert!(no_clan.is_empty());
    }
}
