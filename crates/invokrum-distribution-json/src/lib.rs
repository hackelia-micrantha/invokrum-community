//! Strict canonical JSON adapter for `invokrum.pack-bundle/v1`.
//!
//! This crate owns serialization, duplicate-key rejection, canonical JSON byte
//! representation, and SHA-256 subject derivation for validated distribution
//! values. It performs no network, filesystem, clock, credential, or signature
//! provider access.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fmt;

use invokrum_digest::sha256_lower_hex;
use invokrum_distribution::{
    BundleFile, BundleLimits, BundleManifest, BundlePath, DistributionError, Sha256Digest,
};
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

pub const MAX_BUNDLE_MANIFEST_BYTES: usize = 1_048_576;
const DUPLICATE_KEY_MARKER: &str = "invokrum_duplicate_object_key";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BundleDocument {
    format: String,
    digest_algorithm: String,
    entry_point: String,
    files: Vec<FileDocument>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileDocument {
    path: String,
    byte_length: u64,
    digest: String,
}

#[derive(Serialize)]
struct CanonicalBundle<'a> {
    format: &'static str,
    digest_algorithm: &'static str,
    entry_point: &'a str,
    files: Vec<CanonicalFile<'a>>,
}

#[derive(Serialize)]
struct CanonicalFile<'a> {
    path: &'a str,
    byte_length: u64,
    digest: &'a str,
}

impl<'a> From<&'a BundleManifest> for CanonicalBundle<'a> {
    fn from(manifest: &'a BundleManifest) -> Self {
        Self {
            format: manifest.format(),
            digest_algorithm: manifest.digest_algorithm(),
            entry_point: manifest.entry_point().as_str(),
            files: manifest
                .files()
                .iter()
                .map(|file| CanonicalFile {
                    path: file.path().as_str(),
                    byte_length: file.byte_length(),
                    digest: file.digest().as_str(),
                })
                .collect(),
        }
    }
}

/// Decodes exact canonical `invokrum.pack-bundle/v1` JSON bytes.
///
/// # Errors
///
/// Fails closed for oversized input, malformed or duplicate-key JSON, unknown
/// fields, invalid domain values, resource-limit violations, and any byte stream
/// that differs from the canonical re-encoding of the validated manifest.
pub fn decode_bundle_manifest(
    bytes: &[u8],
    limits: BundleLimits,
) -> Result<BundleManifest, BundleJsonError> {
    if bytes.len() > MAX_BUNDLE_MANIFEST_BYTES {
        return Err(BundleJsonError::InputTooLarge);
    }

    reject_duplicate_keys(bytes)?;
    let document: BundleDocument =
        serde_json::from_slice(bytes).map_err(|_| BundleJsonError::MalformedJson)?;

    if document.files.len() > limits.maximum_files() {
        return Err(BundleJsonError::Validation(DistributionError::TooManyFiles));
    }
    let mut aggregate = 0_u64;
    for file in &document.files {
        if file.byte_length > limits.maximum_file_bytes() {
            return Err(BundleJsonError::Validation(DistributionError::FileTooLarge));
        }
        aggregate = aggregate
            .checked_add(file.byte_length)
            .ok_or(BundleJsonError::Validation(
                DistributionError::BundleTooLarge,
            ))?;
        if aggregate > limits.maximum_expanded_bytes() {
            return Err(BundleJsonError::Validation(
                DistributionError::BundleTooLarge,
            ));
        }
    }

    let entry_point =
        BundlePath::parse(document.entry_point).map_err(BundleJsonError::Validation)?;
    let files = document
        .files
        .into_iter()
        .map(|file| {
            Ok(BundleFile::new(
                BundlePath::parse(file.path).map_err(BundleJsonError::Validation)?,
                file.byte_length,
                Sha256Digest::parse(file.digest).map_err(BundleJsonError::Validation)?,
            ))
        })
        .collect::<Result<Vec<_>, BundleJsonError>>()?;

    let manifest = BundleManifest::new(
        &document.format,
        &document.digest_algorithm,
        entry_point,
        files,
        limits,
    )
    .map_err(BundleJsonError::Validation)?;

    if encode_bundle_manifest(&manifest)? != bytes {
        return Err(BundleJsonError::NonCanonical);
    }
    Ok(manifest)
}

/// Encodes a validated manifest as exact canonical JSON bytes.
///
/// # Errors
///
/// Returns [`BundleJsonError::Canonicalization`] if serialization unexpectedly
/// fails.
pub fn encode_bundle_manifest(manifest: &BundleManifest) -> Result<Vec<u8>, BundleJsonError> {
    serde_json::to_vec(&CanonicalBundle::from(manifest))
        .map_err(|_| BundleJsonError::Canonicalization)
}

/// Derives the immutable SHA-256 subject digest of canonical bundle-manifest
/// bytes.
///
/// # Errors
///
/// Returns a canonicalization error if canonical JSON serialization fails.
pub fn bundle_subject_digest(manifest: &BundleManifest) -> Result<Sha256Digest, BundleJsonError> {
    let bytes = encode_bundle_manifest(manifest)?;
    Sha256Digest::parse(sha256_lower_hex(&bytes)).map_err(BundleJsonError::Validation)
}

fn reject_duplicate_keys(bytes: &[u8]) -> Result<(), BundleJsonError> {
    match serde_json::from_slice::<DuplicateChecked>(bytes) {
        Ok(_) => Ok(()),
        Err(error) if error.to_string().contains(DUPLICATE_KEY_MARKER) => {
            Err(BundleJsonError::DuplicateKey)
        }
        Err(_) => Err(BundleJsonError::MalformedJson),
    }
}

struct DuplicateChecked;

impl<'de> Deserialize<'de> for DuplicateChecked {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(DuplicateCheckedVisitor)
    }
}

struct DuplicateCheckedVisitor;

impl<'de> Visitor<'de> for DuplicateCheckedVisitor {
    type Value = DuplicateChecked;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value without duplicate object keys")
    }

    fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E> {
        Ok(DuplicateChecked)
    }

    fn visit_i64<E>(self, _value: i64) -> Result<Self::Value, E> {
        Ok(DuplicateChecked)
    }

    fn visit_u64<E>(self, _value: u64) -> Result<Self::Value, E> {
        Ok(DuplicateChecked)
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E> {
        Ok(DuplicateChecked)
    }

    fn visit_str<E>(self, _value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(DuplicateChecked)
    }

    fn visit_string<E>(self, _value: String) -> Result<Self::Value, E> {
        Ok(DuplicateChecked)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(DuplicateChecked)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(DuplicateChecked)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        DuplicateChecked::deserialize(deserializer)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while sequence.next_element::<DuplicateChecked>()?.is_some() {}
        Ok(DuplicateChecked)
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut keys = BTreeSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if !keys.insert(key) {
                return Err(de::Error::custom(DUPLICATE_KEY_MARKER));
            }
            map.next_value::<DuplicateChecked>()?;
        }
        Ok(DuplicateChecked)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BundleJsonError {
    InputTooLarge,
    DuplicateKey,
    MalformedJson,
    Validation(DistributionError),
    NonCanonical,
    Canonicalization,
}

impl fmt::Display for BundleJsonError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InputTooLarge => "bundle manifest exceeds maximum input bytes",
            Self::DuplicateKey => "bundle manifest contains a duplicate JSON object key",
            Self::MalformedJson => "bundle manifest is malformed JSON or has unknown fields",
            Self::Validation(_) => "bundle manifest violates distribution invariants",
            Self::NonCanonical => "bundle manifest is not exact canonical JSON",
            Self::Canonicalization => "bundle manifest canonicalization failed",
        })
    }
}

impl std::error::Error for BundleJsonError {}

#[cfg(test)]
mod tests {
    use super::*;
    use invokrum_distribution::{BUNDLE_FORMAT, SHA256_ALGORITHM};

    const ZERO_DIGEST: &str = "0000000000000000000000000000000000000000000000000000000000000000";

    #[test]
    fn duplicate_keys_are_rejected_before_semantic_decoding() {
        let bytes = format!(
            "{{\"format\":\"{BUNDLE_FORMAT}\",\"format\":\"{BUNDLE_FORMAT}\",\"digest_algorithm\":\"{SHA256_ALGORITHM}\",\"entry_point\":\"pack.yaml\",\"files\":[{{\"path\":\"pack.yaml\",\"byte_length\":1,\"digest\":\"{ZERO_DIGEST}\"}}]}}"
        );
        assert_eq!(
            decode_bundle_manifest(bytes.as_bytes(), BundleLimits::default()),
            Err(BundleJsonError::DuplicateKey)
        );
    }

    #[test]
    fn whitespace_and_noncanonical_file_order_are_rejected() {
        let bytes = format!(
            "{{\"format\":\"{BUNDLE_FORMAT}\",\"digest_algorithm\":\"{SHA256_ALGORITHM}\",\"entry_point\":\"pack.yaml\",\"files\":[{{\"path\":\"pack.yaml\",\"byte_length\":1,\"digest\":\"{ZERO_DIGEST}\"}}]}}\n"
        );
        assert_eq!(
            decode_bundle_manifest(bytes.as_bytes(), BundleLimits::default()),
            Err(BundleJsonError::NonCanonical)
        );
    }
}
