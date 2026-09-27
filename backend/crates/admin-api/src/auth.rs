use crate::state::AppState;
use axum::extract::{ConnectInfo, Request, State};
use axum::http::{Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use governor::{DefaultKeyedRateLimiter, Quota, RateLimiter};
use ipnet::IpNet;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use plugin_sdk::Identity;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::num::NonZeroU32;
use std::sync::Arc;

// ────────────────────────────────────────────────────────
// Rate Limiting
// ────────────────────────────────────────────────────────

/// Keyed limiters are pruned once they track this many client IPs.
const RATE_LIMIT_PRUNE_THRESHOLD: usize = 10_000;

/// Per-client-IP rate limiters for the admin API.
pub struct AdminRateLimits {
    general: DefaultKeyedRateLimiter<IpAddr>,
    login: DefaultKeyedRateLimiter<IpAddr>,
    trusted_proxies: Vec<IpNet>,
}

impl AdminRateLimits {
    pub fn new(requests_per_sec: u32, login_per_min: u32) -> Self {
        let rps = NonZeroU32::new(requests_per_sec).unwrap_or(NonZeroU32::MIN);
        let login = NonZeroU32::new(login_per_min).unwrap_or(NonZeroU32::MIN);
        Self {
            general: RateLimiter::keyed(Quota::per_second(rps)),
            login: RateLimiter::keyed(Quota::per_minute(login)),
            trusted_proxies: Vec::new(),
        }
    }

    /// Trust `X-Forwarded-For` from these peers (IPs or CIDRs; invalid entries are
    /// skipped — config validation rejects them earlier).
    pub fn with_trusted_proxies(mut self, proxies: &[String]) -> Self {
        self.trusted_proxies = proxies
            .iter()
            .filter_map(|p| {
                p.parse::<IpNet>()
                    .or_else(|_| p.parse::<IpAddr>().map(IpNet::from))
                    .ok()
            })
            .collect();
        self
    }

    fn is_trusted(&self, ip: &IpAddr) -> bool {
        self.trusted_proxies.iter().any(|net| net.contains(ip))
    }

    /// Client IP for rate limiting. `X-Forwarded-For` is honored only when the socket
    /// peer is a trusted proxy; otherwise it is attacker-controlled. The chain is walked
    /// right-to-left, skipping trusted hops, so a client cannot spoof the leftmost entry.
    /// `ConnectInfo` is absent in in-process tests (`oneshot`), which share one bucket.
    fn client_ip(&self, request: &Request) -> IpAddr {
        let peer = request
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ConnectInfo(addr)| addr.ip())
            .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
        if !self.is_trusted(&peer) {
            return peer;
        }
        let forwarded = request
            .headers()
            .get_all("x-forwarded-for")
            .iter()
            .filter_map(|v| v.to_str().ok())
            .flat_map(|v| v.split(','))
            .filter_map(|s| s.trim().parse::<IpAddr>().ok())
            .collect::<Vec<_>>();
        forwarded
            .iter()
            .rev()
            .find(|ip| !self.is_trusted(ip))
            .copied()
            .unwrap_or(peer)
    }
}

fn check_limit(limiter: &DefaultKeyedRateLimiter<IpAddr>, ip: &IpAddr) -> bool {
    if limiter.len() > RATE_LIMIT_PRUNE_THRESHOLD {
        limiter.retain_recent();
    }
    limiter.check_key(ip).is_ok()
}

// K.D 2026-09-27 P0 Keyed per-IP limiter replaces the unused global limiter; login gets a
// much tighter budget so the password endpoint cannot be brute-forced.
/// Middleware that rate-limits admin API requests per client IP.
pub async fn rate_limit_middleware(
    State(limits): State<Arc<AdminRateLimits>>,
    request: Request,
    next: Next,
) -> Response {
    let ip = limits.client_ip(&request);
    let is_login = request.method() == Method::POST && request.uri().path() == "/api/auth/login";

    let allowed = check_limit(&limits.general, &ip) && (!is_login || check_limit(&limits.login, &ip));
    if allowed {
        next.run(request).await
    } else {
        tracing::warn!(client_ip = %ip, path = %request.uri().path(), "admin API rate limit exceeded");
        error_response(StatusCode::TOO_MANY_REQUESTS, "Too Many Requests", "rate limit exceeded")
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

    fn satisfies(&self, required: AdminRole) -> bool {
        match required {
            AdminRole::Viewer => true,
            AdminRole::Operator => self.can_write(),
            AdminRole::Admin => self.can_admin(),
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "admin" => Some(AdminRole::Admin),
            "operator" => Some(AdminRole::Operator),
            "viewer" => Some(AdminRole::Viewer),
            _ => None,
        }
    }
}

/// Auth context inserted into request extensions by [`admin_auth`].
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

impl ApiKeyEntry {
    /// Parse `admin_api.api_keys` entries. Format: `"<role>:<key>"` or a bare `"<key>"`
    /// (treated as admin). Blank entries are skipped.
    pub fn parse_all(raw: &[String]) -> Vec<ApiKeyEntry> {
        raw.iter()
            .enumerate()
            .filter_map(|(i, entry)| {
                let entry = entry.trim();
                let (role, key) = match entry.split_once(':') {
                    Some((prefix, rest)) => match AdminRole::parse(prefix) {
                        Some(role) => (role, rest),
                        None => (AdminRole::Admin, entry),
                    },
                    None => (AdminRole::Admin, entry),
                };
                (!key.is_empty()).then(|| ApiKeyEntry {
                    key: key.to_string(),
                    role,
                    identity: format!("api-key#{i}"),
                })
            })
            .collect()
    }
}

/// Constant-time comparison so API-key checks do not leak a matching prefix via timing.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Routes reachable without credentials. Everything else is denied by default.
fn is_public(method: &Method, path: &str) -> bool {
    matches!(
        (method.as_str(), path),
        ("POST", "/api/auth/login") | ("GET" | "HEAD", "/api/health")
    )
}

/// Minimum role for a built-in admin route: reads need viewer, mutations need operator,
/// and user/config/billing management needs admin. New routes inherit these defaults.
pub fn required_role(method: &Method, path: &str) -> AdminRole {
    const ADMIN_PREFIXES: [&str; 3] = ["/api/users", "/api/config", "/api/billing"];
    if ADMIN_PREFIXES
        .iter()
        .any(|p| path == *p || path.starts_with(&format!("{p}/")))
    {
        return AdminRole::Admin;
    }
    if matches!(*method, Method::GET | Method::HEAD) {
        AdminRole::Viewer
    } else {
        AdminRole::Operator
    }
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

fn error_response(status: StatusCode, error: &str, message: &str) -> Response {
    (
        status,
        axum::Json(serde_json::json!({
            "success": false,
            "error":   error,
            "message": message,
        })),
    )
        .into_response()
}

fn unauthorized(message: &str) -> Response {
    error_response(StatusCode::UNAUTHORIZED, "Unauthorized", message)
}

// ────────────────────────────────────────────────────────
// Middleware: admin authentication + authorization
// ────────────────────────────────────────────────────────

/// Bearer token from the `Authorization` header, or — for `/ws/*` only — the `token`
/// query parameter, because browsers cannot set headers on WebSocket upgrades.
fn bearer_token(request: &Request) -> Option<String> {
    if let Some(token) = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
    {
        return Some(token.trim().to_string());
    }
    if !request.uri().path().starts_with("/ws/") {
        return None;
    }
    request.uri().query()?.split('&').find_map(|pair| {
        pair.strip_prefix("token=")
            .filter(|t| !t.is_empty())
            .map(str::to_string)
    })
}

/// Credentials presented by the caller, extracted up front so no `&Request` (whose body
/// is not `Sync`) is held across an `.await`.
enum Credentials {
    Bearer(String),
    ApiKey(String),
    None,
}

fn credentials(request: &Request) -> Credentials {
    if let Some(token) = bearer_token(request) {
        return Credentials::Bearer(token);
    }
    match request.headers().get("x-api-key").and_then(|v| v.to_str().ok()) {
        Some(key) => Credentials::ApiKey(key.to_string()),
        None => Credentials::None,
    }
}

async fn authenticate(
    state: &AppState,
    credentials: Credentials,
) -> Result<AuthContext, Box<Response>> {
    if let Credentials::Bearer(token) = credentials {
        let claims = validate_jwt_token(&token, &state.jwt_config)
            .map_err(|_| Box::new(unauthorized("invalid or expired token")))?;
        // Re-read the account so a ban, deletion or role change takes effect immediately
        // instead of when the token expires.
        let user = state
            .auth_store
            .get(&claims.sub)
            .await
            .ok_or_else(|| Box::new(unauthorized("account no longer exists")))?;
        if user.banned {
            return Err(Box::new(unauthorized("account is banned")));
        }
        return Ok(AuthContext {
            role: user.role,
            identity: user.username,
        });
    }

    if let Credentials::ApiKey(key) = credentials {
        let raw_keys = state
            .config
            .read()
            .map(|c| c.admin_api.api_keys.clone())
            .unwrap_or_default();
        return ApiKeyEntry::parse_all(&raw_keys)
            .into_iter()
            .find(|e| constant_time_eq(e.key.as_bytes(), key.as_bytes()))
            .map(|e| AuthContext {
                role: e.role,
                identity: e.identity,
            })
            .ok_or_else(|| Box::new(unauthorized("invalid API key")));
    }

    Err(Box::new(unauthorized(
        "missing authentication (Authorization: Bearer or X-Api-Key)",
    )))
}

// K.D 2026-09-27 P0 Router-level auth for every built-in admin route (previously only the
// plugin sub-router was protected). Also inserts the AuthContext that config handlers need.
/// Authenticate the caller (JWT or API key), enforce [`required_role`], and insert
/// [`AuthContext`] plus a plugin-facing [`Identity`] into the request extensions.
pub async fn admin_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    if is_public(&method, &path) {
        return next.run(request).await;
    }

    let ctx = match authenticate(&state, credentials(&request)).await {
        Ok(ctx) => ctx,
        Err(resp) => return *resp,
    };

    let required = required_role(&method, &path);
    if !ctx.role.satisfies(required) {
        tracing::warn!(identity = %ctx.identity, role = ?ctx.role, required = ?required,
            %method, %path, "admin API access denied");
        return error_response(
            StatusCode::FORBIDDEN,
            "Forbidden",
            &format!("{required:?} role required").to_lowercase(),
        );
    }

    let identity = Identity::new(ctx.identity.clone(), format!("{:?}", ctx.role).to_lowercase());
    request.extensions_mut().insert(identity);
    request.extensions_mut().insert(ctx);
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limits_are_per_ip() {
        let limits = AdminRateLimits::new(1, 1);
        let a: IpAddr = "10.0.0.1".parse().unwrap();
        let b: IpAddr = "10.0.0.2".parse().unwrap();
        assert!(check_limit(&limits.general, &a));
        assert!(!check_limit(&limits.general, &a));
        // A different client has its own bucket.
        assert!(check_limit(&limits.general, &b));
    }

    fn request_from(peer: &str, xff: Option<&str>) -> Request {
        let mut builder = Request::builder().uri("/api/info");
        if let Some(xff) = xff {
            builder = builder.header("x-forwarded-for", xff);
        }
        let mut req = builder.body(axum::body::Body::empty()).unwrap();
        req.extensions_mut()
            .insert(ConnectInfo(format!("{peer}:1234").parse::<SocketAddr>().unwrap()));
        req
    }

    #[test]
    fn test_client_ip_ignores_xff_from_untrusted_peer() {
        let limits = AdminRateLimits::new(1, 1);
        let ip = limits.client_ip(&request_from("203.0.113.5", Some("1.1.1.1")));
        assert_eq!(ip, "203.0.113.5".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn test_client_ip_uses_xff_behind_trusted_proxy() {
        let limits = AdminRateLimits::new(1, 1).with_trusted_proxies(&["172.16.0.0/12".into()]);
        // Client spoofs a leftmost entry; nginx appends the real address on the right.
        let ip = limits.client_ip(&request_from("172.18.0.3", Some("9.9.9.9, 198.51.100.7")));
        assert_eq!(ip, "198.51.100.7".parse::<IpAddr>().unwrap());
        // No header → fall back to the proxy itself.
        let ip = limits.client_ip(&request_from("172.18.0.3", None));
        assert_eq!(ip, "172.18.0.3".parse::<IpAddr>().unwrap());
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
    fn test_required_role_policy() {
        assert_eq!(required_role(&Method::GET, "/api/connections"), AdminRole::Viewer);
        assert_eq!(required_role(&Method::GET, "/ws/events"), AdminRole::Viewer);
        assert_eq!(required_role(&Method::POST, "/api/guard/ban"), AdminRole::Operator);
        assert_eq!(required_role(&Method::DELETE, "/api/sessions/x"), AdminRole::Operator);
        assert_eq!(required_role(&Method::GET, "/api/users"), AdminRole::Admin);
        assert_eq!(required_role(&Method::POST, "/api/users/bob/ban"), AdminRole::Admin);
        assert_eq!(required_role(&Method::GET, "/api/config"), AdminRole::Admin);
        assert_eq!(required_role(&Method::PUT, "/api/billing/plan/c1"), AdminRole::Admin);
        // Prefix match must respect path segments.
        assert_eq!(required_role(&Method::GET, "/api/usersettings"), AdminRole::Viewer);
    }

    #[test]
    fn test_public_routes_are_minimal() {
        assert!(is_public(&Method::POST, "/api/auth/login"));
        assert!(is_public(&Method::GET, "/api/health"));
        assert!(is_public(&Method::HEAD, "/api/health"));
        assert!(!is_public(&Method::GET, "/api/health/detailed"));
        assert!(!is_public(&Method::GET, "/api/auth/login"));
        assert!(!is_public(&Method::GET, "/api/info"));
    }

    #[test]
    fn test_api_key_parsing() {
        let keys = ApiKeyEntry::parse_all(&[
            "operator:op-key".to_string(),
            "bare-key".to_string(),
            "unknown:with-colon".to_string(),
            "  ".to_string(),
        ]);
        assert_eq!(keys.len(), 3);
        assert_eq!((keys[0].key.as_str(), keys[0].role), ("op-key", AdminRole::Operator));
        assert_eq!((keys[1].key.as_str(), keys[1].role), ("bare-key", AdminRole::Admin));
        // An unrecognized prefix is part of the key itself.
        assert_eq!(keys[2].key, "unknown:with-colon");
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq(b"secret", b"secret"));
        assert!(!constant_time_eq(b"secret", b"secreT"));
        assert!(!constant_time_eq(b"secret", b"secret2"));
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
