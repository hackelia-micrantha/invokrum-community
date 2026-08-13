//! Provider-neutral pack distribution and publisher-trust domain contracts.
//!
//! This crate owns parsing-neutral values and deterministic validation for
//! immutable pack-bundle identity and host-owned publisher policy. It performs
//! no serialization, hashing, filesystem access, network access, clock access,
//! credential lookup, or signature verification.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;

pub const BUNDLE_FORMAT: &str = "invokrum.pack-bundle/v1";
pub const SHA256_ALGORITHM: &str = "sha256";
pub const MAX_BUNDLE_FILES: usize = 512;
pub const MAX_BUNDLE_FILE_BYTES: u64 = 1_048_576;
pub const MAX_BUNDLE_EXPANDED_BYTES: u64 = 33_554_432;
pub const MAX_BUNDLE_PATH_BYTES: usize = 1_024;
pub const MAX_IDENTITY_ATTRIBUTES: usize = 32;
pub const MAX_IDENTITY_NAME_BYTES: usize = 128;
pub const MAX_IDENTITY_VALUE_BYTES: usize = 1_024;
pub const MAX_TRUST_RULES: usize = 64;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct BundlePath(String);

impl BundlePath {
    /// Parses a portable ASCII path relative to a pack root.
    ///
    /// # Errors
    ///
    /// Returns [`DistributionError::InvalidPath`] when the path is empty,
    /// oversized, non-ASCII, absolute, platform-prefixed, contains unsupported
    /// separators or control characters, or contains empty, `.` or `..`
    /// segments.
    pub fn parse(value: impl Into<String>) -> Result<Self, DistributionError> {
        let value = value.into();
        let invalid = value.is_empty()
            || value.len() > MAX_BUNDLE_PATH_BYTES
            || !value.is_ascii()
            || value.starts_with('/')
            || value.ends_with('/')
            || value.contains('\\')
            || value.contains(':')
            || value.bytes().any(|byte| byte.is_ascii_control())
            || value
                .split('/')
                .any(|segment| segment.is_empty() || matches!(segment, "." | ".."));
        if invalid {
            return Err(DistributionError::InvalidPath);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BundlePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Sha256Digest(String);

impl Sha256Digest {
    /// Parses an exact lowercase hexadecimal SHA-256 digest.
    ///
    /// # Errors
    ///
    /// Returns [`DistributionError::InvalidDigest`] unless the input contains
    /// exactly 64 lowercase hexadecimal ASCII characters.
    pub fn parse(value: impl Into<String>) -> Result<Self, DistributionError> {
        let value = value.into();
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
        {
            return Err(DistributionError::InvalidDigest);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Sha256Digest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Host-selected bundle limits, clamped to the immutable v1 format maxima.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BundleLimits {
    maximum_files: usize,
    maximum_file_bytes: u64,
    maximum_expanded_bytes: u64,
}

impl BundleLimits {
    /// Creates host limits that may tighten, but never relax, v1 hard bounds.
    #[must_use]
    pub const fn new(
        maximum_files: usize,
        maximum_file_bytes: u64,
        maximum_expanded_bytes: u64,
    ) -> Self {
        Self {
            maximum_files: if maximum_files > MAX_BUNDLE_FILES {
                MAX_BUNDLE_FILES
            } else {
                maximum_files
            },
            maximum_file_bytes: if maximum_file_bytes > MAX_BUNDLE_FILE_BYTES {
                MAX_BUNDLE_FILE_BYTES
            } else {
                maximum_file_bytes
            },
            maximum_expanded_bytes: if maximum_expanded_bytes > MAX_BUNDLE_EXPANDED_BYTES {
                MAX_BUNDLE_EXPANDED_BYTES
            } else {
                maximum_expanded_bytes
            },
        }
    }

    #[must_use]
    pub const fn maximum_files(self) -> usize {
        self.maximum_files
    }

    #[must_use]
    pub const fn maximum_file_bytes(self) -> u64 {
        self.maximum_file_bytes
    }

    #[must_use]
    pub const fn maximum_expanded_bytes(self) -> u64 {
        self.maximum_expanded_bytes
    }
}

impl Default for BundleLimits {
    fn default() -> Self {
        Self::new(
            MAX_BUNDLE_FILES,
            MAX_BUNDLE_FILE_BYTES,
            MAX_BUNDLE_EXPANDED_BYTES,
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BundleFile {
    path: BundlePath,
    byte_length: u64,
    digest: Sha256Digest,
}

impl BundleFile {
    #[must_use]
    pub const fn new(path: BundlePath, byte_length: u64, digest: Sha256Digest) -> Self {
        Self {
            path,
            byte_length,
            digest,
        }
    }

    #[must_use]
    pub const fn path(&self) -> &BundlePath {
        &self.path
    }

    #[must_use]
    pub const fn byte_length(&self) -> u64 {
        self.byte_length
    }

    #[must_use]
    pub const fn digest(&self) -> &Sha256Digest {
        &self.digest
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BundleManifest {
    entry_point: BundlePath,
    files: Vec<BundleFile>,
}

impl BundleManifest {
    /// Constructs and normalizes an immutable bundle manifest.
    ///
    /// # Errors
    ///
    /// Returns a stable [`DistributionError`] when a format, algorithm, limit,
    /// uniqueness, or entry-point invariant is violated.
    pub fn new(
        format: &str,
        digest_algorithm: &str,
        entry_point: BundlePath,
        mut files: Vec<BundleFile>,
        limits: BundleLimits,
    ) -> Result<Self, DistributionError> {
        if format != BUNDLE_FORMAT {
            return Err(DistributionError::UnsupportedBundleFormat);
        }
        if digest_algorithm != SHA256_ALGORITHM {
            return Err(DistributionError::UnsupportedDigestAlgorithm);
        }
        if files.is_empty() {
            return Err(DistributionError::EmptyBundle);
        }
        if files.len() > limits.maximum_files() {
            return Err(DistributionError::TooManyFiles);
        }

        files.sort_by(|left, right| left.path.cmp(&right.path));
        if files
            .windows(2)
            .any(|window| window[0].path == window[1].path)
        {
            return Err(DistributionError::DuplicatePath);
        }

        let mut expanded_bytes = 0_u64;
        for file in &files {
            if file.byte_length > limits.maximum_file_bytes() {
                return Err(DistributionError::FileTooLarge);
            }
            expanded_bytes = expanded_bytes
                .checked_add(file.byte_length)
                .ok_or(DistributionError::BundleTooLarge)?;
            if expanded_bytes > limits.maximum_expanded_bytes() {
                return Err(DistributionError::BundleTooLarge);
            }
        }

        if files.iter().all(|file| file.path != entry_point) {
            return Err(DistributionError::EntryPointMissing);
        }

        Ok(Self { entry_point, files })
    }

    #[must_use]
    pub const fn format(&self) -> &'static str {
        BUNDLE_FORMAT
    }

    #[must_use]
    pub const fn digest_algorithm(&self) -> &'static str {
        SHA256_ALGORITHM
    }

    #[must_use]
    pub const fn entry_point(&self) -> &BundlePath {
        &self.entry_point
    }

    #[must_use]
    pub fn files(&self) -> &[BundleFile] {
        &self.files
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct VerificationMechanism(String);

impl VerificationMechanism {
    /// Parses a normalized verification mechanism identifier.
    ///
    /// # Errors
    ///
    /// Returns [`DistributionError::InvalidVerificationMechanism`] unless the
    /// value is lowercase ASCII and uses only letters, digits, `.`, `_`, or `-`.
    pub fn parse(value: impl Into<String>) -> Result<Self, DistributionError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 128
            || !value.bytes().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'.' | b'_' | b'-')
            })
        {
            return Err(DistributionError::InvalidVerificationMechanism);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublisherIdentity {
    attributes: BTreeMap<String, String>,
}

impl PublisherIdentity {
    /// Creates a normalized publisher identity from explicit attributes.
    ///
    /// # Errors
    ///
    /// Returns a stable [`DistributionError`] for empty/oversized identities,
    /// duplicate names, invalid normalized names, or invalid values.
    pub fn new(attributes: Vec<(String, String)>) -> Result<Self, DistributionError> {
        if attributes.is_empty() {
            return Err(DistributionError::EmptyPublisherIdentity);
        }
        if attributes.len() > MAX_IDENTITY_ATTRIBUTES {
            return Err(DistributionError::TooManyIdentityAttributes);
        }

        let mut normalized = BTreeMap::new();
        for (name, value) in attributes {
            if !valid_attribute_name(&name) {
                return Err(DistributionError::InvalidIdentityAttributeName);
            }
            if value.is_empty()
                || value.len() > MAX_IDENTITY_VALUE_BYTES
                || value.chars().any(char::is_control)
            {
                return Err(DistributionError::InvalidIdentityAttributeValue);
            }
            if normalized.insert(name, value).is_some() {
                return Err(DistributionError::DuplicateIdentityAttribute);
            }
        }

        Ok(Self {
            attributes: normalized,
        })
    }

    #[must_use]
    pub const fn attributes(&self) -> &BTreeMap<String, String> {
        &self.attributes
    }

    fn contains_all(&self, required: &Self) -> bool {
        required
            .attributes
            .iter()
            .all(|(name, value)| self.attributes.get(name) == Some(value))
    }
}

fn valid_attribute_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTITY_NAME_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublisherAssertion {
    mechanism: VerificationMechanism,
    subject: Sha256Digest,
    identity: PublisherIdentity,
}

impl PublisherAssertion {
    #[must_use]
    pub const fn new(
        mechanism: VerificationMechanism,
        subject: Sha256Digest,
        identity: PublisherIdentity,
    ) -> Self {
        Self {
            mechanism,
            subject,
            identity,
        }
    }

    #[must_use]
    pub const fn mechanism(&self) -> &VerificationMechanism {
        &self.mechanism
    }

    #[must_use]
    pub const fn subject(&self) -> &Sha256Digest {
        &self.subject
    }

    #[must_use]
    pub const fn identity(&self) -> &PublisherIdentity {
        &self.identity
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrustRule {
    mechanism: VerificationMechanism,
    required_identity: PublisherIdentity,
}

impl TrustRule {
    #[must_use]
    pub const fn new(
        mechanism: VerificationMechanism,
        required_identity: PublisherIdentity,
    ) -> Self {
        Self {
            mechanism,
            required_identity,
        }
    }

    fn matches(&self, assertion: &PublisherAssertion) -> bool {
        self.mechanism == assertion.mechanism
            && assertion.identity.contains_all(&self.required_identity)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrustPolicy {
    rules: Vec<TrustRule>,
}

impl TrustPolicy {
    /// Constructs explicit host-owned publisher policy.
    ///
    /// # Errors
    ///
    /// Returns [`DistributionError::EmptyTrustPolicy`] or
    /// [`DistributionError::TooManyTrustRules`] for invalid rule counts.
    pub fn new(rules: Vec<TrustRule>) -> Result<Self, DistributionError> {
        if rules.is_empty() {
            return Err(DistributionError::EmptyTrustPolicy);
        }
        if rules.len() > MAX_TRUST_RULES {
            return Err(DistributionError::TooManyTrustRules);
        }
        Ok(Self { rules })
    }

    /// Authorizes one externally verified assertion for an expected immutable
    /// bundle subject.
    ///
    /// # Errors
    ///
    /// Fails closed when the signed subject is not the expected bundle subject
    /// or no explicit host rule matches the verifier mechanism and identity.
    pub fn authorize(
        &self,
        assertion: &PublisherAssertion,
        expected_subject: &Sha256Digest,
    ) -> Result<(), TrustError> {
        if assertion.subject != *expected_subject {
            return Err(TrustError::SubjectMismatch);
        }
        if self.rules.iter().any(|rule| rule.matches(assertion)) {
            return Ok(());
        }
        Err(TrustError::PublisherNotAllowed)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrustError {
    SubjectMismatch,
    PublisherNotAllowed,
}

impl fmt::Display for TrustError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::SubjectMismatch => "signed subject does not match expected bundle subject",
            Self::PublisherNotAllowed => "publisher identity is not allowed by host trust policy",
        })
    }
}

impl std::error::Error for TrustError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DistributionError {
    InvalidPath,
    InvalidDigest,
    UnsupportedBundleFormat,
    UnsupportedDigestAlgorithm,
    EmptyBundle,
    TooManyFiles,
    DuplicatePath,
    FileTooLarge,
    BundleTooLarge,
    EntryPointMissing,
    InvalidVerificationMechanism,
    EmptyPublisherIdentity,
    TooManyIdentityAttributes,
    InvalidIdentityAttributeName,
    InvalidIdentityAttributeValue,
    DuplicateIdentityAttribute,
    EmptyTrustPolicy,
    TooManyTrustRules,
}

impl fmt::Display for DistributionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidPath => "invalid bundle-relative path",
            Self::InvalidDigest => "invalid SHA-256 digest",
            Self::UnsupportedBundleFormat => "unsupported bundle format",
            Self::UnsupportedDigestAlgorithm => "unsupported bundle digest algorithm",
            Self::EmptyBundle => "bundle must contain at least one file",
            Self::TooManyFiles => "bundle contains too many files",
            Self::DuplicatePath => "bundle contains a duplicate file path",
            Self::FileTooLarge => "bundle file exceeds configured size limit",
            Self::BundleTooLarge => "bundle expanded size exceeds configured limit",
            Self::EntryPointMissing => "bundle entry point is not an enumerated file",
            Self::InvalidVerificationMechanism => "invalid verification mechanism",
            Self::EmptyPublisherIdentity => "publisher identity must contain attributes",
            Self::TooManyIdentityAttributes => "publisher identity has too many attributes",
            Self::InvalidIdentityAttributeName => "invalid publisher identity attribute name",
            Self::InvalidIdentityAttributeValue => "invalid publisher identity attribute value",
            Self::DuplicateIdentityAttribute => "duplicate publisher identity attribute",
            Self::EmptyTrustPolicy => "host trust policy must contain at least one rule",
            Self::TooManyTrustRules => "host trust policy contains too many rules",
        })
    }
}

impl std::error::Error for DistributionError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(character: char) -> Sha256Digest {
        Sha256Digest::parse(character.to_string().repeat(64)).expect("test digest should be valid")
    }

    fn file(path: &str, bytes: u64, character: char) -> BundleFile {
        BundleFile::new(
            BundlePath::parse(path).expect("test path should be valid"),
            bytes,
            digest(character),
        )
    }

    #[test]
    fn bundle_manifest_normalizes_paths_and_enforces_entry_point() {
        let manifest = BundleManifest::new(
            BUNDLE_FORMAT,
            SHA256_ALGORITHM,
            BundlePath::parse("pack.yaml").expect("entry point should be valid"),
            vec![
                file("pack.yaml", 10, '0'),
                file("overlays/core.md", 12, '1'),
            ],
            BundleLimits::default(),
        )
        .expect("manifest should be valid");

        assert_eq!(manifest.files()[0].path().as_str(), "overlays/core.md");
        assert_eq!(manifest.files()[1].path().as_str(), "pack.yaml");

        let missing = BundleManifest::new(
            BUNDLE_FORMAT,
            SHA256_ALGORITHM,
            BundlePath::parse("missing.yaml").expect("entry point should be valid"),
            vec![file("pack.yaml", 10, '0')],
            BundleLimits::default(),
        );
        assert_eq!(missing, Err(DistributionError::EntryPointMissing));
    }

    #[test]
    fn bundle_manifest_rejects_duplicates_and_tighter_limits() {
        let duplicate = BundleManifest::new(
            BUNDLE_FORMAT,
            SHA256_ALGORITHM,
            BundlePath::parse("pack.yaml").expect("entry point should be valid"),
            vec![file("pack.yaml", 1, '0'), file("pack.yaml", 1, '1')],
            BundleLimits::default(),
        );
        assert_eq!(duplicate, Err(DistributionError::DuplicatePath));

        let over_file_limit = BundleManifest::new(
            BUNDLE_FORMAT,
            SHA256_ALGORITHM,
            BundlePath::parse("pack.yaml").expect("entry point should be valid"),
            vec![file("pack.yaml", 2, '0')],
            BundleLimits::new(1, 1, 8),
        );
        assert_eq!(over_file_limit, Err(DistributionError::FileTooLarge));
    }

    #[test]
    fn host_limits_cannot_relax_v1_hard_maxima() {
        let relaxed = BundleLimits::new(usize::MAX, u64::MAX, u64::MAX);
        assert_eq!(relaxed.maximum_files(), MAX_BUNDLE_FILES);
        assert_eq!(relaxed.maximum_file_bytes(), MAX_BUNDLE_FILE_BYTES);
        assert_eq!(relaxed.maximum_expanded_bytes(), MAX_BUNDLE_EXPANDED_BYTES);
    }

    #[test]
    fn paths_and_digests_fail_closed() {
        for path in [
            "",
            "/pack.yaml",
            "../pack.yaml",
            "a//b",
            "a\\b",
            "C:/pack.yaml",
            "a\tb",
            "café.md",
        ] {
            assert!(BundlePath::parse(path).is_err(), "path should fail: {path}");
        }
        assert!(Sha256Digest::parse("A".repeat(64)).is_err());
        assert!(Sha256Digest::parse("0".repeat(63)).is_err());
    }

    #[test]
    fn publisher_policy_requires_expected_subject_and_explicit_identity() {
        let mechanism =
            VerificationMechanism::parse("sigstore-keyless-v1").expect("mechanism should be valid");
        let assertion = PublisherAssertion::new(
            mechanism.clone(),
            digest('a'),
            PublisherIdentity::new(vec![
                (
                    "issuer".into(),
                    "https://token.actions.githubusercontent.com".into(),
                ),
                (
                    "repository".into(),
                    "hackelia-micrantha/invokrum-packs".into(),
                ),
                ("workflow".into(), ".github/workflows/release.yml".into()),
            ])
            .expect("identity should be valid"),
        );
        let policy = TrustPolicy::new(vec![TrustRule::new(
            mechanism,
            PublisherIdentity::new(vec![
                (
                    "repository".into(),
                    "hackelia-micrantha/invokrum-packs".into(),
                ),
                ("workflow".into(), ".github/workflows/release.yml".into()),
            ])
            .expect("rule identity should be valid"),
        )])
        .expect("policy should be valid");

        assert_eq!(policy.authorize(&assertion, &digest('a')), Ok(()));
        assert_eq!(
            policy.authorize(&assertion, &digest('b')),
            Err(TrustError::SubjectMismatch)
        );
    }

    #[test]
    fn cryptographically_valid_but_unrecognized_identity_is_denied() {
        let assertion = PublisherAssertion::new(
            VerificationMechanism::parse("ed25519-key-v1").expect("mechanism should be valid"),
            digest('a'),
            PublisherIdentity::new(vec![("key-id".into(), "attacker-key".into())])
                .expect("identity should be valid"),
        );
        let policy = TrustPolicy::new(vec![TrustRule::new(
            VerificationMechanism::parse("ed25519-key-v1").expect("mechanism should be valid"),
            PublisherIdentity::new(vec![("key-id".into(), "trusted-key".into())])
                .expect("identity should be valid"),
        )])
        .expect("policy should be valid");

        assert_eq!(
            policy.authorize(&assertion, &digest('a')),
            Err(TrustError::PublisherNotAllowed)
        );
    }

    #[test]
    fn publisher_identity_rejects_duplicate_or_non_normalized_names() {
        assert_eq!(
            PublisherIdentity::new(vec![
                ("issuer".into(), "one".into()),
                ("issuer".into(), "two".into()),
            ]),
            Err(DistributionError::DuplicateIdentityAttribute)
        );
        assert_eq!(
            PublisherIdentity::new(vec![("Issuer".into(), "one".into())]),
            Err(DistributionError::InvalidIdentityAttributeName)
        );
    }
}
