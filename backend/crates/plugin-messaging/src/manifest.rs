use plugin_sdk::traits::{
    ActivationEvent, CommandContribution, PluginContributions, PluginPermissions,
    RouteContribution, SettingContribution,
};
use plugin_sdk::PluginManifest;

/// Build and return the `PluginManifest` for the built-in messaging plugin.
///
/// This manifest is used by the plugin-host for lifecycle management,
/// capability discovery, and OpenAPI/Swagger documentation generation.
pub fn messaging_manifest() -> PluginManifest {
    PluginManifest {
        id: "io.draox.messaging".to_string(),
        name: "Messaging".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        author: "Draox Team".to_string(),
        description: "Built-in plugin providing direct, channel, and broadcast messaging with \
            presence tracking, typing indicators, read receipts, and file attachments."
            .to_string(),
        plugin_type: "builtin".to_string(),
        activation: ActivationEvent::OnStartup,
        min_server_version: Some("0.1.0".to_string()),
        dependencies: std::collections::HashMap::new(),
        contributions: PluginContributions {
            commands: vec![
                CommandContribution {
                    name: "messaging.send".to_string(),
                    description: "Send a direct or channel message".to_string(),
                },
                CommandContribution {
                    name: "messaging.delete".to_string(),
                    description: "Delete a message by ID".to_string(),
                },
                CommandContribution {
                    name: "messaging.channel.create".to_string(),
                    description: "Create a new messaging channel".to_string(),
                },
                CommandContribution {
                    name: "messaging.channel.delete".to_string(),
                    description: "Delete a messaging channel".to_string(),
                },
                CommandContribution {
                    name: "messaging.presence.set".to_string(),
                    description: "Set the current user's presence status".to_string(),
                },
            ],
            routes: vec![
                RouteContribution {
                    method: "POST".to_string(),
                    path: "/api/messages/send".to_string(),
                    description: "Send a message".to_string(),
                },
                RouteContribution {
                    method: "GET".to_string(),
                    path: "/api/messages/{id}".to_string(),
                    description: "Get message by ID".to_string(),
                },
                RouteContribution {
                    method: "DELETE".to_string(),
                    path: "/api/messages/{id}".to_string(),
                    description: "Delete a message".to_string(),
                },
                RouteContribution {
                    method: "POST".to_string(),
                    path: "/api/channels".to_string(),
                    description: "Create a channel".to_string(),
                },
                RouteContribution {
                    method: "GET".to_string(),
                    path: "/api/channels".to_string(),
                    description: "List channels".to_string(),
                },
                RouteContribution {
                    method: "GET".to_string(),
                    path: "/api/channels/{id}/messages".to_string(),
                    description: "Get channel messages".to_string(),
                },
                RouteContribution {
                    method: "GET".to_string(),
                    path: "/api/presence/{user_id}".to_string(),
                    description: "Get user presence".to_string(),
                },
                RouteContribution {
                    method: "POST".to_string(),
                    path: "/api/files/upload".to_string(),
                    description: "Upload a file".to_string(),
                },
            ],
            events: vec![
                "messaging.message_sent".to_string(),
                "messaging.message_delivered".to_string(),
                "messaging.message_read".to_string(),
                "messaging.message_deleted".to_string(),
                "messaging.channel_created".to_string(),
                "messaging.channel_deleted".to_string(),
                "messaging.user_joined_channel".to_string(),
                "messaging.user_left_channel".to_string(),
                "messaging.presence_changed".to_string(),
                "messaging.typing_started".to_string(),
                "messaging.file_uploaded".to_string(),
            ],
            settings: vec![
                SettingContribution {
                    key: "messaging.max_messages".to_string(),
                    description: "Maximum number of messages to retain in memory".to_string(),
                    value_type: "number".to_string(),
                    default: Some(serde_json::json!(100_000)),
                },
                SettingContribution {
                    key: "messaging.max_offline_queue".to_string(),
                    description: "Maximum queued messages per offline user".to_string(),
                    value_type: "number".to_string(),
                    default: Some(serde_json::json!(100)),
                },
                SettingContribution {
                    key: "messaging.typing_timeout_secs".to_string(),
                    description: "Seconds before a typing indicator expires automatically"
                        .to_string(),
                    value_type: "number".to_string(),
                    default: Some(serde_json::json!(5)),
                },
                SettingContribution {
                    key: "messaging.moderation.max_messages_per_minute".to_string(),
                    description: "Rate limit: maximum messages a user can send per minute"
                        .to_string(),
                    value_type: "number".to_string(),
                    default: Some(serde_json::json!(60)),
                },
            ],
        },
        permissions: PluginPermissions {
            storage: true,
            cache: true,
            connections: true,
            events: true,
            scheduler: false,
            network: false,
            filesystem: false,
        },
        wasm: None,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_is_valid() {
        let manifest = messaging_manifest();
        // The PluginManifest::validate() method checks all required fields.
        manifest.validate().expect("messaging manifest should be valid");
    }

    #[test]
    fn test_manifest_fields() {
        let manifest = messaging_manifest();

        assert_eq!(manifest.id, "io.draox.messaging");
        assert_eq!(manifest.name, "Messaging");
        assert_eq!(manifest.plugin_type, "builtin");
        assert_eq!(manifest.activation, ActivationEvent::OnStartup);
        assert!(manifest.wasm.is_none(), "builtin plugins have no wasm section");

        // Permissions
        assert!(manifest.permissions.storage);
        assert!(manifest.permissions.cache);
        assert!(manifest.permissions.connections);
        assert!(manifest.permissions.events);
        assert!(!manifest.permissions.network);
        assert!(!manifest.permissions.filesystem);

        // Contributions
        assert!(!manifest.contributions.commands.is_empty());
        assert!(!manifest.contributions.routes.is_empty());
        assert!(!manifest.contributions.events.is_empty());
        assert!(!manifest.contributions.settings.is_empty());

        // All declared events must be namespaced under "messaging."
        for event in &manifest.contributions.events {
            assert!(
                event.starts_with("messaging."),
                "event '{}' should be namespaced under 'messaging.'",
                event
            );
        }
    }
}
