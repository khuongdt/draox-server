/// Metadata describing a single REST endpoint contributed by the messaging plugin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessagingRouteInfo {
    pub method: &'static str,
    pub path: &'static str,
    pub description: &'static str,
}

/// Returns the full list of HTTP REST endpoint definitions for the messaging plugin.
///
/// These are declarative route records used for OpenAPI/Swagger documentation generation,
/// admin-api route registration, and capability discovery. They do not contain handler logic.
pub fn messaging_routes() -> Vec<MessagingRouteInfo> {
    vec![
        // ── Messages ─────────────────────────────────────────────────────────
        MessagingRouteInfo {
            method: "POST",
            path: "/api/messages/send",
            description: "Send a direct or channel message",
        },
        MessagingRouteInfo {
            method: "GET",
            path: "/api/messages/{id}",
            description: "Get a message by its ID",
        },
        MessagingRouteInfo {
            method: "DELETE",
            path: "/api/messages/{id}",
            description: "Delete a message by its ID",
        },
        MessagingRouteInfo {
            method: "PATCH",
            path: "/api/messages/{id}",
            description: "Edit the content of a message",
        },
        MessagingRouteInfo {
            method: "GET",
            path: "/api/messages/search",
            description: "Full-text search across messages",
        },
        MessagingRouteInfo {
            method: "POST",
            path: "/api/messages/{id}/react",
            description: "Add a reaction emoji to a message",
        },
        MessagingRouteInfo {
            method: "DELETE",
            path: "/api/messages/{id}/react/{emoji}",
            description: "Remove a reaction emoji from a message",
        },
        // ── Channels ─────────────────────────────────────────────────────────
        MessagingRouteInfo {
            method: "POST",
            path: "/api/channels",
            description: "Create a new messaging channel",
        },
        MessagingRouteInfo {
            method: "GET",
            path: "/api/channels",
            description: "List all available channels",
        },
        MessagingRouteInfo {
            method: "GET",
            path: "/api/channels/{id}",
            description: "Get channel details by ID",
        },
        MessagingRouteInfo {
            method: "DELETE",
            path: "/api/channels/{id}",
            description: "Delete a channel",
        },
        MessagingRouteInfo {
            method: "GET",
            path: "/api/channels/{id}/messages",
            description: "Get messages in a channel (paginated)",
        },
        MessagingRouteInfo {
            method: "POST",
            path: "/api/channels/{id}/subscribe",
            description: "Subscribe the current user to a channel",
        },
        MessagingRouteInfo {
            method: "POST",
            path: "/api/channels/{id}/unsubscribe",
            description: "Unsubscribe the current user from a channel",
        },
        // ── Presence ─────────────────────────────────────────────────────────
        MessagingRouteInfo {
            method: "GET",
            path: "/api/presence/{user_id}",
            description: "Get presence status for a user",
        },
        // ── Files ────────────────────────────────────────────────────────────
        MessagingRouteInfo {
            method: "POST",
            path: "/api/files/upload",
            description: "Upload a file and get a FileReference",
        },
        MessagingRouteInfo {
            method: "GET",
            path: "/api/files/{id}",
            description: "Get metadata for an uploaded file",
        },
        MessagingRouteInfo {
            method: "DELETE",
            path: "/api/files/{id}",
            description: "Delete an uploaded file",
        },
    ]
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_routes_non_empty_and_have_required_fields() {
        let routes = messaging_routes();
        assert!(!routes.is_empty(), "must define at least one route");

        for route in &routes {
            assert!(
                !route.method.is_empty(),
                "method must not be empty: {:?}",
                route
            );
            assert!(
                route.path.starts_with('/'),
                "path must start with '/': {:?}",
                route
            );
            assert!(
                !route.description.is_empty(),
                "description must not be empty: {:?}",
                route
            );
            assert!(
                ["GET", "POST", "PUT", "PATCH", "DELETE"].contains(&route.method),
                "unknown HTTP method '{}': {:?}",
                route.method,
                route
            );
        }
    }

    #[test]
    fn test_expected_endpoints_present() {
        let routes = messaging_routes();
        let lookup: HashSet<(&str, &str)> =
            routes.iter().map(|r| (r.method, r.path)).collect();

        // Core messaging endpoints that must exist.
        assert!(lookup.contains(&("POST", "/api/messages/send")));
        assert!(lookup.contains(&("GET", "/api/messages/{id}")));
        assert!(lookup.contains(&("POST", "/api/channels")));
        assert!(lookup.contains(&("GET", "/api/channels")));
        assert!(lookup.contains(&("POST", "/api/files/upload")));
        assert!(lookup.contains(&("GET", "/api/presence/{user_id}")));
    }
}
