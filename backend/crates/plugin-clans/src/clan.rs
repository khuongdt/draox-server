use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use server_core::ClientId;
use std::collections::HashMap;

/// Unique clan identifier.
pub type ClanId = String;

/// Role within a clan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClanRole {
    Owner,
    Officer,
    Member,
    Recruit,
}

impl ClanRole {
    pub fn can_manage_members(&self) -> bool {
        matches!(self, ClanRole::Owner | ClanRole::Officer)
    }

    pub fn can_manage_clan(&self) -> bool {
        matches!(self, ClanRole::Owner)
    }

    pub fn rank(&self) -> u8 {
        match self {
            ClanRole::Owner => 4,
            ClanRole::Officer => 3,
            ClanRole::Member => 2,
            ClanRole::Recruit => 1,
        }
    }
}

/// A clan membership entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClanMember {
    pub client_id: ClientId,
    pub role: ClanRole,
    pub joined_at: DateTime<Utc>,
}

/// A clan/group.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Clan {
    pub id: ClanId,
    pub name: String,
    pub tag: String,
    pub description: String,
    pub owner_id: ClientId,
    pub members: Vec<ClanMember>,
    pub max_members: usize,
    pub created_at: DateTime<Utc>,
    pub division: Option<String>,
    pub icon_url: String,
    pub tags: Vec<String>,
    pub settings: HashMap<String, String>,
}

impl Default for Clan {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            tag: String::new(),
            description: String::new(),
            owner_id: ClientId::from_str(""),
            members: Vec::new(),
            max_members: 50,
            created_at: Utc::now(),
            division: None,
            icon_url: String::new(),
            tags: Vec::new(),
            settings: HashMap::new(),
        }
    }
}

impl Clan {
    pub fn new(
        id: ClanId,
        name: String,
        tag: String,
        owner_id: ClientId,
        max_members: usize,
    ) -> Self {
        let now = Utc::now();
        let owner_member = ClanMember {
            client_id: owner_id.clone(),
            role: ClanRole::Owner,
            joined_at: now,
        };

        Self {
            id,
            name,
            tag,
            description: String::new(),
            owner_id,
            members: vec![owner_member],
            max_members,
            created_at: now,
            division: None,
            icon_url: String::new(),
            tags: Vec::new(),
            settings: HashMap::new(),
        }
    }

    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    pub fn is_full(&self) -> bool {
        self.members.len() >= self.max_members
    }

    pub fn find_member(&self, client_id: &ClientId) -> Option<&ClanMember> {
        self.members.iter().find(|m| &m.client_id == client_id)
    }

    pub fn get_role(&self, client_id: &ClientId) -> Option<ClanRole> {
        self.find_member(client_id).map(|m| m.role)
    }

    pub fn is_member(&self, client_id: &ClientId) -> bool {
        self.find_member(client_id).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clan_creation() {
        let owner = ClientId::from_str("cli_owner1");
        let clan = Clan::new(
            "clan_1".to_string(),
            "Test Clan".to_string(),
            "TC".to_string(),
            owner.clone(),
            50,
        );

        assert_eq!(clan.name, "Test Clan");
        assert_eq!(clan.member_count(), 1);
        assert!(!clan.is_full());
        assert!(clan.is_member(&owner));
        assert_eq!(clan.get_role(&owner), Some(ClanRole::Owner));
    }

    #[test]
    fn test_clan_role_permissions() {
        assert!(ClanRole::Owner.can_manage_clan());
        assert!(ClanRole::Owner.can_manage_members());
        assert!(!ClanRole::Officer.can_manage_clan());
        assert!(ClanRole::Officer.can_manage_members());
        assert!(!ClanRole::Member.can_manage_members());
        assert!(!ClanRole::Recruit.can_manage_members());
    }

    #[test]
    fn test_clan_role_rank() {
        assert!(ClanRole::Owner.rank() > ClanRole::Officer.rank());
        assert!(ClanRole::Officer.rank() > ClanRole::Member.rank());
        assert!(ClanRole::Member.rank() > ClanRole::Recruit.rank());
    }

    #[test]
    fn test_clan_metadata_serde_default() {
        // Deserialize JSON without the new metadata fields — serde(default) should fill them in
        let json = r#"{
            "id": "clan_abc",
            "name": "Legacy Clan",
            "tag": "LC",
            "description": "An old clan",
            "owner_id": "cli_owner",
            "members": [],
            "max_members": 50,
            "created_at": "2025-01-01T00:00:00Z",
            "division": null
        }"#;

        let clan: Clan = serde_json::from_str(json).unwrap();
        assert_eq!(clan.icon_url, "");
        assert!(clan.tags.is_empty());
        assert!(clan.settings.is_empty());

        // Round-trip with metadata populated
        let mut clan2 = Clan::new(
            "clan_rt".to_string(),
            "Round Trip".to_string(),
            "RT".to_string(),
            ClientId::from_str("cli_rt"),
            10,
        );
        clan2.icon_url = "https://example.com/icon.png".to_string();
        clan2.tags = vec!["competitive".to_string(), "pvp".to_string()];
        clan2.settings.insert("open_join".to_string(), "true".to_string());

        let serialized = serde_json::to_string(&clan2).unwrap();
        let deserialized: Clan = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.icon_url, "https://example.com/icon.png");
        assert_eq!(deserialized.tags.len(), 2);
        assert_eq!(deserialized.settings.get("open_join").unwrap(), "true");
    }
}
