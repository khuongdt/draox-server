use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use governor::clock::DefaultClock;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;
use std::sync::Arc;

// ────────────────────────────────────────────────────────
// Rate Limiting
// ────────────────────────────────────────────────────────

/// A shared rate limiter for the admin API.
pub type AdminRateLimiter = Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>;

/// Create a new admin API rate limiter.
/// Default: 100 requests per second.
pub fn create_admin_rate_limiter(requests_per_sec: u32) -> AdminRateLimiter {
    let quota = Quota::per_second(
        NonZeroU32::new(requests_per_sec).unwrap_or(NonZeroU32::new(100).unwrap()),
    );
    Arc::new(RateLimiter::direct(quota))
}

/// Middleware that rate-limits admin API requests.
pub async fn rate_limit_middleware(request: Request, next: Next) -> Response {
    // Try to get the rate limiter from extensions
    if let Some(limiter) = request.extensions().get::<AdminRateLimiter>() {
        match limiter.check() {
            Ok(_) => next.run(request).await,
            Err(_) => (
                StatusCode::TOO_MANY_REQUESTS,
                axum::Json(serde_json::json!({"error": "rate limit exceeded"})),
            )
                .into_response(),
        }
    } else {
        // No rate limiter configured, pass through
        next.run(request).await
    }
}

// ────────────────────────────────────────────────────────
// RBAC
// ────────────────────────────────────────────────────────

/// Admin role for RBAC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdminRole {
    Admin,
    Operator,
    Viewer,
}

impl AdminRole {
    pub fn can_write(&self) -> bool {
        matches!(self, AdminRole::Admin | AdminRole::Operator)
    }

    pub fn can_admin(&self) -> bool {
        matches!(self, AdminRole::Admin)
    }
}

/// Auth context extracted from request headers.
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub role: AdminRole,
    pub identity: String,
}

/// API key entry for static key auth.
#[derive(Debug, Clone)]
pub struct ApiKeyEntry {
    pub key: String,
    pub role: AdminRole,
    pub identity: String,
}

// ────────────────────────────────────────────────────────
// JWT Claims
// ────────────────────────────────────────────────────────

/// JWT claims for admin API tokens.
#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    /// Subject (identity).
    pub sub: String,
    /// Role.
    pub role: AdminRole,
    /// Expiration time (UNIX timestamp).
    pub exp: u64,
    /// Issued at (UNIX timestamp).
    pub iat: u64,
}

/// JWT configuration.
#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub expiry_secs: u64,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: "draox-default-jwt-secret-change-me".to_string(),
            expiry_secs: 3600,
        }
    }
}

/// Create a JWT token.
pub fn create_jwt_token(
    identity: &str,
    role: AdminRole,
    config: &JwtConfig,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = chrono::Utc::now().timestamp() as u64;
    let claims = JwtClaims {
        sub: identity.to_string(),
        role,
        exp: now + config.expiry_secs,
        iat: now,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.secret.as_bytes()),
    )
}

/// Validate a JWT token and extract claims.
pub fn validate_jwt_token(
    token: &str,
    config: &JwtConfig,
) -> Result<JwtClaims, jsonwebtoken::errors::Error> {
    let token_data = decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(config.secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}

// ────────────────────────────────────────────────────────
// Middleware: API key auth
// ────────────────────────────────────────────────────────

/// Middleware that checks for API key in `X-Api-Key` header or JWT in `Authorization: Bearer`.
pub async fn api_key_auth(mut request: Request, next: Next) -> Response {
    // Try JWT Bearer token first
    if let Some(auth_header) = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
    {
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            if let Some(jwt_config) = request.extensions().get::<JwtConfig>().cloned() {
                match validate_jwt_token(token, &jwt_config) {
                    Ok(claims) => {
                        request.extensions_mut().insert(AuthContext {
                            role: claims.role,
                            identity: claims.sub,
                        });
                        return next.run(request).await;
                    }
                    Err(_) => {
                        return (
                            StatusCode::UNAUTHORIZED,
                            axum::Json(serde_json::json!({"error": "invalid JWT token"})),
                        )
                            .into_response();
                    }
                }
            }
        }
    }

    // Fall back to API key
    let api_key = request
        .headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let Some(key) = api_key else {
        return (
            StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({
                "error": "missing authentication (X-Api-Key or Authorization: Bearer)"
            })),
        )
            .into_response();
    };

    let keys = request
        .extensions()
        .get::<Vec<ApiKeyEntry>>()
        .cloned()
        .unwrap_or_default();

    let entry = keys.iter().find(|e| e.key == key);

    match entry {
        Some(entry) => {
            request.extensions_mut().insert(AuthContext {
                role: entry.role,
                identity: entry.identity.clone(),
            });
            next.run(request).await
        }
        None => (
            StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({"error": "invalid API key"})),
        )
            .into_response(),
    }
}

/// Require at least write permission.
pub async fn require_write(request: Request, next: Next) -> Response {
    if let Some(ctx) = request.extensions().get::<AuthContext>() {
        if ctx.role.can_write() {
            return next.run(request).await;
        }
        return (
            StatusCode::FORBIDDEN,
            axum::Json(serde_json::json!({"error": "insufficient permissions"})),
        )
            .into_response();
    }
    (
        StatusCode::UNAUTHORIZED,
        axum::Json(serde_json::json!({"error": "not authenticated"})),
    )
        .into_response()
}

/// Require admin role.
pub async fn require_admin(request: Request, next: Next) -> Response {
    if let Some(ctx) = request.extensions().get::<AuthContext>() {
        if ctx.role.can_admin() {
            return next.run(request).await;
        }
        return (
            StatusCode::FORBIDDEN,
            axum::Json(serde_json::json!({"error": "admin role required"})),
        )
            .into_response();
    }
    (
        StatusCode::UNAUTHORIZED,
        axum::Json(serde_json::json!({"error": "not authenticated"})),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_admin_rate_limiter() {
        let limiter = create_admin_rate_limiter(100);
        // First request should succeed
        assert!(limiter.check().is_ok());
    }

    #[test]
    fn test_admin_role_permissions() {
        assert!(AdminRole::Admin.can_write());
        assert!(AdminRole::Admin.can_admin());
        assert!(AdminRole::Operator.can_write());
        assert!(!AdminRole::Operator.can_admin());
        assert!(!AdminRole::Viewer.can_write());
        assert!(!AdminRole::Viewer.can_admin());
    }

    #[test]
    fn test_admin_role_serialization() {
        let json = serde_json::to_string(&AdminRole::Admin).unwrap();
        assert_eq!(json, "\"admin\"");
        let role: AdminRole = serde_json::from_str("\"operator\"").unwrap();
        assert_eq!(role, AdminRole::Operator);
    }

    #[test]
    fn test_jwt_create_and_validate() {
        let config = JwtConfig {
            secret: "test-secret-key".to_string(),
            expiry_secs: 3600,
        };

        let token = create_jwt_token("admin@draox.io", AdminRole::Admin, &config).unwrap();
        assert!(!token.is_empty());

        let claims = validate_jwt_token(&token, &config).unwrap();
        assert_eq!(claims.sub, "admin@draox.io");
        assert_eq!(claims.role, AdminRole::Admin);
    }

    #[test]
    fn test_jwt_invalid_token() {
        let config = JwtConfig::default();
        let result = validate_jwt_token("invalid.token.here", &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_jwt_wrong_secret() {
        let config1 = JwtConfig {
            secret: "secret-1".to_string(),
            expiry_secs: 3600,
        };
        let config2 = JwtConfig {
            secret: "secret-2".to_string(),
            expiry_secs: 3600,
        };

        let token = create_jwt_token("user", AdminRole::Viewer, &config1).unwrap();
        let result = validate_jwt_token(&token, &config2);
        assert!(result.is_err());
    }
}
