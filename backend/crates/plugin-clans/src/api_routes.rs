/// Metadata describing a single REST endpoint exposed by the Clans plugin.
///
/// These are **definitions only** — the actual axum handler registration lives
/// in the `admin-api` crate.  The Clans plugin publishes this table so that the
/// admin-api can auto-register routes and generate OpenAPI documentation without
/// hard-coding every path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClanRouteInfo {
    /// HTTP method: "GET", "POST", "PUT", "PATCH", or "DELETE".
    pub method: &'static str,
    /// Path template using `{param}` placeholders.
    pub path: &'static str,
    /// Human-readable description of what the endpoint does.
    pub description: &'static str,
    /// Minimum admin-API role required to call this endpoint.
    /// One of: "viewer", "operator", "admin".
    pub min_role: &'static str,
}

/// Return the full list of REST endpoints contributed by the Clans plugin.
///
/// Covers CRUD for clans, members, divisions, channels, alliances, and invites
/// (~25 endpoints).
pub fn clan_routes() -> Vec<ClanRouteInfo> {
    vec![
        // ── Clans CRUD ──────────────────────────────────────────────────────
        ClanRouteInfo {
            method: "GET",
            path: "/api/clans",
            description: "List all clans (paginated)",
            min_role: "viewer",
        },
        ClanRouteInfo {
            method: "POST",
            path: "/api/clans",
            description: "Create a new clan",
            min_role: "operator",
        },
        ClanRouteInfo {
            method: "GET",
            path: "/api/clans/search",
            description: "Search clans by name or tag",
            min_role: "viewer",
        },
        ClanRouteInfo {
            method: "GET",
            path: "/api/clans/{clan_id}",
            description: "Get clan details",
            min_role: "viewer",
        },
        ClanRouteInfo {
            method: "PUT",
            path: "/api/clans/{clan_id}",
            description: "Update clan metadata (name, tag, description, icon_url, tags)",
            min_role: "operator",
        },
        ClanRouteInfo {
            method: "DELETE",
            path: "/api/clans/{clan_id}",
            description: "Delete a clan (owner only)",
            min_role: "operator",
        },
        ClanRouteInfo {
            method: "GET",
            path: "/api/clans/{clan_id}/stats",
            description: "Get clan statistics (member count, role distribution, age)",
            min_role: "viewer",
        },
        // ── Members ─────────────────────────────────────────────────────────
        ClanRouteInfo {
            method: "GET",
            path: "/api/clans/{clan_id}/members",
            description: "List all clan members",
            min_role: "viewer",
        },
        ClanRouteInfo {
            method: "PUT",
            path: "/api/clans/{clan_id}/members/{client_id}/role",
            description: "Update a member's role",
            min_role: "operator",
        },
        ClanRouteInfo {
            method: "DELETE",
            path: "/api/clans/{clan_id}/members/{client_id}",
            description: "Remove (kick) a member from the clan",
            min_role: "operator",
        },
        ClanRouteInfo {
            method: "POST",
            path: "/api/clans/{clan_id}/members/{client_id}/ban",
            description: "Ban a member from the clan",
            min_role: "operator",
        },
        ClanRouteInfo {
            method: "POST",
            path: "/api/clans/{clan_id}/transfer",
            description: "Transfer clan ownership to another member",
            min_role: "operator",
        },
        // ── Invites ─────────────────────────────────────────────────────────
        ClanRouteInfo {
            method: "GET",
            path: "/api/clans/{clan_id}/invites",
            description: "List active invite codes for a clan",
            min_role: "operator",
        },
        ClanRouteInfo {
            method: "POST",
            path: "/api/clans/{clan_id}/invites",
            description: "Generate a new invite code",
            min_role: "operator",
        },
        ClanRouteInfo {
            method: "DELETE",
            path: "/api/clans/{clan_id}/invites/{code}",
            description: "Revoke an invite code",
            min_role: "operator",
        },
        ClanRouteInfo {
            method: "POST",
            path: "/api/clans/join",
            description: "Join a clan using an invite code",
            min_role: "operator",
        },
        // ── Divisions ───────────────────────────────────────────────────────
        ClanRouteInfo {
            method: "GET",
            path: "/api/clans/{clan_id}/divisions",
            description: "List all divisions in a clan",
            min_role: "viewer",
        },
        ClanRouteInfo {
            method: "POST",
            path: "/api/clans/{clan_id}/divisions",
            description: "Create a new division",
            min_role: "operator",
        },
        ClanRouteInfo {
            method: "PUT",
            path: "/api/clans/{clan_id}/divisions/{div_id}",
            description: "Update a division (name, description, leader)",
            min_role: "operator",
        },
        ClanRouteInfo {
            method: "DELETE",
            path: "/api/clans/{clan_id}/divisions/{div_id}",
            description: "Delete a division",
            min_role: "operator",
        },
        // ── Channels ────────────────────────────────────────────────────────
        ClanRouteInfo {
            method: "GET",
            path: "/api/clans/{clan_id}/channels",
            description: "List all channels in a clan",
            min_role: "viewer",
        },
        ClanRouteInfo {
            method: "POST",
            path: "/api/clans/{clan_id}/channels",
            description: "Create a new channel",
            min_role: "operator",
        },
        ClanRouteInfo {
            method: "DELETE",
            path: "/api/clans/{clan_id}/channels/{channel_id}",
            description: "Delete a channel",
            min_role: "operator",
        },
        // ── Alliances ───────────────────────────────────────────────────────
        ClanRouteInfo {
            method: "GET",
            path: "/api/clans/{clan_id}/alliances",
            description: "List all alliances for a clan (any status)",
            min_role: "viewer",
        },
        ClanRouteInfo {
            method: "POST",
            path: "/api/clans/{clan_id}/alliances",
            description: "Propose an alliance with another clan",
            min_role: "operator",
        },
        ClanRouteInfo {
            method: "PUT",
            path: "/api/alliances/{alliance_id}/accept",
            description: "Accept a pending alliance proposal",
            min_role: "operator",
        },
        ClanRouteInfo {
            method: "PUT",
            path: "/api/alliances/{alliance_id}/reject",
            description: "Reject a pending alliance proposal",
            min_role: "operator",
        },
        ClanRouteInfo {
            method: "DELETE",
            path: "/api/alliances/{alliance_id}",
            description: "Dissolve an active alliance",
            min_role: "operator",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_count_and_methods() {
        let routes = clan_routes();

        // Must have at least 25 endpoints
        assert!(
            routes.len() >= 25,
            "expected ≥25 routes, got {}",
            routes.len()
        );

        // All methods must be valid HTTP verbs
        let valid_methods = ["GET", "POST", "PUT", "PATCH", "DELETE"];
        for route in &routes {
            assert!(
                valid_methods.contains(&route.method),
                "unknown HTTP method '{}' on path '{}'",
                route.method,
                route.path
            );
        }
    }

    #[test]
    fn test_all_paths_start_with_slash() {
        for route in clan_routes() {
            assert!(
                route.path.starts_with('/'),
                "path '{}' must start with '/'",
                route.path
            );
        }
    }

    #[test]
    fn test_min_roles_are_valid() {
        let valid_roles = ["viewer", "operator", "admin"];
        for route in clan_routes() {
            assert!(
                valid_roles.contains(&route.min_role),
                "unknown min_role '{}' on path '{}'",
                route.min_role,
                route.path
            );
        }
    }

    #[test]
    fn test_coverage_of_crud_operations() {
        let routes = clan_routes();
        let base_clan_methods: Vec<&str> = routes
            .iter()
            .filter(|r| r.path == "/api/clans")
            .map(|r| r.method)
            .collect();

        assert!(base_clan_methods.contains(&"GET"), "need GET /api/clans");
        assert!(base_clan_methods.contains(&"POST"), "need POST /api/clans");

        let has_delete_clan = routes
            .iter()
            .any(|r| r.path == "/api/clans/{clan_id}" && r.method == "DELETE");
        assert!(has_delete_clan, "need DELETE /api/clans/{{clan_id}}");

        let has_divisions = routes.iter().any(|r| r.path.contains("divisions"));
        assert!(has_divisions, "need division endpoints");

        let has_channels = routes.iter().any(|r| r.path.contains("channels"));
        assert!(has_channels, "need channel endpoints");

        let has_alliances = routes.iter().any(|r| r.path.contains("alliances"));
        assert!(has_alliances, "need alliance endpoints");

        let has_invites = routes.iter().any(|r| r.path.contains("invites"));
        assert!(has_invites, "need invite endpoints");
    }
}
