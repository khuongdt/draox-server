use serde::{Deserialize, Serialize};

/// All events that the Clans plugin can emit onto the server EventBus.
///
/// These are standalone types — they do not depend on the EventBus directly
/// so that this crate remains free of runtime coupling.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClanEvent {
    ClanCreated {
        clan_id: String,
        name: String,
        owner: String,
    },
    ClanDeleted {
        clan_id: String,
    },
    MemberJoined {
        clan_id: String,
        member_id: String,
    },
    MemberLeft {
        clan_id: String,
        member_id: String,
    },
    MemberKicked {
        clan_id: String,
        member_id: String,
        by: String,
    },
    MemberBanned {
        clan_id: String,
        member_id: String,
        by: String,
    },
    RoleChanged {
        clan_id: String,
        member_id: String,
        old_role: String,
        new_role: String,
    },
    OwnershipTransferred {
        clan_id: String,
        old_owner: String,
        new_owner: String,
    },
    AllianceFormed {
        clan_a: String,
        clan_b: String,
    },
    AllianceBroken {
        clan_a: String,
        clan_b: String,
    },
    DivisionCreated {
        clan_id: String,
        division_id: String,
        name: String,
    },
    ChannelCreated {
        clan_id: String,
        channel_id: String,
        name: String,
    },
}

impl ClanEvent {
    /// A stable string key identifying the event variant.
    ///
    /// Used as the topic/subject when publishing to an EventBus.
    pub fn event_type(&self) -> &'static str {
        match self {
            ClanEvent::ClanCreated { .. } => "clan_created",
            ClanEvent::ClanDeleted { .. } => "clan_deleted",
            ClanEvent::MemberJoined { .. } => "member_joined",
            ClanEvent::MemberLeft { .. } => "member_left",
            ClanEvent::MemberKicked { .. } => "member_kicked",
            ClanEvent::MemberBanned { .. } => "member_banned",
            ClanEvent::RoleChanged { .. } => "role_changed",
            ClanEvent::OwnershipTransferred { .. } => "ownership_transferred",
            ClanEvent::AllianceFormed { .. } => "alliance_formed",
            ClanEvent::AllianceBroken { .. } => "alliance_broken",
            ClanEvent::DivisionCreated { .. } => "division_created",
            ClanEvent::ChannelCreated { .. } => "channel_created",
        }
    }

    /// Extract the primary `clan_id` from any event variant.
    ///
    /// For alliance events that involve two clans, `clan_a` is returned.
    pub fn clan_id(&self) -> &str {
        match self {
            ClanEvent::ClanCreated { clan_id, .. } => clan_id,
            ClanEvent::ClanDeleted { clan_id } => clan_id,
            ClanEvent::MemberJoined { clan_id, .. } => clan_id,
            ClanEvent::MemberLeft { clan_id, .. } => clan_id,
            ClanEvent::MemberKicked { clan_id, .. } => clan_id,
            ClanEvent::MemberBanned { clan_id, .. } => clan_id,
            ClanEvent::RoleChanged { clan_id, .. } => clan_id,
            ClanEvent::OwnershipTransferred { clan_id, .. } => clan_id,
            ClanEvent::AllianceFormed { clan_a, .. } => clan_a,
            ClanEvent::AllianceBroken { clan_a, .. } => clan_a,
            ClanEvent::DivisionCreated { clan_id, .. } => clan_id,
            ClanEvent::ChannelCreated { clan_id, .. } => clan_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_strings() {
        let events = vec![
            (
                ClanEvent::ClanCreated {
                    clan_id: "c1".into(),
                    name: "Clan 1".into(),
                    owner: "u1".into(),
                },
                "clan_created",
            ),
            (ClanEvent::ClanDeleted { clan_id: "c1".into() }, "clan_deleted"),
            (
                ClanEvent::MemberJoined { clan_id: "c1".into(), member_id: "u1".into() },
                "member_joined",
            ),
            (
                ClanEvent::MemberLeft { clan_id: "c1".into(), member_id: "u1".into() },
                "member_left",
            ),
            (
                ClanEvent::MemberKicked {
                    clan_id: "c1".into(),
                    member_id: "u1".into(),
                    by: "u2".into(),
                },
                "member_kicked",
            ),
            (
                ClanEvent::MemberBanned {
                    clan_id: "c1".into(),
                    member_id: "u1".into(),
                    by: "u2".into(),
                },
                "member_banned",
            ),
            (
                ClanEvent::RoleChanged {
                    clan_id: "c1".into(),
                    member_id: "u1".into(),
                    old_role: "recruit".into(),
                    new_role: "member".into(),
                },
                "role_changed",
            ),
            (
                ClanEvent::OwnershipTransferred {
                    clan_id: "c1".into(),
                    old_owner: "u1".into(),
                    new_owner: "u2".into(),
                },
                "ownership_transferred",
            ),
            (
                ClanEvent::AllianceFormed { clan_a: "c1".into(), clan_b: "c2".into() },
                "alliance_formed",
            ),
            (
                ClanEvent::AllianceBroken { clan_a: "c1".into(), clan_b: "c2".into() },
                "alliance_broken",
            ),
            (
                ClanEvent::DivisionCreated {
                    clan_id: "c1".into(),
                    division_id: "d1".into(),
                    name: "Div1".into(),
                },
                "division_created",
            ),
            (
                ClanEvent::ChannelCreated {
                    clan_id: "c1".into(),
                    channel_id: "ch1".into(),
                    name: "general".into(),
                },
                "channel_created",
            ),
        ];

        for (event, expected_type) in &events {
            assert_eq!(event.event_type(), *expected_type, "wrong type for {:?}", event);
        }
    }

    #[test]
    fn test_clan_id_extraction() {
        let ev = ClanEvent::MemberKicked {
            clan_id: "clan_xyz".into(),
            member_id: "u1".into(),
            by: "u2".into(),
        };
        assert_eq!(ev.clan_id(), "clan_xyz");

        // Alliance event uses clan_a as the primary clan_id
        let ev_alliance = ClanEvent::AllianceFormed {
            clan_a: "clan_first".into(),
            clan_b: "clan_second".into(),
        };
        assert_eq!(ev_alliance.clan_id(), "clan_first");
    }

    #[test]
    fn test_serde_round_trip() {
        let event = ClanEvent::RoleChanged {
            clan_id: "clan_abc".into(),
            member_id: "cli_123".into(),
            old_role: "recruit".into(),
            new_role: "officer".into(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("role_changed"));
        assert!(json.contains("clan_abc"));

        let decoded: ClanEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.event_type(), "role_changed");
        assert_eq!(decoded.clan_id(), "clan_abc");
    }

    #[test]
    fn test_all_variants_have_unique_type_strings() {
        let types = vec![
            "clan_created",
            "clan_deleted",
            "member_joined",
            "member_left",
            "member_kicked",
            "member_banned",
            "role_changed",
            "ownership_transferred",
            "alliance_formed",
            "alliance_broken",
            "division_created",
            "channel_created",
        ];

        // All 12 types must be unique
        let mut seen = std::collections::HashSet::new();
        for t in &types {
            assert!(seen.insert(*t), "duplicate event type: {t}");
        }
        assert_eq!(seen.len(), 12);
    }
}
