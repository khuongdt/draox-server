pub mod auth;
pub mod auth_store;
pub mod error;
pub mod response;
pub mod routes;
pub mod seed;
pub mod server;
pub mod state;
pub mod trace_context;

#[cfg(test)]
pub(crate) mod test_support;

pub use server::{AdminServer, AdminServerConfig};
pub use state::AppState;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AdminRole;
    use crate::test_support::{admin_get, admin_request, as_user, make_state};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    async fn json_body(response: axum::response::Response) -> serde_json::Value {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let state = make_state().await;
        let app = routes::build_router(state).await;

        // Liveness stays public so probes (docker healthcheck) keep working.
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = json_body(response).await;
        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["status"], "ok");
    }

    #[tokio::test]
    async fn test_info_endpoint() {
        let state = make_state().await;
        let app = routes::build_router(state.clone()).await;

        let response = app.oneshot(admin_get(&state, "/api/info")).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = json_body(response).await;
        assert_eq!(json["data"]["name"], "Draox Server");
    }

    #[tokio::test]
    async fn test_connections_endpoint() {
        let state = make_state().await;
        let app = routes::build_router(state.clone()).await;

        let response = app
            .oneshot(admin_get(&state, "/api/connections"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = json_body(response).await;
        assert_eq!(json["data"]["total"], 0);
    }

    #[tokio::test]
    async fn test_sessions_endpoint() {
        let state = make_state().await;
        let app = routes::build_router(state.clone()).await;

        let response = app
            .oneshot(admin_get(&state, "/api/sessions"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = json_body(response).await;
        assert_eq!(json["data"]["total"], 0);
    }

    #[tokio::test]
    async fn test_plugins_endpoint() {
        let state = make_state().await;
        let app = routes::build_router(state.clone()).await;

        let response = app
            .oneshot(admin_get(&state, "/api/plugins"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = json_body(response).await;
        assert_eq!(json["data"]["total"], 0);
    }

    #[tokio::test]
    async fn test_guard_stats_endpoint() {
        let state = make_state().await;
        let app = routes::build_router(state.clone()).await;

        let response = app
            .oneshot(admin_get(&state, "/api/guard/stats"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = json_body(response).await;
        assert_eq!(json["data"]["active_bans"], 0);
    }

    #[tokio::test]
    async fn test_metrics_endpoint() {
        let state = make_state().await;

        // Record some metrics
        state.metrics.increment_connections();
        state.metrics.record_bytes_received(1024);
        state.metrics.increment_requests();

        let app = routes::build_router(state.clone()).await;

        let response = app.oneshot(admin_get(&state, "/api/metrics")).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = json_body(response).await;
        assert_eq!(json["data"]["connections_active"], 1);
        assert_eq!(json["data"]["bytes_received"], 1024); // renamed via #[serde(rename)]
        assert_eq!(json["data"]["requests_total"], 1);
    }

    #[tokio::test]
    async fn test_connection_not_found() {
        let state = make_state().await;
        let app = routes::build_router(state.clone()).await;

        let response = app
            .oneshot(admin_get(&state, "/api/connections/nonexistent"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_ban_and_unban() {
        let state = make_state().await;

        // Ban
        let app = routes::build_router(state.clone()).await;
        let response = app
            .oneshot(
                admin_request(&state)
                    .method("POST")
                    .uri("/api/guard/ban")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"ip":"10.0.0.1"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Verify ban is active
        assert_eq!(state.traffic_guard.ban_manager().active_ban_count(), 1);

        // Unban
        let app = routes::build_router(state.clone()).await;
        let response = app
            .oneshot(
                admin_request(&state)
                    .method("POST")
                    .uri("/api/guard/unban")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"ip":"10.0.0.1"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        assert_eq!(state.traffic_guard.ban_manager().active_ban_count(), 0);
    }

    // ── P0 security regressions ──────────────────────────────────────────────

    #[tokio::test]
    async fn test_admin_routes_require_authentication() {
        let state = make_state().await;
        let app = routes::build_router(state).await;

        // Mutations and reads from the report's finding list, plus a WS stream.
        let cases = [
            ("GET", "/api/users"),
            ("POST", "/api/users/viewer/ban"),
            ("POST", "/api/guard/ban"),
            ("POST", "/api/plugins/x/activate"),
            ("GET", "/api/connections"),
            ("GET", "/api/config"),
            ("GET", "/api/health/detailed"),
            ("GET", "/api/info"),
            ("GET", "/ws/events"),
        ];
        for (method, uri) in cases {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(uri)
                        .header("content-type", "application/json")
                        .body(Body::from(r#"{"ip":"10.0.0.9"}"#))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(
                response.status(),
                StatusCode::UNAUTHORIZED,
                "{method} {uri} must require auth"
            );
        }
    }

    #[tokio::test]
    async fn test_invalid_or_forged_token_is_rejected() {
        let state = make_state().await;
        let app = routes::build_router(state.clone()).await;

        let forged = crate::auth::create_jwt_token(
            "admin",
            AdminRole::Admin,
            &crate::auth::JwtConfig {
                secret: "attacker-secret".into(),
                expiry_secs: 3600,
            },
        )
        .unwrap();
        for token in ["garbage", forged.as_str()] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri("/api/connections")
                        .header("authorization", format!("Bearer {token}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }
    }

    #[tokio::test]
    async fn test_role_enforcement() {
        let state = make_state().await;
        let app = routes::build_router(state.clone()).await;

        let call = |username: &'static str, role, method: &'static str, uri: &'static str| {
            let app = app.clone();
            let req = as_user(&state, username, role)
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"ip":"10.0.0.7"}"#))
                .unwrap();
            async move { app.oneshot(req).await.unwrap().status() }
        };

        // Viewer: read-only.
        assert_eq!(call("viewer", AdminRole::Viewer, "GET", "/api/connections").await, StatusCode::OK);
        assert_eq!(call("viewer", AdminRole::Viewer, "POST", "/api/guard/ban").await, StatusCode::FORBIDDEN);
        assert_eq!(call("viewer", AdminRole::Viewer, "GET", "/api/users").await, StatusCode::FORBIDDEN);

        // Operator: operational writes, but no user/config management.
        assert_eq!(call("operator", AdminRole::Operator, "POST", "/api/guard/ban").await, StatusCode::OK);
        assert_eq!(call("operator", AdminRole::Operator, "GET", "/api/users").await, StatusCode::FORBIDDEN);
        assert_eq!(call("operator", AdminRole::Operator, "GET", "/api/config").await, StatusCode::FORBIDDEN);

        // Admin: everything.
        assert_eq!(call("admin", AdminRole::Admin, "GET", "/api/users").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn test_role_comes_from_store_not_token() {
        let state = make_state().await;
        let app = routes::build_router(state.clone()).await;

        // A token claiming admin for the seeded viewer account must not escalate.
        let response = app
            .clone()
            .oneshot(
                as_user(&state, "viewer", AdminRole::Admin)
                    .uri("/api/users")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        // Banning an account revokes its outstanding tokens immediately.
        let mut op = state.auth_store.get("operator").await.unwrap();
        op.banned = true;
        state.auth_store.set(&op).await.unwrap();
        let response = app
            .oneshot(
                as_user(&state, "operator", AdminRole::Operator)
                    .uri("/api/connections")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    /// Minimal plugin exposing `GET|POST /api/echo` that returns the caller identity.
    struct EchoPlugin(server_core::PluginId);

    impl plugin_sdk::Plugin for EchoPlugin {
        fn id(&self) -> &server_core::PluginId {
            &self.0
        }
        fn name(&self) -> &str {
            "echo"
        }
        fn version(&self) -> &str {
            "0.0.0"
        }
        fn activate(
            &mut self,
            _ctx: plugin_sdk::PluginContext,
        ) -> plugin_sdk::BoxFuture<'_, server_core::Result<()>> {
            Box::pin(async { Ok(()) })
        }
        fn deactivate(&mut self) -> plugin_sdk::BoxFuture<'_, server_core::Result<()>> {
            Box::pin(async { Ok(()) })
        }
        fn http_router(&self) -> Option<axum::Router> {
            async fn echo(
                axum::Extension(id): axum::Extension<plugin_sdk::Identity>,
            ) -> String {
                id.user_id
            }
            Some(axum::Router::new().route("/api/echo", axum::routing::get(echo).post(echo)))
        }
    }

    #[tokio::test]
    async fn test_plugin_routes_share_admin_auth() {
        let state = make_state().await;
        let id = server_core::PluginId::from("io.draox.test.echo");
        state
            .plugin_registry
            .register_builtin(Box::new(EchoPlugin(id.clone())))
            .unwrap();
        state.plugin_registry.activate(&id).await.unwrap();
        let app = routes::build_router(state.clone()).await;

        let call = |username: &'static str, role, method: &'static str| {
            let app = app.clone();
            let req = as_user(&state, username, role)
                .method(method)
                .uri("/api/echo")
                .body(Body::empty())
                .unwrap();
            async move { app.oneshot(req).await.unwrap().status() }
        };

        // Identity is still delivered to plugin handlers.
        assert_eq!(call("viewer", AdminRole::Viewer, "GET").await, StatusCode::OK);
        // Role policy now applies: viewers cannot mutate through plugin routes.
        assert_eq!(call("viewer", AdminRole::Viewer, "POST").await, StatusCode::FORBIDDEN);
        assert_eq!(call("operator", AdminRole::Operator, "POST").await, StatusCode::OK);

        // Tokens of a banned account no longer work on plugin routes either.
        let mut op = state.auth_store.get("operator").await.unwrap();
        op.banned = true;
        state.auth_store.set(&op).await.unwrap();
        assert_eq!(call("operator", AdminRole::Operator, "GET").await, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_api_key_auth() {
        let state = make_state().await;
        state.config.write().unwrap().admin_api.api_keys =
            vec!["viewer:view-key".into(), "admin-key".into()];
        let app = routes::build_router(state.clone()).await;

        let call = |key: &'static str, uri: &'static str| {
            let app = app.clone();
            let req = Request::builder()
                .uri(uri)
                .header("x-api-key", key)
                .body(Body::empty())
                .unwrap();
            async move { app.oneshot(req).await.unwrap().status() }
        };

        assert_eq!(call("view-key", "/api/connections").await, StatusCode::OK);
        assert_eq!(call("view-key", "/api/users").await, StatusCode::FORBIDDEN);
        assert_eq!(call("admin-key", "/api/users").await, StatusCode::OK);
        assert_eq!(call("wrong-key", "/api/connections").await, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_ws_stream_accepts_query_token() {
        let state = make_state().await;
        let app = routes::build_router(state.clone()).await;
        let token = crate::test_support::token(&state, "viewer", AdminRole::Viewer);

        // Passes auth; `oneshot` has no upgradable connection, so the WS extractor
        // then rejects it — anything but 401/403 proves the token was accepted.
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/ws/events?token={token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_ne!(response.status(), StatusCode::UNAUTHORIZED);
        assert_ne!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_query_token_not_accepted_on_rest_routes() {
        let state = make_state().await;
        let app = routes::build_router(state.clone()).await;
        let token = crate::test_support::token(&state, "admin", AdminRole::Admin);

        // Tokens in URLs leak via logs/referrers; only WS upgrades may use them.
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/connections?token={token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_get_config_no_longer_500() {
        let state = make_state().await;
        let app = routes::build_router(state.clone()).await;

        let response = app.oneshot(admin_get(&state, "/api/config")).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = json_body(response).await;
        assert_eq!(json["data"]["admin_api"]["jwt_secret"], "[REDACTED]");
    }

    #[tokio::test]
    async fn test_update_and_reload_config_record_caller() {
        let state = make_state().await;
        let dir = std::env::temp_dir().join(format!("draox-cfg-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        let mut state = state;
        state.config_path = path.to_string_lossy().into_owned();
        let app = routes::build_router(state.clone()).await;

        // Round-trip the (redacted) config: previously 500 because AuthContext was missing.
        let current = json_body(app.clone().oneshot(admin_get(&state, "/api/config")).await.unwrap())
            .await["data"]
            .clone();
        let response = app
            .clone()
            .oneshot(
                admin_request(&state)
                    .method("PUT")
                    .uri("/api/config")
                    .header("content-type", "application/json")
                    .body(Body::from(current.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(path.exists(), "config file should be written");

        let response = app
            .oneshot(
                admin_request(&state)
                    .method("POST")
                    .uri("/api/config/reload")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Both config handlers must see the AuthContext inserted by admin_auth.
        assert_eq!(state.audit_log.query_by_actor("admin").len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_rate_limit_returns_429() {
        let state = make_state().await;
        state.config.write().unwrap().admin_api.rate_limit_per_sec = 2;
        let app = routes::build_router(state.clone()).await;

        let mut statuses = Vec::new();
        for _ in 0..5 {
            let response = app
                .clone()
                .oneshot(admin_get(&state, "/api/connections"))
                .await
                .unwrap();
            statuses.push(response.status());
        }
        assert!(statuses.contains(&StatusCode::OK));
        assert!(statuses.contains(&StatusCode::TOO_MANY_REQUESTS));
    }

    #[tokio::test]
    async fn test_login_has_tighter_rate_limit() {
        let state = make_state().await;
        state.config.write().unwrap().admin_api.login_rate_limit_per_min = 2;
        let app = routes::build_router(state).await;

        let mut last = StatusCode::OK;
        for _ in 0..3 {
            last = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/auth/login")
                        .header("content-type", "application/json")
                        .body(Body::from(r#"{"username":"nobody","password":"x"}"#))
                        .unwrap(),
                )
                .await
                .unwrap()
                .status();
        }
        assert_eq!(last, StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn test_dev_login_bypass_disabled_without_env() {
        // Debug test builds used to accept admin/draox unconditionally.
        if std::env::var("DRAOX_ENV").as_deref() == Ok("development") {
            return;
        }
        let state = make_state().await;
        // Give the seeded admin a real password so only the bypass could accept "draox".
        crate::seed::seed_default_users(&state.auth_store).await;
        let mut admin = state.auth_store.get("admin").await.unwrap();
        admin.password_hash = crate::seed::hash_password("a-real-password").unwrap();
        state.auth_store.set(&admin).await.unwrap();
        let app = routes::build_router(state).await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"username":"admin","password":"draox"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
