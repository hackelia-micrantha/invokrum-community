use std::fmt;

use invokrum_core::{Composition, Identifier, OverlayPack, PackRelativePath};
use serde::{Deserialize, Serialize};

use crate::canonical::{canonical_json, pack_bytes, profile_bytes};

pub const LOCKFILE_FORMAT: &str = "invokrum.lock/v1";
pub const CANONICALIZATION_FORMAT: &str = "invokrum.canonical-json/v1";
pub const SHA256_ALGORITHM: &str = "sha256";
pub const MAX_LOCKFILE_BYTES: usize = 1_048_576;
pub const MAX_LOCKED_OVERLAYS: usize = 256;

/// Injected content-digest capability used by lock generation and verification.
pub trait Digester {
    fn algorithm(&self) -> &'static str;
    fn digest(&self, bytes: &[u8]) -> String;
}

/// Deterministic SHA-256 implementation used by the v1 integrity format.
#[derive(Clone, Copy, Debug, Default)]
pub struct Sha256Digester;

impl Digester for Sha256Digester {
    fn algorithm(&self) -> &'static str {
        SHA256_ALGORITHM
    }

    fn digest(&self, bytes: &[u8]) -> String {
        crate::sha256::lower_hex(&crate::sha256::digest(bytes))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Lockfile {
    pub format: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub manifest: LockedManifest,
    pub manifest_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LockedManifest {
    pub engine_inputs_digest: String,
    pub pack: LockedPack,
    pub profile: LockedProfile,
    pub overlays: Vec<LockedOverlay>,
    pub output: LockedOutput,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LockedPack {
    pub id: String,
    pub schema: String,
    pub digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LockedProfile {
    pub id: String,
    pub digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LockedOverlay {
    pub class: String,
    pub id: String,
    pub source: String,
    pub byte_length: usize,
    pub digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LockedOutput {
    pub byte_length: usize,
    pub digest: String,
}

#[derive(Serialize)]
struct EngineInputs<'a> {
    pack_digest: &'a str,
    profile_digest: &'a str,
    overlays: &'a [LockedOverlay],
}

/// Stable categories explaining why current inputs differ from a valid lock.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DriftKind {
    PackMetadata,
    ProfileSelection,
    OverlaySet,
    OverlayContent { index: usize },
    RenderedOutput,
}

/// Ordered verification result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationReport {
    drifts: Vec<DriftKind>,
}

impl VerificationReport {
    #[must_use]
    pub fn is_verified(&self) -> bool {
        self.drifts.is_empty()
    }

    #[must_use]
    pub fn drifts(&self) -> &[DriftKind] {
        &self.drifts
    }
}

/// Integrity-format or operation failure distinct from repository drift.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntegrityError {
    UnknownProfile,
    InconsistentComposition,
    Encode,
    Decode,
    LockfileTooLarge,
    TooManyOverlays,
    NonCanonicalEncoding,
    InvalidIdentity,
    UnsupportedFormat,
    UnsupportedCanonicalization,
    UnsupportedDigestAlgorithm,
    InvalidDigest,
    LockfileIntegrityMismatch,
}

impl fmt::Display for IntegrityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnknownProfile => "composition profile is not declared by the pack",
            Self::InconsistentComposition => "composition does not describe the supplied pack",
            Self::Encode => "failed to encode canonical integrity data",
            Self::Decode => "failed to decode lockfile",
            Self::LockfileTooLarge => "lockfile exceeds the v1 byte limit",
            Self::TooManyOverlays => "lockfile exceeds the v1 overlay limit",
            Self::NonCanonicalEncoding => "lockfile is not canonical v1 JSON",
            Self::InvalidIdentity => "lockfile contains an invalid identifier or source path",
            Self::UnsupportedFormat => "unsupported lockfile format",
            Self::UnsupportedCanonicalization => "unsupported canonicalization format",
            Self::UnsupportedDigestAlgorithm => "unsupported digest algorithm",
            Self::InvalidDigest => "lockfile contains an invalid digest",
            Self::LockfileIntegrityMismatch => "lockfile integrity check failed",
        })
    }
}

impl std::error::Error for IntegrityError {}

/// Builds deterministic, encodable v1 lock material from a validated pack and composition.
///
/// Secret variable values cannot be persisted because this operation accepts no
/// variable-value input and the format contains only structural identities and
/// content digests.
///
/// # Errors
///
/// Returns [`IntegrityError`] when the composition is inconsistent with the pack,
/// the profile cannot be found, the injected digester is not v1 SHA-256, a v1
/// resource limit is exceeded, or canonical serialization fails.
pub fn build_lockfile(
    pack: &OverlayPack,
    composition: &Composition,
    digester: &impl Digester,
) -> Result<Lockfile, IntegrityError> {
    if digester.algorithm() != SHA256_ALGORITHM {
        return Err(IntegrityError::UnsupportedDigestAlgorithm);
    }
    if composition.segments().len() > MAX_LOCKED_OVERLAYS {
        return Err(IntegrityError::TooManyOverlays);
    }
    ensure_composition_matches(pack, composition)?;
    let profile = pack
        .profiles()
        .iter()
        .find(|profile| profile.id == composition.manifest().profile)
        .ok_or(IntegrityError::UnknownProfile)?;

    let pack_digest = digest_canonical(
        digester,
        &pack_bytes(pack).map_err(|_| IntegrityError::Encode)?,
    );
    let profile_digest = digest_canonical(
        digester,
        &profile_bytes(profile).map_err(|_| IntegrityError::Encode)?,
    );
    let overlays = composition
        .segments()
        .iter()
        .map(|segment| LockedOverlay {
            class: segment.class.to_string(),
            id: segment.overlay.to_string(),
            source: segment.source.as_str().to_owned(),
            byte_length: segment.bytes.len(),
            digest: digester.digest(&segment.bytes),
        })
        .collect::<Vec<_>>();
    let engine_inputs_digest =
        digest_engine_inputs(digester, &pack_digest, &profile_digest, &overlays)?;
    let manifest = LockedManifest {
        engine_inputs_digest,
        pack: LockedPack {
            id: pack.id.to_string(),
            schema: pack.schema_family.clone(),
            digest: pack_digest,
        },
        profile: LockedProfile {
            id: profile.id.to_string(),
            digest: profile_digest,
        },
        overlays,
        output: LockedOutput {
            byte_length: composition.normalized_context().len(),
            digest: digester.digest(composition.normalized_context()),
        },
    };
    let manifest_digest = digest_canonical(
        digester,
        &canonical_json(&manifest).map_err(|_| IntegrityError::Encode)?,
    );
    let lockfile = Lockfile {
        format: LOCKFILE_FORMAT.to_owned(),
        canonicalization: CANONICALIZATION_FORMAT.to_owned(),
        digest_algorithm: digester.algorithm().to_owned(),
        manifest,
        manifest_digest,
    };

    validate_lockfile(&lockfile, digester)?;
    ensure_encoded_size(&lockfile)?;
    Ok(lockfile)
}

/// Encodes a validated lockfile as compact canonical UTF-8 JSON bytes.
///
/// # Errors
///
/// Returns [`IntegrityError`] when the format is unsupported, an integrity field
/// is invalid, a v1 resource limit is exceeded, or JSON serialization fails.
pub fn encode_lockfile(lockfile: &Lockfile) -> Result<Vec<u8>, IntegrityError> {
    validate_lockfile(lockfile, &Sha256Digester)?;
    let bytes = canonical_json(lockfile).map_err(|_| IntegrityError::Encode)?;
    if bytes.len() > MAX_LOCKFILE_BYTES {
        return Err(IntegrityError::LockfileTooLarge);
    }
    Ok(bytes)
}

/// Decodes and validates exact canonical v1 lockfile JSON.
///
/// Semantically equivalent JSON with whitespace, reordered fields, duplicate
/// keys, or alternate string escapes is rejected rather than normalized.
///
/// # Errors
///
/// Returns [`IntegrityError`] for oversized or malformed input, noncanonical
/// encoding, unsupported versions or algorithms, invalid identities or digest
/// text, excessive overlays, or internally inconsistent lock material.
pub fn decode_lockfile(bytes: &[u8]) -> Result<Lockfile, IntegrityError> {
    if bytes.len() > MAX_LOCKFILE_BYTES {
        return Err(IntegrityError::LockfileTooLarge);
    }
    let lockfile: Lockfile = serde_json::from_slice(bytes).map_err(|_| IntegrityError::Decode)?;
    validate_lockfile(&lockfile, &Sha256Digester)?;
    let canonical = canonical_json(&lockfile).map_err(|_| IntegrityError::Encode)?;
    if canonical != bytes {
        return Err(IntegrityError::NonCanonicalEncoding);
    }
    Ok(lockfile)
}

/// Verifies current pack and composition state against a validated lockfile.
///
/// # Errors
///
/// Returns [`IntegrityError`] when the lockfile is unsupported or internally
/// inconsistent, or current lock generation fails.
pub fn verify(
    lockfile: &Lockfile,
    pack: &OverlayPack,
    composition: &Composition,
) -> Result<VerificationReport, IntegrityError> {
    verify_with(lockfile, pack, composition, &Sha256Digester)
}

/// Verifies current state using an explicitly injected digest implementation.
///
/// # Errors
///
/// Returns [`IntegrityError`] when the lockfile is unsupported or internally
/// inconsistent, the digest implementation does not implement v1 SHA-256, or
/// current lock generation fails.
pub fn verify_with(
    lockfile: &Lockfile,
    pack: &OverlayPack,
    composition: &Composition,
    digester: &impl Digester,
) -> Result<VerificationReport, IntegrityError> {
    validate_lockfile(lockfile, digester)?;
    let current = build_lockfile(pack, composition, digester)?;
    let mut drifts = Vec::new();

    if lockfile.manifest.pack != current.manifest.pack {
        drifts.push(DriftKind::PackMetadata);
    }
    if lockfile.manifest.profile != current.manifest.profile {
        drifts.push(DriftKind::ProfileSelection);
    }

    if same_overlay_set(&lockfile.manifest.overlays, &current.manifest.overlays) {
        for (index, (expected, actual)) in lockfile
            .manifest
            .overlays
            .iter()
            .zip(&current.manifest.overlays)
            .enumerate()
        {
            if expected.byte_length != actual.byte_length || expected.digest != actual.digest {
                drifts.push(DriftKind::OverlayContent { index });
            }
        }
    } else {
        drifts.push(DriftKind::OverlaySet);
    }

    if lockfile.manifest.output != current.manifest.output {
        drifts.push(DriftKind::RenderedOutput);
    }

    Ok(VerificationReport { drifts })
}

fn ensure_composition_matches(
    pack: &OverlayPack,
    composition: &Composition,
) -> Result<(), IntegrityError> {
    let manifest = composition.manifest();
    if manifest.pack != pack.id || manifest.schema_family != pack.schema_family {
        return Err(IntegrityError::InconsistentComposition);
    }
    Ok(())
}

fn ensure_encoded_size(lockfile: &Lockfile) -> Result<(), IntegrityError> {
    let bytes = canonical_json(lockfile).map_err(|_| IntegrityError::Encode)?;
    if bytes.len() > MAX_LOCKFILE_BYTES {
        Err(IntegrityError::LockfileTooLarge)
    } else {
        Ok(())
    }
}

fn validate_lockfile(lockfile: &Lockfile, digester: &impl Digester) -> Result<(), IntegrityError> {
    if lockfile.format != LOCKFILE_FORMAT {
        return Err(IntegrityError::UnsupportedFormat);
    }
    if lockfile.canonicalization != CANONICALIZATION_FORMAT {
        return Err(IntegrityError::UnsupportedCanonicalization);
    }
    if lockfile.digest_algorithm != SHA256_ALGORITHM || digester.algorithm() != SHA256_ALGORITHM {
        return Err(IntegrityError::UnsupportedDigestAlgorithm);
    }
    if lockfile.manifest.overlays.len() > MAX_LOCKED_OVERLAYS {
        return Err(IntegrityError::TooManyOverlays);
    }
    validate_identities(lockfile)?;
    validate_digests(lockfile)?;

    let expected_inputs = digest_engine_inputs(
        digester,
        &lockfile.manifest.pack.digest,
        &lockfile.manifest.profile.digest,
        &lockfile.manifest.overlays,
    )?;
    if expected_inputs != lockfile.manifest.engine_inputs_digest {
        return Err(IntegrityError::LockfileIntegrityMismatch);
    }
    let expected_manifest = digest_canonical(
        digester,
        &canonical_json(&lockfile.manifest).map_err(|_| IntegrityError::Encode)?,
    );
    if expected_manifest != lockfile.manifest_digest {
        return Err(IntegrityError::LockfileIntegrityMismatch);
    }
    Ok(())
}

fn validate_identities(lockfile: &Lockfile) -> Result<(), IntegrityError> {
    Identifier::parse(&lockfile.manifest.pack.id).map_err(|_| IntegrityError::InvalidIdentity)?;
    Identifier::parse(&lockfile.manifest.profile.id)
        .map_err(|_| IntegrityError::InvalidIdentity)?;
    for overlay in &lockfile.manifest.overlays {
        Identifier::parse(&overlay.class).map_err(|_| IntegrityError::InvalidIdentity)?;
        Identifier::parse(&overlay.id).map_err(|_| IntegrityError::InvalidIdentity)?;
        PackRelativePath::parse(&overlay.source).map_err(|_| IntegrityError::InvalidIdentity)?;
    }
    Ok(())
}

fn validate_digests(lockfile: &Lockfile) -> Result<(), IntegrityError> {
    let digests = std::iter::once(lockfile.manifest.engine_inputs_digest.as_str())
        .chain(std::iter::once(lockfile.manifest.pack.digest.as_str()))
        .chain(std::iter::once(lockfile.manifest.profile.digest.as_str()))
        .chain(
            lockfile
                .manifest
                .overlays
                .iter()
                .map(|overlay| overlay.digest.as_str()),
        )
        .chain(std::iter::once(lockfile.manifest.output.digest.as_str()))
        .chain(std::iter::once(lockfile.manifest_digest.as_str()));
    if digests.into_iter().all(valid_sha256) {
        Ok(())
    } else {
        Err(IntegrityError::InvalidDigest)
    }
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn same_overlay_set(expected: &[LockedOverlay], actual: &[LockedOverlay]) -> bool {
    expected.len() == actual.len()
        && expected.iter().zip(actual).all(|(left, right)| {
            left.class == right.class && left.id == right.id && left.source == right.source
        })
}

fn digest_engine_inputs(
    digester: &impl Digester,
    pack_digest: &str,
    profile_digest: &str,
    overlays: &[LockedOverlay],
) -> Result<String, IntegrityError> {
    let material = EngineInputs {
        pack_digest,
        profile_digest,
        overlays,
    };
    Ok(digest_canonical(
        digester,
        &canonical_json(&material).map_err(|_| IntegrityError::Encode)?,
    ))
}

fn digest_canonical(digester: &impl Digester, bytes: &[u8]) -> String {
    digester.digest(bytes)
}
