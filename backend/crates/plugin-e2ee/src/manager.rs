use std::sync::Arc;
use dashmap::DashMap;
use thiserror::Error;
use tracing::{debug, warn};
use x25519_dalek::PublicKey;
use crate::keypair::{EphemeralKeyPair, IdentityKeyPair};
use crate::session::{E2EESession, EncryptedMessage, HandshakeBundle};
use crate::ratchet::RatchetError;

#[derive(Debug, Error)]
pub enum E2EEError {
    #[error("no active session with peer {0}")]
    NoSession(String),
    #[error("invalid handshake bundle")]
    InvalidHandshake,
    #[error("ratchet error: {0}")]
    Ratchet(#[from] RatchetError),
    #[error("public key decode error")]
    KeyDecode,
}

/// Per-client E2EE manager.
///
/// Each client holds one `E2EEManager`. Sessions are keyed by `peer_id`.
/// `prekey` is the per-manager ephemeral used in the published bundle — its
/// private key is retained so `accept_session` can complete the DH.
pub struct E2EEManager {
    identity: IdentityKeyPair,
    /// Prekey ephemeral: published in `my_bundle`, used in `accept_session`.
    prekey: EphemeralKeyPair,
    sessions: DashMap<String, E2EESession>,
}

impl E2EEManager {
    pub fn new() -> Self {
        Self {
            identity: IdentityKeyPair::generate(),
            prekey: EphemeralKeyPair::generate(),
            sessions: DashMap::new(),
        }
    }

    /// Return this client's public handshake bundle to publish to the server.
    pub fn my_bundle(&self) -> HandshakeBundle {
        HandshakeBundle {
            identity_key: *self.identity.public.as_bytes(),
            ephemeral_key: *self.prekey.public.as_bytes(),
        }
    }

    /// Initiate a session with `peer_id` using their published [`HandshakeBundle`].
    pub fn initiate_session(
        &self,
        peer_id: impl Into<String>,
        peer_bundle: &HandshakeBundle,
    ) -> Result<HandshakeBundle, E2EEError> {
        let peer_id = peer_id.into();
        let peer_ident = PublicKey::from(peer_bundle.identity_key);
        let peer_eph = PublicKey::from(peer_bundle.ephemeral_key);
        let my_eph = EphemeralKeyPair::generate();
        let my_eph_pub = *my_eph.public.as_bytes();
        let session = E2EESession::initiate(
            &peer_id,
            &self.identity,
            &my_eph,
            &peer_ident,
            &peer_eph,
        );
        debug!(peer = %peer_id, "E2EE session initiated");
        self.sessions.insert(peer_id, session);
        Ok(HandshakeBundle {
            identity_key: *self.identity.public.as_bytes(),
            ephemeral_key: my_eph_pub,
        })
    }

    /// Accept a session initiated by a remote peer.
    pub fn accept_session(
        &self,
        peer_id: impl Into<String>,
        initiator_bundle: &HandshakeBundle,
    ) -> Result<(), E2EEError> {
        let peer_id = peer_id.into();
        let init_ident = PublicKey::from(initiator_bundle.identity_key);
        let init_eph = PublicKey::from(initiator_bundle.ephemeral_key);
        // Use stored prekey (whose public part was included in my_bundle).
        let session = E2EESession::respond(
            &peer_id,
            &self.identity,
            &self.prekey,
            &init_ident,
            &init_eph,
        );
        debug!(peer = %peer_id, "E2EE session accepted");
        self.sessions.insert(peer_id, session);
        Ok(())
    }

    /// Encrypt a plaintext message for `peer_id`.
    pub fn encrypt(
        &self,
        peer_id: &str,
        plaintext: &[u8],
    ) -> Result<EncryptedMessage, E2EEError> {
        let mut session = self
            .sessions
            .get_mut(peer_id)
            .ok_or_else(|| E2EEError::NoSession(peer_id.to_string()))?;
        Ok(session.encrypt(plaintext)?)
    }

    /// Decrypt a message received from `peer_id`.
    pub fn decrypt(
        &self,
        peer_id: &str,
        msg: &EncryptedMessage,
    ) -> Result<Vec<u8>, E2EEError> {
        let mut session = self
            .sessions
            .get_mut(peer_id)
            .ok_or_else(|| E2EEError::NoSession(peer_id.to_string()))?;
        Ok(session.decrypt(msg)?)
    }

    pub fn has_session(&self, peer_id: &str) -> bool {
        self.sessions.contains_key(peer_id)
    }

    pub fn remove_session(&self, peer_id: &str) {
        self.sessions.remove(peer_id);
        warn!(peer = %peer_id, "E2EE session removed");
    }

    /// Return own identity public key bytes.
    pub fn identity_public_key(&self) -> [u8; 32] {
        *self.identity.public.as_bytes()
    }
}

impl Default for E2EEManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_session_roundtrip() {
        let alice = Arc::new(E2EEManager::new());
        let bob = Arc::new(E2EEManager::new());

        // Bob publishes his bundle; Alice initiates
        let bob_bundle = bob.my_bundle();
        let alice_bundle = alice.initiate_session("bob", &bob_bundle).unwrap();

        // Bob accepts Alice's bundle
        bob.accept_session("alice", &alice_bundle).unwrap();

        // Alice sends an encrypted message
        let plaintext = b"hello from alice";
        let enc = alice.encrypt("bob", plaintext).unwrap();

        // Bob decrypts
        let dec = bob.decrypt("alice", &enc).unwrap();
        assert_eq!(dec, plaintext);
    }

    #[test]
    fn test_no_session_error() {
        let alice = E2EEManager::new();
        let err = alice.encrypt("nobody", b"test").unwrap_err();
        assert!(matches!(err, E2EEError::NoSession(_)));
    }
}
