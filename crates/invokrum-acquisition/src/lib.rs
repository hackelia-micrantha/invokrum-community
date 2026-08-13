//! Offline acquisition use cases over exact in-memory candidate bytes.
//!
//! This crate verifies that a candidate file set exactly matches a validated
//! `invokrum.pack-bundle/v1` manifest and expected immutable subject. It performs
//! no filesystem, archive, network, process, environment, clock, credential,
//! trust-store, serialization, or signature-provider access.

#![forbid(unsafe_code)]

use std::fmt;

use invokrum_digest::sha256_lower_hex;
use invokrum_distribution::{
    BundleManifest, BundlePath, MAX_BUNDLE_EXPANDED_BYTES, MAX_BUNDLE_FILE_BYTES, MAX_BUNDLE_FILES,
    Sha256Digest,
};

/// Exact, per-file-bounded candidate bytes supplied by an outer adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateFile {
    path: BundlePath,
    bytes: Vec<u8>,
}

impl CandidateFile {
    /// Creates a candidate file whose owned bytes fit the immutable v1 per-file bound.
    ///
    /// # Errors
    ///
    /// Returns [`CandidateVerificationError::FileTooLarge`] when `bytes` exceeds
    /// [`MAX_BUNDLE_FILE_BYTES`].
    pub fn new(path: BundlePath, bytes: Vec<u8>) -> Result<Self, CandidateVerificationError> {
        let byte_length = u64::try_from(bytes.len())
            .map_err(|_| CandidateVerificationError::FileTooLarge { path: path.clone() })?;
        if byte_length > MAX_BUNDLE_FILE_BYTES {
            return Err(CandidateVerificationError::FileTooLarge { path });
        }
        Ok(Self { path, bytes })
    }

    #[must_use]
    pub const fn path(&self) -> &BundlePath {
        &self.path
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Exact bytes proven to match one declared bundle file record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedFile {
    path: BundlePath,
    bytes: Vec<u8>,
    digest: Sha256Digest,
}

impl VerifiedFile {
    #[must_use]
    pub const fn path(&self) -> &BundlePath {
        &self.path
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    #[must_use]
    pub const fn digest(&self) -> &Sha256Digest {
        &self.digest
    }
}

/// Immutable in-memory result suitable for a later quarantine installer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedBundle {
    subject: Sha256Digest,
    entry_point: BundlePath,
    files: Vec<VerifiedFile>,
}

impl VerifiedBundle {
    #[must_use]
    pub const fn subject(&self) -> &Sha256Digest {
        &self.subject
    }

    #[must_use]
    pub const fn entry_point(&self) -> &BundlePath {
        &self.entry_point
    }

    #[must_use]
    pub fn files(&self) -> &[VerifiedFile] {
        &self.files
    }
}

/// Verifies exact candidate bytes against a validated bundle manifest.
///
/// `manifest_subject` must be the immutable subject derived for this `manifest`
/// by the canonical bundle serialization boundary. This use case intentionally
/// does not depend on that serialization adapter; it checks that the supplied
/// subject is the exact subject the caller expected before inspecting candidate
/// bytes.
///
/// # Errors
///
/// Fails closed for subject mismatch, candidate resource-limit violations,
/// duplicate/missing/undeclared paths, byte-length mismatch, or digest mismatch.
pub fn verify_candidate(
    manifest: &BundleManifest,
    manifest_subject: &Sha256Digest,
    expected_subject: &Sha256Digest,
    mut candidates: Vec<CandidateFile>,
) -> Result<VerifiedBundle, CandidateVerificationError> {
    if manifest_subject != expected_subject {
        return Err(CandidateVerificationError::SubjectMismatch);
    }

    if candidates.len() > MAX_BUNDLE_FILES {
        return Err(CandidateVerificationError::TooManyFiles);
    }

    let mut aggregate_bytes = 0_u64;
    for candidate in &candidates {
        let byte_length = u64::try_from(candidate.bytes.len())
            .map_err(|_| CandidateVerificationError::BundleTooLarge)?;
        if byte_length > MAX_BUNDLE_FILE_BYTES {
            return Err(CandidateVerificationError::FileTooLarge {
                path: candidate.path.clone(),
            });
        }
        aggregate_bytes = aggregate_bytes
            .checked_add(byte_length)
            .ok_or(CandidateVerificationError::BundleTooLarge)?;
        if aggregate_bytes > MAX_BUNDLE_EXPANDED_BYTES {
            return Err(CandidateVerificationError::BundleTooLarge);
        }
    }

    candidates.sort_by(|left, right| left.path.cmp(&right.path));
    if let Some(duplicate) = candidates
        .windows(2)
        .find(|window| window[0].path == window[1].path)
    {
        return Err(CandidateVerificationError::DuplicatePath {
            path: duplicate[0].path.clone(),
        });
    }

    let manifest_files = manifest.files();
    let mut verified = Vec::with_capacity(manifest_files.len());
    let mut manifest_index = 0_usize;
    let mut candidate_index = 0_usize;

    while manifest_index < manifest_files.len() && candidate_index < candidates.len() {
        let declared = &manifest_files[manifest_index];
        let candidate = &candidates[candidate_index];

        match candidate.path.cmp(declared.path()) {
            std::cmp::Ordering::Less => {
                return Err(CandidateVerificationError::UndeclaredFile {
                    path: candidate.path.clone(),
                });
            }
            std::cmp::Ordering::Greater => {
                return Err(CandidateVerificationError::MissingFile {
                    path: declared.path().clone(),
                });
            }
            std::cmp::Ordering::Equal => {
                let actual_length = u64::try_from(candidate.bytes.len())
                    .map_err(|_| CandidateVerificationError::BundleTooLarge)?;
                if actual_length != declared.byte_length() {
                    return Err(CandidateVerificationError::LengthMismatch {
                        path: declared.path().clone(),
                        expected: declared.byte_length(),
                        actual: actual_length,
                    });
                }

                let actual_digest = sha256_lower_hex(&candidate.bytes);
                if actual_digest != declared.digest().as_str() {
                    return Err(CandidateVerificationError::DigestMismatch {
                        path: declared.path().clone(),
                    });
                }

                manifest_index += 1;
                candidate_index += 1;
            }
        }
    }

    if manifest_index < manifest_files.len() {
        return Err(CandidateVerificationError::MissingFile {
            path: manifest_files[manifest_index].path().clone(),
        });
    }
    if candidate_index < candidates.len() {
        return Err(CandidateVerificationError::UndeclaredFile {
            path: candidates[candidate_index].path.clone(),
        });
    }

    for (declared, candidate) in manifest_files.iter().zip(candidates) {
        verified.push(VerifiedFile {
            path: candidate.path,
            bytes: candidate.bytes,
            digest: declared.digest().clone(),
        });
    }

    Ok(VerifiedBundle {
        subject: manifest_subject.clone(),
        entry_point: manifest.entry_point().clone(),
        files: verified,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CandidateVerificationError {
    SubjectMismatch,
    TooManyFiles,
    FileTooLarge {
        path: BundlePath,
    },
    BundleTooLarge,
    DuplicatePath {
        path: BundlePath,
    },
    MissingFile {
        path: BundlePath,
    },
    UndeclaredFile {
        path: BundlePath,
    },
    LengthMismatch {
        path: BundlePath,
        expected: u64,
        actual: u64,
    },
    DigestMismatch {
        path: BundlePath,
    },
}

impl fmt::Display for CandidateVerificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SubjectMismatch => {
                formatter.write_str("bundle subject does not match expected subject")
            }
            Self::TooManyFiles => formatter.write_str("candidate contains too many files"),
            Self::FileTooLarge { path } => {
                write!(formatter, "candidate file exceeds v1 size limit: {path}")
            }
            Self::BundleTooLarge => {
                formatter.write_str("candidate aggregate bytes exceed v1 size limit")
            }
            Self::DuplicatePath { path } => {
                write!(formatter, "candidate contains duplicate path: {path}")
            }
            Self::MissingFile { path } => {
                write!(formatter, "candidate is missing declared file: {path}")
            }
            Self::UndeclaredFile { path } => {
                write!(formatter, "candidate contains undeclared file: {path}")
            }
            Self::LengthMismatch { path, .. } => {
                write!(
                    formatter,
                    "candidate byte length differs from manifest: {path}"
                )
            }
            Self::DigestMismatch { path } => {
                write!(formatter, "candidate digest differs from manifest: {path}")
            }
        }
    }
}

impl std::error::Error for CandidateVerificationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use invokrum_distribution::{BUNDLE_FORMAT, BundleFile, BundleLimits, SHA256_ALGORITHM};

    fn path(value: &str) -> BundlePath {
        BundlePath::parse(value).expect("test path should be valid")
    }

    fn digest(bytes: &[u8]) -> Sha256Digest {
        Sha256Digest::parse(sha256_lower_hex(bytes)).expect("digest should be valid")
    }

    fn subject(character: char) -> Sha256Digest {
        Sha256Digest::parse(character.to_string().repeat(64)).expect("subject should be valid")
    }

    fn candidate(candidate_path: &str, bytes: &[u8]) -> CandidateFile {
        CandidateFile::new(path(candidate_path), bytes.to_vec())
            .expect("test candidate should fit the v1 file bound")
    }

    fn manifest() -> BundleManifest {
        BundleManifest::new(
            BUNDLE_FORMAT,
            SHA256_ALGORITHM,
            path("pack.yaml"),
            vec![
                BundleFile::new(path("pack.yaml"), 4, digest(b"pack")),
                BundleFile::new(path("overlays/core.md"), 4, digest(b"core")),
            ],
            BundleLimits::default(),
        )
        .expect("manifest should be valid")
    }

    fn matching_candidates() -> Vec<CandidateFile> {
        vec![
            candidate("pack.yaml", b"pack"),
            candidate("overlays/core.md", b"core"),
        ]
    }

    #[test]
    fn candidate_file_rejects_bytes_above_v1_bound() {
        let candidate_path = path("pack.yaml");
        let oversized =
            vec![0_u8; usize::try_from(MAX_BUNDLE_FILE_BYTES + 1).expect("limit fits usize")];

        assert_eq!(
            CandidateFile::new(candidate_path.clone(), oversized),
            Err(CandidateVerificationError::FileTooLarge {
                path: candidate_path
            })
        );
    }

    #[test]
    fn verifies_exact_bytes_and_normalizes_file_order() {
        let manifest = manifest();
        let bundle_subject = subject('a');
        let verified = verify_candidate(
            &manifest,
            &bundle_subject,
            &bundle_subject,
            matching_candidates(),
        )
        .expect("candidate should verify");

        assert_eq!(verified.subject(), &bundle_subject);
        assert_eq!(verified.entry_point().as_str(), "pack.yaml");
        assert_eq!(verified.files()[0].path().as_str(), "overlays/core.md");
        assert_eq!(verified.files()[0].bytes(), b"core");
        assert_eq!(verified.files()[1].path().as_str(), "pack.yaml");
        assert_eq!(verified.files()[1].bytes(), b"pack");
    }

    #[test]
    fn subject_mismatch_precedes_candidate_set_validation() {
        let manifest = manifest();
        let duplicate = vec![
            candidate("pack.yaml", b"pack"),
            candidate("pack.yaml", b"pack"),
        ];

        assert_eq!(
            verify_candidate(&manifest, &subject('a'), &subject('b'), duplicate),
            Err(CandidateVerificationError::SubjectMismatch)
        );
    }

    #[test]
    fn rejects_duplicate_missing_and_undeclared_paths() {
        let manifest = manifest();
        let bundle_subject = subject('a');

        let duplicate = vec![
            candidate("pack.yaml", b"pack"),
            candidate("pack.yaml", b"pack"),
        ];
        assert_eq!(
            verify_candidate(&manifest, &bundle_subject, &bundle_subject, duplicate),
            Err(CandidateVerificationError::DuplicatePath {
                path: path("pack.yaml")
            })
        );

        let missing = vec![candidate("pack.yaml", b"pack")];
        assert_eq!(
            verify_candidate(&manifest, &bundle_subject, &bundle_subject, missing),
            Err(CandidateVerificationError::MissingFile {
                path: path("overlays/core.md")
            })
        );

        let mut extra = matching_candidates();
        extra.push(candidate("extra.md", b"extra"));
        assert_eq!(
            verify_candidate(&manifest, &bundle_subject, &bundle_subject, extra),
            Err(CandidateVerificationError::UndeclaredFile {
                path: path("extra.md")
            })
        );
    }

    #[test]
    fn rejects_length_and_digest_mismatch() {
        let manifest = manifest();
        let bundle_subject = subject('a');

        let mut wrong_length = matching_candidates();
        wrong_length[1] = candidate("overlays/core.md", b"cores");
        assert_eq!(
            verify_candidate(&manifest, &bundle_subject, &bundle_subject, wrong_length),
            Err(CandidateVerificationError::LengthMismatch {
                path: path("overlays/core.md"),
                expected: 4,
                actual: 5,
            })
        );

        let mut wrong_digest = matching_candidates();
        wrong_digest[1] = candidate("overlays/core.md", b"xxxx");
        assert_eq!(
            verify_candidate(&manifest, &bundle_subject, &bundle_subject, wrong_digest),
            Err(CandidateVerificationError::DigestMismatch {
                path: path("overlays/core.md")
            })
        );
    }
}
