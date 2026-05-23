use totp_rs::{Algorithm, Secret, TOTP};
use crate::types::{IdentityError, IdentityResult};

pub struct TotpService;

impl TotpService {
    /// Generate a new TOTP secret (base32-encoded).
    pub fn generate_secret() -> String {
        let secret = Secret::generate_secret();
        secret.to_encoded().to_string()
    }

    /// Generate the TOTP provisioning URI for QR code display.
    pub fn provisioning_uri(secret: &str, account: &str, issuer: &str) -> IdentityResult<String> {
        let totp = Self::build_totp(secret, account, issuer)?;
        Ok(totp.get_url())
    }

    /// Verify a 6-digit TOTP code.
    pub fn verify_code(secret: &str, code: &str, account: &str, issuer: &str) -> IdentityResult<bool> {
        let totp = Self::build_totp(secret, account, issuer)?;
        Ok(totp.check_current(code).unwrap_or(false))
    }

    fn build_totp(secret: &str, account: &str, issuer: &str) -> IdentityResult<TOTP> {
        let secret_bytes = Secret::Encoded(secret.to_string());
        TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            30,
            secret_bytes.to_bytes().map_err(|e| IdentityError::Internal(e.to_string()))?,
            Some(issuer.to_string()),
            account.to_string(),
        )
        .map_err(|e| IdentityError::Internal(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_secret_not_empty() {
        let secret = TotpService::generate_secret();
        assert!(!secret.is_empty());
    }

    #[test]
    fn test_provisioning_uri_format() {
        let secret = TotpService::generate_secret();
        let uri = TotpService::provisioning_uri(&secret, "user@example.com", "Draox").unwrap();
        assert!(uri.starts_with("otpauth://totp/"));
    }
}
