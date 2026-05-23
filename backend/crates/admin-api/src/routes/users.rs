use crate::auth::{validate_jwt_token, AdminRole};
use crate::auth_store::AdminUser;
use crate::response::ApiResponse;
use crate::state::AppState;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct UserSummary {
    pub username: String,
    pub role: AdminRole,
    pub banned: bool,
}

#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub role: AdminRole,
}

#[derive(Deserialize)]
pub struct UpdateUserRequest {
    pub password: Option<String>,
    pub role: Option<AdminRole>,
}

type ErrResp = (StatusCode, Json<serde_json::Value>);

fn bad_request(msg: &str) -> ErrResp {
    (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": msg})))
}

fn not_found() -> ErrResp {
    (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "user not found"})))
}

fn internal() -> ErrResp {
    (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "internal server error"})))
}

fn hash_password(password: &str) -> Result<String, ()> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|_| ())
}

/// Extract caller identity from Authorization Bearer header if present.
fn caller_identity(headers: &HeaderMap, jwt_secret: &str) -> Option<String> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())?
        .strip_prefix("Bearer ")?;

    let cfg = crate::auth::JwtConfig {
        secret: jwt_secret.to_string(),
        expiry_secs: 0,
    };
    validate_jwt_token(token, &cfg).ok().map(|c| c.sub)
}

/// GET /api/users
pub async fn list_users(State(state): State<AppState>) -> impl IntoResponse {
    let users: Vec<UserSummary> = state
        .auth_store
        .list()
        .await
        .into_iter()
        .map(|u| UserSummary { username: u.username, role: u.role, banned: u.banned })
        .collect();

    ApiResponse::ok(users)
}

/// POST /api/users
pub async fn create_user(
    State(state): State<AppState>,
    Json(body): Json<CreateUserRequest>,
) -> Result<impl IntoResponse, ErrResp> {
    if body.username.trim().is_empty() {
        return Err(bad_request("username cannot be empty"));
    }
    if body.password.len() < 8 {
        return Err(bad_request("password must be at least 8 characters"));
    }
    if state.auth_store.exists(&body.username).await {
        return Err(bad_request("username already exists"));
    }

    let password_hash = hash_password(&body.password).map_err(|_| internal())?;
    let user = AdminUser {
        username: body.username.clone(),
        password_hash,
        role: body.role,
        banned: false,
    };
    state.auth_store.set(&user).await.map_err(|_| internal())?;

    Ok(ApiResponse::<()>::message(format!("user '{}' created", body.username)))
}

/// PUT /api/users/:username
pub async fn update_user(
    State(state): State<AppState>,
    Path(username): Path<String>,
    Json(body): Json<UpdateUserRequest>,
) -> Result<impl IntoResponse, ErrResp> {
    let mut user = state
        .auth_store
        .get(&username)
        .await
        .ok_or_else(not_found)?;

    if let Some(new_password) = body.password {
        if new_password.len() < 8 {
            return Err(bad_request("password must be at least 8 characters"));
        }
        user.password_hash = hash_password(&new_password).map_err(|_| internal())?;
    }
    if let Some(new_role) = body.role {
        user.role = new_role;
    }

    state.auth_store.set(&user).await.map_err(|_| internal())?;

    Ok(ApiResponse::<()>::message(format!("user '{}' updated", username)))
}

/// DELETE /api/users/:username
pub async fn delete_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(username): Path<String>,
) -> Result<impl IntoResponse, ErrResp> {
    // Prevent self-deletion by reading the caller's identity from JWT
    if let Some(caller) = caller_identity(&headers, &state.jwt_config.secret) {
        if caller == username {
            return Err(bad_request("cannot delete your own account"));
        }
    }

    if !state.auth_store.exists(&username).await {
        return Err(not_found());
    }

    state.auth_store.delete(&username).await.map_err(|_| internal())?;

    Ok(ApiResponse::<()>::message(format!("user '{}' deleted", username)))
}

/// POST /api/users/:username/ban
pub async fn ban_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(username): Path<String>,
) -> Result<impl IntoResponse, ErrResp> {
    if let Some(caller) = caller_identity(&headers, &state.jwt_config.secret) {
        if caller == username {
            return Err(bad_request("cannot ban your own account"));
        }
    }

    let mut user = state.auth_store.get(&username).await.ok_or_else(not_found)?;
    user.banned = true;
    state.auth_store.set(&user).await.map_err(|_| internal())?;

    Ok(ApiResponse::<()>::message(format!("user '{}' banned", username)))
}

/// POST /api/users/:username/unban
pub async fn unban_user(
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> Result<impl IntoResponse, ErrResp> {
    let mut user = state.auth_store.get(&username).await.ok_or_else(not_found)?;
    user.banned = false;
    state.auth_store.set(&user).await.map_err(|_| internal())?;

    Ok(ApiResponse::<()>::message(format!("user '{}' unbanned", username)))
}
