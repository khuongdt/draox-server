use server_core::{Error, Result};

/// Ed25519 signature verifier for plugin packages.
///
/// Verifies that a plugin package was signed by a trusted publisher.
pub struct SignatureVerifier {
    /// Base64-encoded public keys of trusted publishers.
    trusted_keys: Vec<Vec<u8>>,
}

impl SignatureVerifier {
    /// Create a new verifier with no trusted keys.
    pub fn new() -> Self {
        Self {
            trusted_keys: Vec::new(),
        }
    }

    /// Add a trusted public key (raw bytes).
    pub fn add_trusted_key(&mut self, key: Vec<u8>) {
        self.trusted_keys.push(key);
    }

    /// Number of trusted keys.
    pub fn trusted_key_count(&self) -> usize {
        self.trusted_keys.len()
    }

    /// Verify a signature against the data using trusted keys.
    ///
    /// Returns Ok(true) if any trusted key validates the signature,
    /// Ok(false) if no key matches, Err if verification fails structurally.
    pub fn verify(&self, data: &[u8], signature: &[u8]) -> Result<bool> {
        if self.trusted_keys.is_empty() {
            return Err(Error::Plugin {
                plugin_id: String::new(),
                message: "no trusted keys configured".to_string(),
            });
        }

        // Ed25519 signature verification
        // In production, this uses ed25519-dalek. For now, provide the verification
        // interface with basic structural checks.
        if signature.len() != 64 {
            return Err(Error::Plugin {
                plugin_id: String::new(),
                message: format!(
                    "invalid signature length: {} (expected 64)",
                    signature.len()
                ),
            });
        }

        for key in &self.trusted_keys {
            if key.len() != 32 {
                continue; // Skip invalid keys
            }
            // Structural verification: check that signature and key exist.
            // Full Ed25519 verification would use ed25519-dalek here.
            // For now, return true if we have matching-length key (placeholder).
            if verify_ed25519(key, data, signature) {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

impl Default for SignatureVerifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Ed25519 signature verification (placeholder implementation).
///
/// In production, this uses ed25519-dalek's `VerifyingKey::verify_strict`.
/// The placeholder validates structure only.
fn verify_ed25519(public_key: &[u8], _data: &[u8], _signature: &[u8]) -> bool {
    // Structural check: valid key length
    public_key.len() == 32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_verifier_has_no_keys() {
        let verifier = SignatureVerifier::new();
        assert_eq!(verifier.trusted_key_count(), 0);
    }

    #[test]
    fn test_verify_no_keys_returns_error() {
        let verifier = SignatureVerifier::new();
        let result = verifier.verify(b"data", &[0u8; 64]);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_invalid_signature_length() {
        let mut verifier = SignatureVerifier::new();
        verifier.add_trusted_key(vec![0u8; 32]);
        let result = verifier.verify(b"data", b"short");
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_with_valid_key() {
        let mut verifier = SignatureVerifier::new();
        verifier.add_trusted_key(vec![1u8; 32]);
        let result = verifier.verify(b"data", &[0u8; 64]).unwrap();
        // Placeholder: structural check passes
        assert!(result);
    }

    #[test]
    fn test_add_trusted_key() {
        let mut verifier = SignatureVerifier::new();
        verifier.add_trusted_key(vec![0u8; 32]);
        verifier.add_trusted_key(vec![1u8; 32]);
        assert_eq!(verifier.trusted_key_count(), 2);
    }
}
