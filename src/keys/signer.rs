use ed25519_dalek::{Signer as _, SigningKey};

use crate::crypto::{EventSignatureBytes, EventSigner, SignerId};
use crate::error::Result;

/// The first production `EventSigner`: an Ed25519 signing key loaded from the
/// user-level keystore, paired with its derived `did:key` identity.
///
/// Signing is infallible. The only fallible work — reading and decoding the key
/// file — happens at load time, before this signer exists; once constructed,
/// `sign_event_message` cannot fail. That is load-bearing for the write path:
/// resolution (which can fail) is kept entirely ahead of signing (which cannot),
/// so signing never gates a write.
// `pub`: the binary CLI crate receives this concrete signer from the resolver and
// threads it into `.sign_with(...)`.
pub struct FileEd25519Signer {
    signer_id: SignerId,
    signing_key: SigningKey,
}

impl FileEd25519Signer {
    /// Build a signer from a raw 32-byte Ed25519 seed, deriving the `did:key`
    /// identity from the public key. `pub(crate)`: a library-internal constructor
    /// used by the loaders and tests; the CLI obtains signers via `load_signer*`.
    pub(crate) fn from_seed(seed: [u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(&seed);
        let signer_id = SignerId::from_ed25519_public_key(signing_key.verifying_key().to_bytes());
        Self {
            signer_id,
            signing_key,
        }
    }
}

impl EventSigner for FileEd25519Signer {
    fn signer_id(&self) -> &SignerId {
        &self.signer_id
    }

    fn sign_event_message(&self, message: &[u8]) -> Result<EventSignatureBytes> {
        // Ed25519 signing over a loaded key is infallible; the `Result` is the
        // trait's shape, not a failure surface here.
        let signature = self.signing_key.sign(message);
        Ok(EventSignatureBytes::from_bytes(&signature.to_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{EventVerificationStatus, verify_ed25519_strict};
    use crate::session::event::{EVENT_TO_BE_SIGNED_V1_PAYLOAD_TYPE, pre_authentication_encoding};

    #[test]
    fn file_signer_signs_dsse_pae_bytes_that_verify_valid() {
        // A loaded signer built directly from a known seed (no disk needed for
        // this unit; load-from-disk is covered by the store round-trip test).
        let signer = FileEd25519Signer::from_seed([9_u8; 32]);

        // Build real event-to-be-signed PAE bytes for the v1 payload type.
        let message = pre_authentication_encoding(
            EVENT_TO_BE_SIGNED_V1_PAYLOAD_TYPE,
            br#"{"schema":"shore.event","version":1}"#,
        );

        let sig = signer.sign_event_message(&message).unwrap();

        assert!(sig.is_base64());
        assert_eq!(
            verify_ed25519_strict(signer.signer_id(), &message, sig.as_str()).unwrap(),
            EventVerificationStatus::Valid
        );
    }

    #[test]
    fn file_signer_signer_id_is_the_derived_did_key() {
        let signer = FileEd25519Signer::from_seed([9_u8; 32]);
        let expected = crate::crypto::SignerId::from_ed25519_public_key(
            ed25519_dalek::SigningKey::from_bytes(&[9_u8; 32])
                .verifying_key()
                .to_bytes(),
        );
        assert_eq!(signer.signer_id(), &expected);
    }

    #[test]
    fn signing_the_same_message_twice_is_stable() {
        // Ed25519 over a loaded key is deterministic and infallible: same bytes in,
        // same signature out, no error path.
        let signer = FileEd25519Signer::from_seed([3_u8; 32]);
        let message = b"DSSEv1 4 test 5 hello";
        let a = signer.sign_event_message(message).unwrap();
        let b = signer.sign_event_message(message).unwrap();
        assert_eq!(a, b);
    }
}
