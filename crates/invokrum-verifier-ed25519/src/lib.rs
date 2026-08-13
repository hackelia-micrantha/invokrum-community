//! Concrete Ed25519 publisher-signature verification for immutable bundle subjects.
//!
//! This outer adapter performs only bounded cryptographic verification and
//! normalization into the provider-neutral distribution-domain assertion. It
//! performs no filesystem, network, clock, environment, credential, trust-policy,
//! installation, or composition behavior.

#![forbid(unsafe_code)]

use ed25519_dalek::{Signature, VerifyingKey};
use invokrum_digest::sha256_lower_hex;
use invokrum_distribution::{
    DistributionError, PublisherAssertion, PublisherIdentity, Sha256Digest, VerificationMechanism,
};
use std::fmt;

pub const VERIFICATION_MECHANISM: &str = "ed25519-subject-v1";
pub const IDENTITY_KEY_SHA256: &str = "key.sha256";
pub const PUBLIC_KEY_BYTES: usize = 32;
pub const SIGNATURE_BYTES: usize = 64;

const MESSAGE_DOMAIN: &str = "invokrum.publisher-signature/v1";

/// Returns the exact versioned bytes covered by an Ed25519 publisher signature.
///
/// The message is deliberately domain-separated from signatures used by other
/// protocols and binds only the already-derived immutable bundle subject:
///
/// ```text
/// invokrum.publisher-signature/v1\n
/// sha256:<64 lowercase hexadecimal subject>\n
/// ```
#[must_use]
pub fn subject_message(subject: &Sha256Digest) -> String {
    format!("{MESSAGE_DOMAIN}\nsha256:{}\n", subject.as_str())
}

/// Stateless verifier for the `ed25519-subject-v1` mechanism.
#[derive(Clone, Copy, Debug, Default)]
pub struct Ed25519SubjectVerifier;

impl Ed25519SubjectVerifier {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Verifies a raw Ed25519 signature over the exact immutable bundle subject.
    ///
    /// # Errors
    ///
    /// Fails closed for malformed key/signature lengths, invalid or weak public
    /// keys, cryptographic verification failure, or an internal normalization
    /// invariant failure. A [`PublisherAssertion`] is returned only after strict
    /// cryptographic verification succeeds.
    pub fn verify(
        self,
        subject: &Sha256Digest,
        public_key: &[u8],
        signature: &[u8],
    ) -> Result<PublisherAssertion, VerificationError> {
        let public_key: &[u8; PUBLIC_KEY_BYTES] = public_key
            .try_into()
            .map_err(|_| VerificationError::InvalidPublicKeyLength)?;
        let verifying_key = VerifyingKey::from_bytes(public_key)
            .map_err(|_| VerificationError::InvalidPublicKey)?;
        if verifying_key.is_weak() {
            return Err(VerificationError::WeakPublicKey);
        }

        let signature = Signature::from_slice(signature)
            .map_err(|_| VerificationError::InvalidSignatureLength)?;
        verifying_key
            .verify_strict(subject_message(subject).as_bytes(), &signature)
            .map_err(|_| VerificationError::SignatureVerificationFailed)?;

        let identity = PublisherIdentity::new(vec![(
            IDENTITY_KEY_SHA256.to_owned(),
            sha256_lower_hex(public_key),
        )])
        .map_err(VerificationError::NormalizationInvariant)?;
        let mechanism = VerificationMechanism::parse(VERIFICATION_MECHANISM)
            .map_err(VerificationError::NormalizationInvariant)?;

        Ok(PublisherAssertion::new(
            mechanism,
            subject.clone(),
            identity,
        ))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerificationError {
    InvalidPublicKeyLength,
    InvalidPublicKey,
    WeakPublicKey,
    InvalidSignatureLength,
    SignatureVerificationFailed,
    NormalizationInvariant(DistributionError),
}

impl fmt::Display for VerificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidPublicKeyLength => "Ed25519 public key must contain exactly 32 bytes",
            Self::InvalidPublicKey => "invalid Ed25519 public key",
            Self::WeakPublicKey => "weak Ed25519 public key is not accepted",
            Self::InvalidSignatureLength => "Ed25519 signature must contain exactly 64 bytes",
            Self::SignatureVerificationFailed => "Ed25519 subject signature verification failed",
            Self::NormalizationInvariant(_) => {
                "verified publisher assertion could not be normalized"
            }
        })
    }
}

impl std::error::Error for VerificationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NormalizationInvariant(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use invokrum_distribution::{PublisherIdentity, TrustError, TrustPolicy, TrustRule};

    // RFC 8032 test-vector #1 public key. The RFC signature below verifies the
    // published empty-message vector directly against the same strict primitive
    // used by the adapter. The second signature is an Invokrum-specific golden
    // over SUBJECT_ZERO generated from the vector's published private seed.
    const PUBLIC_KEY: [u8; PUBLIC_KEY_BYTES] =
        hex32("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a");
    const RFC8032_EMPTY_SIGNATURE: [u8; SIGNATURE_BYTES] = hex64(
        "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
    );
    const SIGNATURE: [u8; SIGNATURE_BYTES] = hex64(
        "9d2736ae5a0df81b19cfe74242ccdf98808c9bac8db90f33264e7d894ac33d5c522e7b54fe621b8e1919ab6bf6a04f81ab688ac8ab17e90abd22a45abde40f08",
    );
    const FINGERPRINT: &str = "21fe31dfa154a261626bf854046fd2271b7bed4b6abe45aa58877ef47f9721b9";
    const SUBJECT_ZERO: &str = "0000000000000000000000000000000000000000000000000000000000000000";

    #[test]
    fn rfc8032_vector_one_verifies_with_strict_primitive() {
        let key = VerifyingKey::from_bytes(&PUBLIC_KEY).expect("RFC public key should parse");
        let signature = Signature::from_slice(&RFC8032_EMPTY_SIGNATURE)
            .expect("RFC signature should have the required length");
        key.verify_strict(b"", &signature)
            .expect("RFC 8032 vector #1 should verify");
    }

    #[test]
    fn subject_message_is_exact_and_domain_separated() {
        let subject = digest(SUBJECT_ZERO);
        assert_eq!(
            subject_message(&subject),
            concat!(
                "invokrum.publisher-signature/v1\n",
                "sha256:",
                "0000000000000000000000000000000000000000000000000000000000000000",
                "\n"
            )
        );
    }

    #[test]
    fn golden_signature_normalizes_provider_neutral_assertion() {
        let subject = digest(SUBJECT_ZERO);
        let assertion = Ed25519SubjectVerifier::new()
            .verify(&subject, &PUBLIC_KEY, &SIGNATURE)
            .expect("golden signature should verify");

        assert_eq!(assertion.mechanism().as_str(), VERIFICATION_MECHANISM);
        assert_eq!(assertion.subject(), &subject);
        assert_eq!(
            assertion.identity().attributes().get(IDENTITY_KEY_SHA256),
            Some(&FINGERPRINT.to_owned())
        );
    }

    #[test]
    fn malformed_lengths_fail_before_crypto() {
        let subject = digest(SUBJECT_ZERO);
        let oversized_key = [0_u8; PUBLIC_KEY_BYTES + 1];
        let oversized_signature = [0_u8; SIGNATURE_BYTES + 1];

        assert_eq!(
            Ed25519SubjectVerifier::new().verify(&subject, &PUBLIC_KEY[..31], &SIGNATURE),
            Err(VerificationError::InvalidPublicKeyLength)
        );
        assert_eq!(
            Ed25519SubjectVerifier::new().verify(&subject, &oversized_key, &SIGNATURE),
            Err(VerificationError::InvalidPublicKeyLength)
        );
        assert_eq!(
            Ed25519SubjectVerifier::new().verify(&subject, &PUBLIC_KEY, &SIGNATURE[..63]),
            Err(VerificationError::InvalidSignatureLength)
        );
        assert_eq!(
            Ed25519SubjectVerifier::new().verify(&subject, &PUBLIC_KEY, &oversized_signature),
            Err(VerificationError::InvalidSignatureLength)
        );
    }

    #[test]
    fn weak_key_fails_before_signature_verification() {
        let subject = digest(SUBJECT_ZERO);
        let mut identity_key = [0_u8; PUBLIC_KEY_BYTES];
        identity_key[0] = 1;

        assert_eq!(
            Ed25519SubjectVerifier::new().verify(&subject, &identity_key, &SIGNATURE),
            Err(VerificationError::WeakPublicKey)
        );
    }

    #[test]
    fn subject_key_and_signature_substitution_fail_closed() {
        let verifier = Ed25519SubjectVerifier::new();
        let mut different_subject = SUBJECT_ZERO.as_bytes().to_vec();
        different_subject[63] = b'1';
        let different_subject = digest(std::str::from_utf8(&different_subject).expect("ASCII"));
        assert_eq!(
            verifier.verify(&different_subject, &PUBLIC_KEY, &SIGNATURE),
            Err(VerificationError::SignatureVerificationFailed)
        );

        let mut changed_key = PUBLIC_KEY;
        changed_key[0] ^= 1;
        assert!(
            verifier
                .verify(&digest(SUBJECT_ZERO), &changed_key, &SIGNATURE)
                .is_err()
        );

        let mut changed_signature = SIGNATURE;
        changed_signature[0] ^= 1;
        assert_eq!(
            verifier.verify(&digest(SUBJECT_ZERO), &PUBLIC_KEY, &changed_signature),
            Err(VerificationError::SignatureVerificationFailed)
        );
    }

    #[test]
    fn host_authorization_is_separate_from_crypto_validity() {
        let subject = digest(SUBJECT_ZERO);
        let assertion = Ed25519SubjectVerifier::new()
            .verify(&subject, &PUBLIC_KEY, &SIGNATURE)
            .expect("golden signature should verify");

        let allowed_identity = PublisherIdentity::new(vec![(
            IDENTITY_KEY_SHA256.to_owned(),
            FINGERPRINT.to_owned(),
        )])
        .expect("identity should be valid");
        let policy = TrustPolicy::new(vec![TrustRule::new(
            VerificationMechanism::parse(VERIFICATION_MECHANISM)
                .expect("mechanism should be valid"),
            allowed_identity,
        )])
        .expect("policy should be valid");
        assert_eq!(policy.authorize(&assertion, &subject), Ok(()));

        let denied_identity = PublisherIdentity::new(vec![(
            IDENTITY_KEY_SHA256.to_owned(),
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_owned(),
        )])
        .expect("identity should be valid");
        let denied_policy = TrustPolicy::new(vec![TrustRule::new(
            VerificationMechanism::parse(VERIFICATION_MECHANISM)
                .expect("mechanism should be valid"),
            denied_identity,
        )])
        .expect("policy should be valid");
        assert_eq!(
            denied_policy.authorize(&assertion, &subject),
            Err(TrustError::PublisherNotAllowed)
        );
    }

    fn digest(value: &str) -> Sha256Digest {
        Sha256Digest::parse(value).expect("test digest should be valid")
    }

    const fn hex_nibble(byte: u8) -> u8 {
        match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            _ => panic!("invalid hex fixture"),
        }
    }

    const fn hex32(value: &str) -> [u8; 32] {
        let bytes = value.as_bytes();
        assert!(bytes.len() == 64);
        let mut output = [0_u8; 32];
        let mut index = 0;
        while index < 32 {
            output[index] = (hex_nibble(bytes[index * 2]) << 4) | hex_nibble(bytes[index * 2 + 1]);
            index += 1;
        }
        output
    }

    const fn hex64(value: &str) -> [u8; 64] {
        let bytes = value.as_bytes();
        assert!(bytes.len() == 128);
        let mut output = [0_u8; 64];
        let mut index = 0;
        while index < 64 {
            output[index] = (hex_nibble(bytes[index * 2]) << 4) | hex_nibble(bytes[index * 2 + 1]);
            index += 1;
        }
        output
    }
}
