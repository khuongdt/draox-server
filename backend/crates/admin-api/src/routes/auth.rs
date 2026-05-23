use crate::auth::{create_jwt_token, validate_jwt_token, AdminRole};
use crate::response::ApiResponse;
use crate::state::AppState;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub username: String,
    pub role: AdminRole,
}

#[derive(Serialize)]
pub struct MeResponse {
    pub username: String,
    pub role: AdminRole,
}

type ErrResp = (StatusCode, Json<serde_json::Value>);

pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, ErrResp> {
    // Dev bypass: admin/draox works without DB in development or debug builds
    if is_dev_env() && body.username == "admin" && body.password == "draox" {
        let token = create_jwt_token("admin", AdminRole::Admin, &state.jwt_config)
            .map_err(|_| internal_error())?;
        return Ok(Json(ApiResponse::ok(LoginResponse {
            token,
            username: "admin".to_string(),
            role: AdminRole::Admin,
        })));
    }

    let user = state
        .auth_store
        .get(&body.username)
        .await
        .ok_or_else(unauthorized)?;

    let parsed = PasswordHash::new(&user.password_hash).map_err(|_| internal_error())?;
    Argon2::default()
        .verify_password(body.password.as_bytes(), &parsed)
        .map_err(|_| unauthorized())?;

    let token = create_jwt_token(&user.username, user.role, &state.jwt_config)
        .map_err(|_| internal_error())?;

    Ok(Json(ApiResponse::ok(LoginResponse {
        token,
        username: user.username,
        role: user.role,
    })))
}

/// GET /api/auth/me — validate the bearer JWT and return the caller's identity + role.
pub async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(str::to_owned);

    let Some(token) = token else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "success": false, "message": "missing Authorization header" })),
        )
            .into_response();
    };

    match validate_jwt_token(&token, &state.jwt_config) {
        Ok(claims) => ApiResponse::ok(MeResponse {
            username: claims.sub,
            role: claims.role,
        })
        .into_response(),
        Err(_) => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "success": false, "message": "invalid or expired token" })),
        )
            .into_response(),
    }
}

fn is_dev_env() -> bool {
    cfg!(debug_assertions)
        || std::env::var("DRAOX_ENV")
            .map(|v| v == "development")
            .unwrap_or(false)
}

fn unauthorized() -> ErrResp {
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({"error": "invalid username or password"})),
    )
}

fn internal_error() -> ErrResp {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({"error": "internal server error"})),
    )
}
