use plugin_sdk::traits::{
    ActivationEvent, CommandContribution, PluginContributions, PluginPermissions,
    RouteContribution, SettingContribution,
};
use plugin_sdk::PluginManifest;

/// Returns the default [`PluginManifest`] for the built-in Clans plugin.
///
/// The manifest describes the plugin identity, contributions (commands, API
/// routes, events, settings), and required permissions so that the plugin-host
/// can validate, register, and sandbox the plugin correctly.
pub fn clans_manifest() -> PluginManifest {
    PluginManifest {
        id: "com.draox.clans".to_string(),
        name: "Clans & Groups".to_string(),
        version: "1.0.0".to_string(),
        author: "Draox Team".to_string(),
        description: "Built-in plugin providing clan/group management: create clans, \
            manage membership, roles, divisions, channels, alliances, and invites."
            .to_string(),
        plugin_type: "builtin".to_string(),
        activation: ActivationEvent::OnStartup,
        min_server_version: Some("1.0.0".to_string()),
        dependencies: std::collections::HashMap::new(),
        contributions: PluginContributions {
            commands: vec![
                CommandContribution {
                    name: "clans.create".to_string(),
                    description: "Create a new clan".to_string(),
                },
                CommandContribution {
                    name: "clans.join".to_string(),
                    description: "Join a clan by invite code".to_string(),
                },
                CommandContribution {
                    name: "clans.leave".to_string(),
                    description: "Leave the current clan".to_string(),
                },
                CommandContribution {
                    name: "clans.invite".to_string(),
                    description: "Generate an invite code for the clan".to_string(),
                },
                CommandContribution {
                    name: "clans.kick".to_string(),
                    description: "Kick a member from the clan".to_string(),
                },
                CommandContribution {
                    name: "clans.ban".to_string(),
                    description: "Ban a member from the clan".to_string(),
                },
                CommandContribution {
                    name: "clans.promote".to_string(),
                    description: "Change a member's role".to_string(),
                },
                CommandContribution {
                    name: "clans.transfer".to_string(),
                    description: "Transfer clan ownership to another member".to_string(),
                },
            ],
            routes: vec![
                RouteContribution {
                    method: "GET".to_string(),
                    path: "/api/clans".to_string(),
                    description: "List all clans".to_string(),
                },
                RouteContribution {
                    method: "POST".to_string(),
                    path: "/api/clans".to_string(),
                    description: "Create a clan".to_string(),
                },
                RouteContribution {
                    method: "GET".to_string(),
                    path: "/api/clans/{id}".to_string(),
                    description: "Get clan details".to_string(),
                },
                RouteContribution {
                    method: "PUT".to_string(),
                    path: "/api/clans/{id}".to_string(),
                    description: "Update clan details".to_string(),
                },
                RouteContribution {
                    method: "DELETE".to_string(),
                    path: "/api/clans/{id}".to_string(),
                    description: "Delete a clan".to_string(),
                },
                RouteContribution {
                    method: "GET".to_string(),
                    path: "/api/clans/{id}/members".to_string(),
                    description: "List clan members".to_string(),
                },
                RouteContribution {
                    method: "GET".to_string(),
                    path: "/api/clans/{id}/divisions".to_string(),
                    description: "List clan divisions".to_string(),
                },
                RouteContribution {
                    method: "GET".to_string(),
                    path: "/api/clans/{id}/channels".to_string(),
                    description: "List clan channels".to_string(),
                },
                RouteContribution {
                    method: "GET".to_string(),
                    path: "/api/clans/{id}/alliances".to_string(),
                    description: "List clan alliances".to_string(),
                },
            ],
            events: vec![
                "clan_created".to_string(),
                "clan_deleted".to_string(),
                "member_joined".to_string(),
                "member_left".to_string(),
                "member_kicked".to_string(),
                "member_banned".to_string(),
                "role_changed".to_string(),
                "ownership_transferred".to_string(),
                "alliance_formed".to_string(),
                "alliance_broken".to_string(),
                "division_created".to_string(),
                "channel_created".to_string(),
            ],
            settings: vec![
                SettingContribution {
                    key: "clans.max_members".to_string(),
                    description: "Maximum number of members per clan".to_string(),
                    value_type: "integer".to_string(),
                    default: Some(serde_json::json!(100)),
                },
                SettingContribution {
                    key: "clans.invite_ttl_seconds".to_string(),
                    description: "Default invite expiry in seconds (0 = no expiry)".to_string(),
                    value_type: "integer".to_string(),
                    default: Some(serde_json::json!(86400)),
                },
                SettingContribution {
                    key: "clans.max_divisions".to_string(),
                    description: "Maximum number of divisions per clan".to_string(),
                    value_type: "integer".to_string(),
                    default: Some(serde_json::json!(10)),
                },
                SettingContribution {
                    key: "clans.max_channels".to_string(),
                    description: "Maximum number of channels per clan".to_string(),
                    value_type: "integer".to_string(),
                    default: Some(serde_json::json!(20)),
                },
            ],
        },
        permissions: PluginPermissions {
            storage: true,
            cache: true,
            connections: false,
            events: true,
            scheduler: false,
            network: false,
            filesystem: false,
        },
        wasm: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_is_valid() {
        let manifest = clans_manifest();
        // Validate uses the same rules as PluginManifest::validate()
        manifest.validate().expect("clans manifest must be valid");

        assert_eq!(manifest.id, "com.draox.clans");
        assert_eq!(manifest.name, "Clans & Groups");
        assert_eq!(manifest.plugin_type, "builtin");
        // wasm section must be absent for builtin plugins
        assert!(manifest.wasm.is_none());
    }

    #[test]
    fn test_manifest_contributions() {
        let manifest = clans_manifest();
        let c = &manifest.contributions;

        // Commands
        assert!(!c.commands.is_empty(), "must have at least one command");
        let command_names: Vec<&str> = c.commands.iter().map(|cmd| cmd.name.as_str()).collect();
        assert!(command_names.contains(&"clans.create"));
        assert!(command_names.contains(&"clans.join"));
        assert!(command_names.contains(&"clans.leave"));

        // Routes
        assert!(!c.routes.is_empty(), "must have at least one route");
        assert!(c.routes.iter().any(|r| r.path == "/api/clans"));

        // Events — must match all ClanEvent variants
        let expected_events = [
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
        for ev in &expected_events {
            assert!(
                c.events.iter().any(|e| e == *ev),
                "missing event: {ev}"
            );
        }

        // Settings
        assert!(!c.settings.is_empty(), "must declare at least one setting");
        assert!(c.settings.iter().any(|s| s.key == "clans.max_members"));
    }
}
