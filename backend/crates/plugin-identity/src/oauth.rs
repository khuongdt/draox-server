use serde::{Deserialize, Serialize};
use crate::types::{AuthProvider, IdentityError, IdentityResult};

/// OAuth2 provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthProviderConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

/// User info returned from an OAuth2 provider after successful login.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthUserInfo {
    pub provider: AuthProvider,
    pub sub: String,
    pub email: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
}

/// Build the OAuth2 authorization URL for the given provider.
pub fn authorization_url(provider: &AuthProvider, config: &OAuthProviderConfig) -> IdentityResult<String> {
    let (auth_url, scope) = match provider {
        AuthProvider::Google => (
            "https://accounts.google.com/o/oauth2/v2/auth",
            "openid email profile",
        ),
        AuthProvider::Discord => (
            "https://discord.com/api/oauth2/authorize",
            "identify email",
        ),
        AuthProvider::Apple => (
            "https://appleid.apple.com/auth/authorize",
            "name email",
        ),
        _ => return Err(IdentityError::Internal("unsupported OAuth provider".to_string())),
    };

    let state = uuid::Uuid::new_v4().to_string();
    let url = format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
        auth_url,
        urlencoding::encode(&config.client_id),
        urlencoding::encode(&config.redirect_uri),
        urlencoding::encode(scope),
        state
    );
    Ok(url)
}

/// Exchange an OAuth2 authorization code for user info.
/// In production this calls the provider's token endpoint and userinfo endpoint.
pub async fn exchange_code(
    provider: &AuthProvider,
    config: &OAuthProviderConfig,
    code: &str,
) -> IdentityResult<OAuthUserInfo> {
    let client = reqwest::Client::new();

    let (token_url, userinfo_url) = match provider {
        AuthProvider::Google => (
            "https://oauth2.googleapis.com/token",
            "https://openidconnect.googleapis.com/v1/userinfo",
        ),
        AuthProvider::Discord => (
            "https://discord.com/api/oauth2/token",
            "https://discord.com/api/users/@me",
        ),
        _ => return Err(IdentityError::Internal("unsupported OAuth provider".to_string())),
    };

    // Exchange code for access token
    let token_resp: serde_json::Value = client
        .post(token_url)
        .form(&[
            ("client_id", config.client_id.as_str()),
            ("client_secret", config.client_secret.as_str()),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", config.redirect_uri.as_str()),
        ])
        .send()
        .await
        .map_err(|e| IdentityError::Internal(e.to_string()))?
        .json()
        .await
        .map_err(|e| IdentityError::Internal(e.to_string()))?;

    let access_token = token_resp["access_token"]
        .as_str()
        .ok_or_else(|| IdentityError::Internal("missing access_token".to_string()))?;

    // Fetch user info
    let user_resp: serde_json::Value = client
        .get(userinfo_url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| IdentityError::Internal(e.to_string()))?
        .json()
        .await
        .map_err(|e| IdentityError::Internal(e.to_string()))?;

    let sub = user_resp["sub"]
        .as_str()
        .or_else(|| user_resp["id"].as_str())
        .unwrap_or_default()
        .to_string();
    let email = user_resp["email"].as_str().unwrap_or_default().to_string();
    let display_name = user_resp["name"]
        .as_str()
        .or_else(|| user_resp["username"].as_str())
        .unwrap_or(&email)
        .to_string();
    let avatar_url = user_resp["picture"]
        .as_str()
        .or_else(|| user_resp["avatar"].as_str())
        .map(|s| s.to_string());

    Ok(OAuthUserInfo {
        provider: provider.clone(),
        sub,
        email,
        display_name,
        avatar_url,
    })
}

// Required for URL encoding in the authorization URL builder
mod urlencoding {
    pub fn encode(s: &str) -> String {
        s.chars()
            .flat_map(|c| {
                if c.is_alphanumeric() || "-_.~".contains(c) {
                    vec![c]
                } else {
                    let b = c as u32;
                    format!("%{b:02X}").chars().collect()
                }
            })
            .collect()
    }
}
