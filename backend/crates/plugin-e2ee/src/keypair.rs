use rand::rngs::OsRng;
use x25519_dalek::{PublicKey, StaticSecret};

/// X25519 identity key pair held by each client.
/// The public key is published to the server; the private key never leaves the device.
pub struct IdentityKeyPair {
    pub(crate) secret: StaticSecret,
    pub public: PublicKey,
}

impl IdentityKeyPair {
    pub fn generate() -> Self {
        let secret = StaticSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&secret);
        Self { secret, public }
    }

    /// Derive a 32-byte shared secret with a remote public key using X25519 DH.
    pub fn dh(&self, remote_public: &PublicKey) -> [u8; 32] {
        *self.secret.diffie_hellman(remote_public).as_bytes()
    }
}

/// Ephemeral key pair used once per session initiation (forward secrecy).
pub struct EphemeralKeyPair {
    pub(crate) secret: StaticSecret,
    pub public: PublicKey,
}

impl EphemeralKeyPair {
    pub fn generate() -> Self {
        let secret = StaticSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&secret);
        Self { secret, public }
    }

    pub fn dh(&self, remote_public: &PublicKey) -> [u8; 32] {
        *self.secret.diffie_hellman(remote_public).as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dh_is_symmetric() {
        let alice = IdentityKeyPair::generate();
        let bob = IdentityKeyPair::generate();
        let shared_ab = alice.dh(&bob.public);
        let shared_ba = bob.dh(&alice.public);
        assert_eq!(shared_ab, shared_ba);
    }

    #[test]
    fn test_different_pairs_differ() {
        let a = IdentityKeyPair::generate();
        let b = IdentityKeyPair::generate();
        let c = IdentityKeyPair::generate();
        assert_ne!(a.dh(&c.public), b.dh(&c.public));
    }
}
