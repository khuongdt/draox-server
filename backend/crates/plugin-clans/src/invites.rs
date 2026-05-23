use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use server_core::ClientId;
use tracing::debug;

use crate::clan::ClanId;

/// An invite code for joining a clan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClanInvite {
    pub code: String,
    pub clan_id: ClanId,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub max_uses: Option<u32>,
    pub use_count: u32,
}

impl ClanInvite {
    /// Returns `true` if the invite has passed its expiration time.
    pub fn is_expired(&self) -> bool {
        if let Some(expires) = self.expires_at {
            Utc::now() > expires
        } else {
            false
        }
    }

    /// Returns `true` if the invite has reached its maximum number of uses.
    pub fn is_exhausted(&self) -> bool {
        if let Some(max) = self.max_uses {
            self.use_count >= max
        } else {
            false
        }
    }

    /// Returns `true` if the invite is neither expired nor exhausted.
    pub fn is_valid(&self) -> bool {
        !self.is_expired() && !self.is_exhausted()
    }
}

/// Manages clan invitations.
pub struct InviteManager {
    /// code -> ClanInvite
    invites: DashMap<String, ClanInvite>,
    /// clan_id -> list of invite codes
    clan_invites: DashMap<ClanId, Vec<String>>,
}

impl InviteManager {
    /// Create a new, empty invite manager.
    pub fn new() -> Self {
        Self {
            invites: DashMap::new(),
            clan_invites: DashMap::new(),
        }
    }

    /// Generate a new invite code for a clan.
    ///
    /// - `expires_in_secs`: optional TTL in seconds from now.
    /// - `max_uses`: optional cap on how many times the code can be redeemed.
    ///
    /// Returns the generated invite code string.
    pub fn create_invite(
        &self,
        clan_id: ClanId,
        created_by: &ClientId,
        expires_in_secs: Option<u64>,
        max_uses: Option<u32>,
    ) -> String {
        let code = format!(
            "inv_{}",
            uuid::Uuid::new_v4()
                .as_simple()
                .to_string()
                .get(..8)
                .unwrap_or("00000000")
        );
        let now = Utc::now();
        let expires_at = expires_in_secs.map(|s| now + Duration::seconds(s as i64));

        let invite = ClanInvite {
            code: code.clone(),
            clan_id: clan_id.clone(),
            created_by: created_by.as_str().to_string(),
            created_at: now,
            expires_at,
            max_uses,
            use_count: 0,
        };

        self.invites.insert(code.clone(), invite);
        self.clan_invites
            .entry(clan_id)
            .or_default()
            .push(code.clone());
        debug!(code = %code, "invite created");
        code
    }

    /// Validate and consume an invite code. Returns the `clan_id` if valid.
    ///
    /// Increments `use_count` on success. Returns `None` if the code does not
    /// exist or the invite is expired / exhausted.
    pub fn use_invite(&self, code: &str) -> Option<ClanId> {
        let mut invite = self.invites.get_mut(code)?;
        if !invite.is_valid() {
            return None;
        }
        invite.use_count += 1;
        Some(invite.clan_id.clone())
    }

    /// Get an invite by code (cloned).
    pub fn get_invite(&self, code: &str) -> Option<ClanInvite> {
        self.invites.get(code).map(|r| r.value().clone())
    }

    /// List all invites for a clan (cloned).
    pub fn list_clan_invites(&self, clan_id: &ClanId) -> Vec<ClanInvite> {
        let codes = match self.clan_invites.get(clan_id) {
            Some(codes) => codes.value().clone(),
            None => return Vec::new(),
        };
        codes
            .iter()
            .filter_map(|code| self.invites.get(code).map(|r| r.value().clone()))
            .collect()
    }

    /// Revoke an invite code. Returns `true` if the code existed and was removed.
    pub fn revoke_invite(&self, code: &str) -> bool {
        let removed = self.invites.remove(code);
        if let Some((_, invite)) = &removed {
            // Remove the code from the clan_invites index
            if let Some(mut codes) = self.clan_invites.get_mut(&invite.clan_id) {
                codes.retain(|c| c != code);
            }
        }
        removed.is_some()
    }

    /// Clean up expired or exhausted invites. Returns how many were removed.
    pub fn cleanup(&self) -> usize {
        let mut to_remove = Vec::new();

        for entry in self.invites.iter() {
            if !entry.value().is_valid() {
                to_remove.push(entry.key().clone());
            }
        }

        let count = to_remove.len();
        for code in &to_remove {
            self.revoke_invite(code);
        }
        if count > 0 {
            debug!(removed = count, "expired/exhausted invites cleaned up");
        }
        count
    }

    /// Total number of active (not yet revoked) invites across all clans.
    pub fn invite_count(&self) -> usize {
        self.invites.len()
    }
}

impl Default for InviteManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_client(name: &str) -> ClientId {
        ClientId::from_str(name)
    }

    #[test]
    fn test_create_and_use_invite() {
        let mgr = InviteManager::new();
        let creator = make_client("cli_creator");
        let clan_id: ClanId = "clan_abc".to_string();

        let code = mgr.create_invite(clan_id.clone(), &creator, None, None);
        assert!(code.starts_with("inv_"));
        assert_eq!(mgr.invite_count(), 1);

        // Using the invite should succeed and return the clan id
        let result = mgr.use_invite(&code);
        assert_eq!(result, Some(clan_id));

        // Verify use_count incremented
        let invite = mgr.get_invite(&code).unwrap();
        assert_eq!(invite.use_count, 1);
    }

    #[test]
    fn test_expired_invite_rejected() {
        let mgr = InviteManager::new();
        let creator = make_client("cli_creator");
        let clan_id: ClanId = "clan_exp".to_string();

        // Create an invite that expired 0 seconds ago (already expired by the
        // time we try to use it — we pass 0 so expires_at == created_at).
        let code = mgr.create_invite(clan_id.clone(), &creator, Some(0), None);

        // The invite should be considered expired
        let invite = mgr.get_invite(&code).unwrap();
        // With 0 seconds, expires_at == created_at, and Utc::now() >= expires_at
        assert!(invite.is_expired() || invite.expires_at.unwrap() <= Utc::now());

        // Using an expired invite returns None
        let result = mgr.use_invite(&code);
        assert!(result.is_none());
    }

    #[test]
    fn test_max_uses_exhausted() {
        let mgr = InviteManager::new();
        let creator = make_client("cli_creator");
        let clan_id: ClanId = "clan_max".to_string();

        // Allow only 2 uses
        let code = mgr.create_invite(clan_id.clone(), &creator, None, Some(2));

        assert_eq!(mgr.use_invite(&code), Some(clan_id.clone()));
        assert_eq!(mgr.use_invite(&code), Some(clan_id));

        // Third use should fail — exhausted
        assert!(mgr.use_invite(&code).is_none());

        let invite = mgr.get_invite(&code).unwrap();
        assert!(invite.is_exhausted());
    }

    #[test]
    fn test_revoke_invite() {
        let mgr = InviteManager::new();
        let creator = make_client("cli_creator");
        let clan_id: ClanId = "clan_rev".to_string();

        let code = mgr.create_invite(clan_id.clone(), &creator, None, None);
        assert_eq!(mgr.invite_count(), 1);

        // Revoke
        assert!(mgr.revoke_invite(&code));
        assert_eq!(mgr.invite_count(), 0);

        // Using a revoked code returns None
        assert!(mgr.use_invite(&code).is_none());

        // Revoking again returns false
        assert!(!mgr.revoke_invite(&code));

        // Clan invites list should be empty
        let clan_invites = mgr.list_clan_invites(&clan_id);
        assert!(clan_invites.is_empty());
    }

    #[test]
    fn test_list_clan_invites() {
        let mgr = InviteManager::new();
        let creator = make_client("cli_creator");
        let clan_a: ClanId = "clan_a".to_string();
        let clan_b: ClanId = "clan_b".to_string();

        // Create 3 invites for clan_a and 1 for clan_b
        mgr.create_invite(clan_a.clone(), &creator, None, None);
        mgr.create_invite(clan_a.clone(), &creator, None, Some(5));
        mgr.create_invite(clan_a.clone(), &creator, None, None);
        mgr.create_invite(clan_b.clone(), &creator, None, None);

        let invites_a = mgr.list_clan_invites(&clan_a);
        assert_eq!(invites_a.len(), 3);
        for inv in &invites_a {
            assert_eq!(inv.clan_id, clan_a);
        }

        let invites_b = mgr.list_clan_invites(&clan_b);
        assert_eq!(invites_b.len(), 1);
        assert_eq!(invites_b[0].clan_id, clan_b);

        // Non-existent clan returns empty
        let invites_none = mgr.list_clan_invites(&"clan_nope".to_string());
        assert!(invites_none.is_empty());
    }
}
