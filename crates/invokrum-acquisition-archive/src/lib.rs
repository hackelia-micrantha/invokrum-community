//! Deterministic bounded archive acquisition for exact Invokrum candidate bytes.
//!
//! The initial archive profile is uncompressed POSIX ustar. Deliberately excluding
//! compression keeps decompression and compression-ratio attack surface out of this
//! first adapter: compressed archive inputs are rejected before any expansion. The
//! adapter validates archive structure, entry metadata, paths, exact manifest-tree
//! membership, and hard resource bounds, then returns owned `CandidateFile` bytes.
//! It performs no hashing, immutable-subject verification, filesystem extraction,
//! installation, network access, credential lookup, trust lookup, or signature work.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fmt;

use invokrum_acquisition::CandidateFile;
use invokrum_distribution::{
    BundleManifest, BundlePath, MAX_BUNDLE_EXPANDED_BYTES, MAX_BUNDLE_FILE_BYTES,
};

const USTAR_BLOCK_BYTES: usize = 512;
const USTAR_NAME_BYTES: usize = 100;
const USTAR_PREFIX_BYTES: usize = 155;
const MAX_USTAR_ENTRIES: usize = 1_024;
const MAX_USTAR_NESTING: usize = 64;
const MAX_USTAR_METADATA_BYTES: usize = (MAX_USTAR_ENTRIES + 2) * USTAR_BLOCK_BYTES;
// 32 MiB payload + 1,024 headers + worst-case block padding + terminators.
const MAX_USTAR_ARCHIVE_BYTES: usize = 34_603_008;

#[derive(Clone, Copy, Debug)]
struct ParserLimits {
    entries: usize,
    metadata_bytes: usize,
    expanded_bytes: u64,
    nesting: usize,
}

impl ParserLimits {
    const DEFAULT: Self = Self {
        entries: MAX_USTAR_ENTRIES,
        metadata_bytes: MAX_USTAR_METADATA_BYTES,
        expanded_bytes: MAX_BUNDLE_EXPANDED_BYTES,
        nesting: MAX_USTAR_NESTING,
    };
}

/// Owned local ustar bytes that can be converted into exact candidate files.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UstarCandidateSource {
    bytes: Vec<u8>,
}

impl UstarCandidateSource {
    /// Accepts one caller-supplied uncompressed ustar archive under the hard input bound.
    ///
    /// # Errors
    ///
    /// Returns [`ArchiveCandidateError::CompressedArchiveUnsupported`] for recognized
    /// compressed archive envelopes and [`ArchiveCandidateError::ArchiveTooLarge`] when
    /// the supplied bytes exceed the deterministic ustar input bound.
    pub fn new(bytes: Vec<u8>) -> Result<Self, ArchiveCandidateError> {
        if looks_compressed(&bytes) && !starts_with_accepted_ustar_header(&bytes) {
            return Err(ArchiveCandidateError::CompressedArchiveUnsupported);
        }
        if bytes.len() > MAX_USTAR_ARCHIVE_BYTES {
            return Err(ArchiveCandidateError::ArchiveTooLarge);
        }
        Ok(Self { bytes })
    }

    /// Parses exact owned candidate bytes for one already-validated bundle manifest.
    ///
    /// # Errors
    ///
    /// Fails closed for malformed or ambiguous ustar structure, unsupported metadata
    /// or entry kinds, invalid/colliding paths, manifest-tree mismatches, or resource
    /// limit violations.
    pub fn load(
        &self,
        manifest: &BundleManifest,
    ) -> Result<Vec<CandidateFile>, ArchiveCandidateError> {
        self.load_with_limits(manifest, ParserLimits::DEFAULT)
    }

    fn load_with_limits(
        &self,
        manifest: &BundleManifest,
        limits: ParserLimits,
    ) -> Result<Vec<CandidateFile>, ArchiveCandidateError> {
        parse_ustar(&self.bytes, manifest, limits)
    }
}

/// Fail-closed archive acquisition errors. These describe archive parsing only;
/// content digest and immutable-subject failures remain owned by `verify_candidate`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArchiveCandidateError {
    ArchiveTooLarge,
    CompressedArchiveUnsupported,
    TruncatedArchive,
    TrailingData,
    MalformedHeader,
    HeaderChecksumMismatch,
    UnsupportedArchiveFormat,
    UnsupportedMetadata,
    UnsupportedEntryType,
    InvalidEntryName,
    LogicalPathCollision { path: BundlePath },
    UndeclaredEntry { path: BundlePath },
    MissingDeclaredFile { path: BundlePath },
    FileTooLarge { path: BundlePath },
    TooManyEntries,
    MetadataLimitExceeded,
    NestingLimitExceeded { path: BundlePath },
    ExpandedBytesLimitExceeded,
}

impl fmt::Display for ArchiveCandidateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArchiveTooLarge => {
                formatter.write_str("archive exceeds the bounded ustar input limit")
            }
            Self::CompressedArchiveUnsupported => formatter.write_str(
                "compressed archives are unsupported by the initial bounded ustar profile",
            ),
            Self::TruncatedArchive => formatter.write_str("archive is truncated"),
            Self::TrailingData => {
                formatter.write_str("archive contains trailing or concatenated data")
            }
            Self::MalformedHeader => {
                formatter.write_str("archive contains a malformed ustar header")
            }
            Self::HeaderChecksumMismatch => {
                formatter.write_str("archive header checksum does not match")
            }
            Self::UnsupportedArchiveFormat => {
                formatter.write_str("archive is not the supported POSIX ustar profile")
            }
            Self::UnsupportedMetadata => {
                formatter.write_str("archive contains unsupported or ambiguous metadata")
            }
            Self::UnsupportedEntryType => {
                formatter.write_str("archive contains a prohibited entry type")
            }
            Self::InvalidEntryName => {
                formatter.write_str("archive contains an invalid bundle entry name")
            }
            Self::LogicalPathCollision { path } => {
                write!(
                    formatter,
                    "archive contains a duplicate or case-colliding path: {path}"
                )
            }
            Self::UndeclaredEntry { path } => {
                write!(formatter, "archive contains an undeclared entry: {path}")
            }
            Self::MissingDeclaredFile { path } => {
                write!(formatter, "archive is missing a declared file: {path}")
            }
            Self::FileTooLarge { path } => {
                write!(
                    formatter,
                    "archive entry exceeds the v1 per-file bound: {path}"
                )
            }
            Self::TooManyEntries => formatter.write_str("archive contains too many entries"),
            Self::MetadataLimitExceeded => {
                formatter.write_str("archive metadata exceeds the deterministic bound")
            }
            Self::NestingLimitExceeded { path } => {
                write!(formatter, "archive entry exceeds the nesting bound: {path}")
            }
            Self::ExpandedBytesLimitExceeded => {
                formatter.write_str("archive expanded bytes exceed the deterministic bound")
            }
        }
    }
}

impl std::error::Error for ArchiveCandidateError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EntryKind {
    File,
    Directory,
}

#[derive(Clone, Debug)]
struct ParsedHeader {
    path: BundlePath,
    size: u64,
    kind: EntryKind,
}

#[derive(Debug)]
struct ExpectedTree {
    files: BTreeSet<BundlePath>,
    directories: BTreeSet<BundlePath>,
}

impl ExpectedTree {
    fn from_manifest(manifest: &BundleManifest) -> Result<Self, ArchiveCandidateError> {
        let files = manifest
            .files()
            .iter()
            .map(|file| file.path().clone())
            .collect::<BTreeSet<_>>();
        let mut directories = BTreeSet::new();

        for file in manifest.files() {
            let mut candidate = file.path().as_str();
            while let Some((parent, _)) = candidate.rsplit_once('/') {
                let directory = BundlePath::parse(parent.to_owned())
                    .map_err(|_| ArchiveCandidateError::InvalidEntryName)?;
                directories.insert(directory);
                candidate = parent;
            }
        }

        ensure_portable_logical_tree(&files, &directories)?;
        Ok(Self { files, directories })
    }

    fn contains(&self, path: &BundlePath, kind: EntryKind) -> bool {
        match kind {
            EntryKind::File => self.files.contains(path),
            EntryKind::Directory => self.directories.contains(path),
        }
    }
}

fn ensure_portable_logical_tree(
    files: &BTreeSet<BundlePath>,
    directories: &BTreeSet<BundlePath>,
) -> Result<(), ArchiveCandidateError> {
    let mut logical_keys = BTreeSet::new();
    for path in files.iter().chain(directories.iter()) {
        if !logical_keys.insert(path.as_str().to_ascii_lowercase()) {
            return Err(ArchiveCandidateError::LogicalPathCollision { path: path.clone() });
        }
    }
    Ok(())
}

fn parse_ustar(
    archive: &[u8],
    manifest: &BundleManifest,
    limits: ParserLimits,
) -> Result<Vec<CandidateFile>, ArchiveCandidateError> {
    if archive.len() < USTAR_BLOCK_BYTES * 2 || archive.len() % USTAR_BLOCK_BYTES != 0 {
        return Err(ArchiveCandidateError::TruncatedArchive);
    }

    let expected = ExpectedTree::from_manifest(manifest)?;
    let mut collision_keys = BTreeSet::<String>::new();
    let mut observed_files = BTreeSet::<BundlePath>::new();
    let mut candidates = Vec::with_capacity(manifest.files().len());
    let mut cursor = 0_usize;
    let mut entry_count = 0_usize;
    let mut metadata_bytes = 0_usize;
    let mut expanded_bytes = 0_u64;

    loop {
        let block_end = cursor
            .checked_add(USTAR_BLOCK_BYTES)
            .ok_or(ArchiveCandidateError::TruncatedArchive)?;
        let block = archive
            .get(cursor..block_end)
            .ok_or(ArchiveCandidateError::TruncatedArchive)?;

        if archive_terminated(
            archive,
            block,
            block_end,
            &mut metadata_bytes,
            limits.metadata_bytes,
        )? {
            break;
        }

        entry_count = entry_count
            .checked_add(1)
            .ok_or(ArchiveCandidateError::TooManyEntries)?;
        if entry_count > limits.entries {
            return Err(ArchiveCandidateError::TooManyEntries);
        }
        add_metadata_bytes(
            &mut metadata_bytes,
            USTAR_BLOCK_BYTES,
            limits.metadata_bytes,
        )?;

        let header = parse_header(block)?;
        let nesting = header
            .path
            .as_str()
            .bytes()
            .filter(|byte| *byte == b'/')
            .count();
        if nesting > limits.nesting {
            return Err(ArchiveCandidateError::NestingLimitExceeded { path: header.path });
        }

        let collision_key = header.path.as_str().to_ascii_lowercase();
        if !collision_keys.insert(collision_key) {
            return Err(ArchiveCandidateError::LogicalPathCollision { path: header.path });
        }
        if !expected.contains(&header.path, header.kind) {
            return Err(ArchiveCandidateError::UndeclaredEntry { path: header.path });
        }

        if header.kind == EntryKind::Directory {
            if header.size != 0 {
                return Err(ArchiveCandidateError::UnsupportedMetadata);
            }
            cursor = block_end;
            continue;
        }
        if header.size > MAX_BUNDLE_FILE_BYTES {
            return Err(ArchiveCandidateError::FileTooLarge { path: header.path });
        }

        expanded_bytes = expanded_bytes
            .checked_add(header.size)
            .ok_or(ArchiveCandidateError::ExpandedBytesLimitExceeded)?;
        if expanded_bytes > limits.expanded_bytes {
            return Err(ArchiveCandidateError::ExpandedBytesLimitExceeded);
        }

        let data_length =
            usize::try_from(header.size).map_err(|_| ArchiveCandidateError::FileTooLarge {
                path: header.path.clone(),
            })?;
        let data_end = block_end
            .checked_add(data_length)
            .ok_or(ArchiveCandidateError::TruncatedArchive)?;
        let data = archive
            .get(block_end..data_end)
            .ok_or(ArchiveCandidateError::TruncatedArchive)?;
        let padded_length = data_length.div_ceil(USTAR_BLOCK_BYTES) * USTAR_BLOCK_BYTES;
        let padded_end = block_end
            .checked_add(padded_length)
            .ok_or(ArchiveCandidateError::TruncatedArchive)?;
        let padding = archive
            .get(data_end..padded_end)
            .ok_or(ArchiveCandidateError::TruncatedArchive)?;
        if padding.iter().any(|byte| *byte != 0) {
            return Err(ArchiveCandidateError::UnsupportedMetadata);
        }

        let path = header.path;
        let candidate = CandidateFile::new(path.clone(), data.to_vec())
            .map_err(|_| ArchiveCandidateError::FileTooLarge { path: path.clone() })?;
        observed_files.insert(path);
        candidates.push(candidate);
        cursor = padded_end;
    }

    ensure_all_declared_files_observed(manifest, &observed_files)?;
    Ok(candidates)
}

fn ensure_all_declared_files_observed(
    manifest: &BundleManifest,
    observed_files: &BTreeSet<BundlePath>,
) -> Result<(), ArchiveCandidateError> {
    for declared in manifest.files() {
        if !observed_files.contains(declared.path()) {
            return Err(ArchiveCandidateError::MissingDeclaredFile {
                path: declared.path().clone(),
            });
        }
    }
    Ok(())
}

fn archive_terminated(
    archive: &[u8],
    block: &[u8],
    block_end: usize,
    metadata_bytes: &mut usize,
    metadata_limit: usize,
) -> Result<bool, ArchiveCandidateError> {
    if !is_zero_block(block) {
        return Ok(false);
    }

    let second_end = block_end
        .checked_add(USTAR_BLOCK_BYTES)
        .ok_or(ArchiveCandidateError::TruncatedArchive)?;
    let second = archive
        .get(block_end..second_end)
        .ok_or(ArchiveCandidateError::TruncatedArchive)?;
    if !is_zero_block(second) {
        return Err(ArchiveCandidateError::MalformedHeader);
    }

    add_metadata_bytes(metadata_bytes, USTAR_BLOCK_BYTES * 2, metadata_limit)?;
    if archive[second_end..].iter().any(|byte| *byte != 0) {
        return Err(ArchiveCandidateError::TrailingData);
    }
    Ok(true)
}

fn add_metadata_bytes(
    metadata_bytes: &mut usize,
    additional_bytes: usize,
    limit: usize,
) -> Result<(), ArchiveCandidateError> {
    *metadata_bytes = metadata_bytes
        .checked_add(additional_bytes)
        .ok_or(ArchiveCandidateError::MetadataLimitExceeded)?;
    if *metadata_bytes > limit {
        return Err(ArchiveCandidateError::MetadataLimitExceeded);
    }
    Ok(())
}

fn parse_header(block: &[u8]) -> Result<ParsedHeader, ArchiveCandidateError> {
    if block.len() != USTAR_BLOCK_BYTES {
        return Err(ArchiveCandidateError::MalformedHeader);
    }

    let expected_checksum = parse_octal(&block[148..156], false)?;
    let actual_checksum = block
        .iter()
        .enumerate()
        .try_fold(0_u64, |sum, (index, byte)| {
            let value = if (148..156).contains(&index) {
                u64::from(b' ')
            } else {
                u64::from(*byte)
            };
            sum.checked_add(value)
                .ok_or(ArchiveCandidateError::MalformedHeader)
        })?;
    if expected_checksum != actual_checksum {
        return Err(ArchiveCandidateError::HeaderChecksumMismatch);
    }

    if &block[257..263] != b"ustar\0" || &block[263..265] != b"00" {
        return Err(ArchiveCandidateError::UnsupportedArchiveFormat);
    }
    if block[500..].iter().any(|byte| *byte != 0) {
        return Err(ArchiveCandidateError::UnsupportedMetadata);
    }

    for field in [
        &block[100..108],
        &block[108..116],
        &block[116..124],
        &block[136..148],
    ] {
        let _ = parse_octal(field, true)?;
    }
    let size = parse_octal(&block[124..136], false)?;
    let device_major = parse_octal(&block[329..337], true)?;
    let device_minor = parse_octal(&block[337..345], true)?;
    if device_major != 0 || device_minor != 0 {
        return Err(ArchiveCandidateError::UnsupportedMetadata);
    }

    let _ = parse_text_field(&block[265..297])?;
    let _ = parse_text_field(&block[297..329])?;
    let link_name = parse_text_field(&block[157..257])?;
    if !link_name.is_empty() {
        return Err(ArchiveCandidateError::UnsupportedMetadata);
    }

    let kind = match block[156] {
        0 | b'0' => EntryKind::File,
        b'5' => EntryKind::Directory,
        b'x' | b'g' | b'L' | b'K' => return Err(ArchiveCandidateError::UnsupportedMetadata),
        _ => return Err(ArchiveCandidateError::UnsupportedEntryType),
    };

    let name = parse_text_field(&block[..USTAR_NAME_BYTES])?;
    if name.is_empty() {
        return Err(ArchiveCandidateError::InvalidEntryName);
    }
    let prefix = parse_text_field(&block[345..345 + USTAR_PREFIX_BYTES])?;
    let mut joined = Vec::with_capacity(prefix.len() + name.len() + 1);
    if !prefix.is_empty() {
        joined.extend_from_slice(prefix);
        joined.push(b'/');
    }
    joined.extend_from_slice(name);

    if kind == EntryKind::Directory && joined.last() == Some(&b'/') {
        joined.pop();
    }
    let path_text =
        String::from_utf8(joined).map_err(|_| ArchiveCandidateError::InvalidEntryName)?;
    let path = BundlePath::parse(path_text).map_err(|_| ArchiveCandidateError::InvalidEntryName)?;

    Ok(ParsedHeader { path, size, kind })
}

fn parse_octal(field: &[u8], allow_empty: bool) -> Result<u64, ArchiveCandidateError> {
    let start = field
        .iter()
        .position(|byte| !matches!(*byte, 0 | b' '))
        .unwrap_or(field.len());
    let end = field
        .iter()
        .rposition(|byte| !matches!(*byte, 0 | b' '))
        .map_or(start, |index| index + 1);
    let digits = &field[start..end];

    if digits.is_empty() {
        return if allow_empty {
            Ok(0)
        } else {
            Err(ArchiveCandidateError::MalformedHeader)
        };
    }
    if !digits.iter().all(|byte| matches!(*byte, b'0'..=b'7')) {
        return Err(ArchiveCandidateError::UnsupportedMetadata);
    }

    digits.iter().try_fold(0_u64, |value, byte| {
        value
            .checked_mul(8)
            .and_then(|next| next.checked_add(u64::from(*byte - b'0')))
            .ok_or(ArchiveCandidateError::UnsupportedMetadata)
    })
}

fn parse_text_field(field: &[u8]) -> Result<&[u8], ArchiveCandidateError> {
    let end = field
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(field.len());
    if field[end..].iter().any(|byte| *byte != 0) {
        return Err(ArchiveCandidateError::UnsupportedMetadata);
    }

    let text = &field[..end];
    if text
        .iter()
        .any(|byte| !byte.is_ascii() || byte.is_ascii_control())
    {
        return Err(ArchiveCandidateError::UnsupportedMetadata);
    }
    Ok(text)
}

fn is_zero_block(block: &[u8]) -> bool {
    block.iter().all(|byte| *byte == 0)
}

fn looks_compressed(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x1f, 0x8b])
        || bytes.starts_with(b"BZh")
        || bytes.starts_with(&[0xfd, b'7', b'z', b'X', b'Z', 0x00])
        || bytes.starts_with(&[0x28, 0xb5, 0x2f, 0xfd])
}

fn starts_with_accepted_ustar_header(bytes: &[u8]) -> bool {
    bytes
        .get(..USTAR_BLOCK_BYTES)
        .is_some_and(|block| parse_header(block).is_ok())
}

#[cfg(test)]
mod tests {
    use invokrum_acquisition::verify_candidate;
    use invokrum_digest::sha256_lower_hex;
    use invokrum_distribution::{
        BUNDLE_FORMAT, BundleFile, BundleLimits, SHA256_ALGORITHM, Sha256Digest,
    };

    use super::*;

    fn path(value: &str) -> BundlePath {
        BundlePath::parse(value).expect("test path should be valid")
    }

    fn digest(bytes: &[u8]) -> Sha256Digest {
        Sha256Digest::parse(sha256_lower_hex(bytes)).expect("test digest should be valid")
    }

    fn subject(character: char) -> Sha256Digest {
        Sha256Digest::parse(character.to_string().repeat(64)).expect("subject should be valid")
    }

    fn manifest(records: &[(&str, &[u8])]) -> BundleManifest {
        BundleManifest::new(
            BUNDLE_FORMAT,
            SHA256_ALGORITHM,
            path("pack.yaml"),
            records
                .iter()
                .map(|(record_path, bytes)| {
                    BundleFile::new(
                        path(record_path),
                        u64::try_from(bytes.len()).expect("test length should fit"),
                        digest(bytes),
                    )
                })
                .collect(),
            BundleLimits::default(),
        )
        .expect("test manifest should be valid")
    }

    fn write_octal(field: &mut [u8], value: u64) {
        field.fill(0);
        let width = field.len() - 1;
        let text = format!("{value:0width$o}");
        assert_eq!(text.len(), width);
        field[..width].copy_from_slice(text.as_bytes());
    }

    fn header(name: &[u8], kind: u8, size: u64) -> [u8; USTAR_BLOCK_BYTES] {
        assert!(name.len() <= USTAR_NAME_BYTES);
        let mut block = [0_u8; USTAR_BLOCK_BYTES];
        block[..name.len()].copy_from_slice(name);
        write_octal(&mut block[100..108], 0o644);
        write_octal(&mut block[108..116], 0);
        write_octal(&mut block[116..124], 0);
        write_octal(&mut block[124..136], size);
        write_octal(&mut block[136..148], 0);
        block[148..156].fill(b' ');
        block[156] = kind;
        block[257..263].copy_from_slice(b"ustar\0");
        block[263..265].copy_from_slice(b"00");
        write_octal(&mut block[329..337], 0);
        write_octal(&mut block[337..345], 0);
        let checksum = block.iter().map(|byte| u64::from(*byte)).sum::<u64>();
        let checksum_text = format!("{checksum:06o}\0 ");
        block[148..156].copy_from_slice(checksum_text.as_bytes());
        block
    }

    fn build_archive(entries: &[(&[u8], u8, &[u8])]) -> Vec<u8> {
        let mut archive = Vec::new();
        for (name, kind, data) in entries {
            archive.extend_from_slice(&header(
                name,
                *kind,
                u64::try_from(data.len()).expect("test data length should fit"),
            ));
            archive.extend_from_slice(data);
            let padded = data.len().div_ceil(USTAR_BLOCK_BYTES) * USTAR_BLOCK_BYTES;
            archive.resize(archive.len() + padded - data.len(), 0);
        }
        archive.resize(archive.len() + USTAR_BLOCK_BYTES * 2, 0);
        archive
    }

    #[test]
    fn valid_nested_archive_returns_exact_bytes_and_verifies() {
        let bundle_manifest = manifest(&[("pack.yaml", b"pack"), ("overlays/core.md", b"core")]);
        let archive = build_archive(&[
            (b"overlays/", b'5', b""),
            (b"overlays/core.md", b'0', b"core"),
            (b"pack.yaml", b'0', b"pack"),
        ]);
        let source = UstarCandidateSource::new(archive).expect("archive should be bounded");
        let candidates = source.load(&bundle_manifest).expect("archive should parse");

        assert_eq!(candidates.len(), 2);
        assert!(
            candidates
                .iter()
                .any(|file| file.path().as_str() == "pack.yaml" && file.bytes() == b"pack")
        );
        assert!(
            candidates.iter().any(|file| {
                file.path().as_str() == "overlays/core.md" && file.bytes() == b"core"
            })
        );

        let bundle_subject = subject('a');
        verify_candidate(
            &bundle_manifest,
            &bundle_subject,
            &bundle_subject,
            candidates,
        )
        .expect("existing verifier should accept exact archive bytes");
    }

    #[test]
    fn rejects_invalid_portable_paths_before_candidate_creation() {
        let bundle_manifest = manifest(&[("pack.yaml", b"pack")]);
        for invalid in [
            b"../pack.yaml".as_slice(),
            b"/pack.yaml".as_slice(),
            b"dir\\pack.yaml".as_slice(),
            b"dir//pack.yaml".as_slice(),
            b"./pack.yaml".as_slice(),
            &[0xff, b'.', b'm', b'd'],
        ] {
            let archive = build_archive(&[(invalid, b'0', b"pack")]);
            let source = UstarCandidateSource::new(archive).unwrap();
            assert!(matches!(
                source.load(&bundle_manifest),
                Err(ArchiveCandidateError::InvalidEntryName
                    | ArchiveCandidateError::UnsupportedMetadata)
            ));
        }
    }

    #[test]
    fn rejects_duplicate_and_ascii_case_collisions() {
        let bundle_manifest = manifest(&[("pack.yaml", b"pack")]);
        let duplicate =
            build_archive(&[(b"pack.yaml", b'0', b"pack"), (b"pack.yaml", b'0', b"pack")]);
        let duplicate_source = UstarCandidateSource::new(duplicate).unwrap();
        assert_eq!(
            duplicate_source.load(&bundle_manifest),
            Err(ArchiveCandidateError::LogicalPathCollision {
                path: path("pack.yaml")
            })
        );

        let collision =
            build_archive(&[(b"pack.yaml", b'0', b"pack"), (b"PACK.YAML", b'0', b"pack")]);
        let collision_source = UstarCandidateSource::new(collision).unwrap();
        assert_eq!(
            collision_source.load(&bundle_manifest),
            Err(ArchiveCandidateError::LogicalPathCollision {
                path: path("PACK.YAML")
            })
        );
    }

    #[test]
    fn rejects_implicit_logical_tree_collisions() {
        let source = UstarCandidateSource::new(build_archive(&[])).unwrap();
        let file_directory =
            manifest(&[("pack.yaml", b"pack"), ("foo", b"foo"), ("foo/bar", b"bar")]);
        assert_eq!(
            source.load(&file_directory),
            Err(ArchiveCandidateError::LogicalPathCollision { path: path("foo") })
        );

        let directory_case = manifest(&[("pack.yaml", b"pack"), ("Dir/a", b"a"), ("dir/b", b"b")]);
        assert!(matches!(
            source.load(&directory_case),
            Err(ArchiveCandidateError::LogicalPathCollision { .. })
        ));
    }

    #[test]
    fn rejects_links_special_entries_and_extension_metadata() {
        let bundle_manifest = manifest(&[("pack.yaml", b"pack")]);
        for kind in [b'1', b'2', b'3', b'4', b'6', b'7'] {
            let source =
                UstarCandidateSource::new(build_archive(&[(b"pack.yaml", kind, b"")])).unwrap();
            assert_eq!(
                source.load(&bundle_manifest),
                Err(ArchiveCandidateError::UnsupportedEntryType)
            );
        }

        for kind in [b'x', b'g', b'L', b'K'] {
            let source =
                UstarCandidateSource::new(build_archive(&[(b"pack.yaml", kind, b"")])).unwrap();
            assert_eq!(
                source.load(&bundle_manifest),
                Err(ArchiveCandidateError::UnsupportedMetadata)
            );
        }
    }

    #[test]
    fn rejects_undeclared_unrelated_and_missing_entries() {
        let bundle_manifest = manifest(&[("pack.yaml", b"pack")]);
        let undeclared = UstarCandidateSource::new(build_archive(&[
            (b"pack.yaml", b'0', b"pack"),
            (b"extra.md", b'0', b"extra"),
        ]))
        .unwrap();
        assert_eq!(
            undeclared.load(&bundle_manifest),
            Err(ArchiveCandidateError::UndeclaredEntry {
                path: path("extra.md")
            })
        );

        let unrelated_directory =
            UstarCandidateSource::new(build_archive(&[(b"extra/", b'5', b"")])).unwrap();
        assert_eq!(
            unrelated_directory.load(&bundle_manifest),
            Err(ArchiveCandidateError::UndeclaredEntry {
                path: path("extra")
            })
        );

        let missing = UstarCandidateSource::new(build_archive(&[])).unwrap();
        assert_eq!(
            missing.load(&bundle_manifest),
            Err(ArchiveCandidateError::MissingDeclaredFile {
                path: path("pack.yaml")
            })
        );
    }

    #[test]
    fn rejects_per_entry_and_parser_resource_limit_violations() {
        let bundle_manifest = manifest(&[("pack.yaml", b"pack"), ("dir/file.md", b"file")]);

        let mut oversized = Vec::from(header(b"pack.yaml", b'0', MAX_BUNDLE_FILE_BYTES + 1));
        oversized.resize(oversized.len() + USTAR_BLOCK_BYTES * 2, 0);
        let oversized_source = UstarCandidateSource::new(oversized).unwrap();
        assert_eq!(
            oversized_source.load(&bundle_manifest),
            Err(ArchiveCandidateError::FileTooLarge {
                path: path("pack.yaml")
            })
        );

        let archive = build_archive(&[
            (b"dir/", b'5', b""),
            (b"dir/file.md", b'0', b"file"),
            (b"pack.yaml", b'0', b"pack"),
        ]);
        let source = UstarCandidateSource::new(archive).unwrap();

        assert_eq!(
            source.load_with_limits(
                &bundle_manifest,
                ParserLimits {
                    entries: 1,
                    ..ParserLimits::DEFAULT
                }
            ),
            Err(ArchiveCandidateError::TooManyEntries)
        );
        assert_eq!(
            source.load_with_limits(
                &bundle_manifest,
                ParserLimits {
                    metadata_bytes: USTAR_BLOCK_BYTES - 1,
                    ..ParserLimits::DEFAULT
                }
            ),
            Err(ArchiveCandidateError::MetadataLimitExceeded)
        );
        assert_eq!(
            source.load_with_limits(
                &bundle_manifest,
                ParserLimits {
                    expanded_bytes: 7,
                    ..ParserLimits::DEFAULT
                }
            ),
            Err(ArchiveCandidateError::ExpandedBytesLimitExceeded)
        );
        assert_eq!(
            source.load_with_limits(
                &bundle_manifest,
                ParserLimits {
                    nesting: 0,
                    ..ParserLimits::DEFAULT
                }
            ),
            Err(ArchiveCandidateError::NestingLimitExceeded {
                path: path("dir/file.md")
            })
        );
    }

    #[test]
    fn compressed_envelopes_are_rejected_before_decompression() {
        for prefix in [
            vec![0x1f, 0x8b, 0x08, 0x00],
            b"BZh9".to_vec(),
            vec![0xfd, b'7', b'z', b'X', b'Z', 0x00],
            vec![0x28, 0xb5, 0x2f, 0xfd],
        ] {
            assert_eq!(
                UstarCandidateSource::new(prefix),
                Err(ArchiveCandidateError::CompressedArchiveUnsupported)
            );
        }
    }

    #[test]
    fn bzip_prefix_filename_is_not_misclassified_as_compressed() {
        let bundle_manifest = manifest(&[("pack.yaml", b"pack"), ("BZhello", b"hello")]);
        let archive = build_archive(&[(b"BZhello", b'0', b"hello"), (b"pack.yaml", b'0', b"pack")]);
        let source = UstarCandidateSource::new(archive).expect("valid ustar should win detection");
        let candidates = source.load(&bundle_manifest).expect("archive should parse");
        assert_eq!(candidates.len(), 2);
    }

    #[test]
    fn rejects_truncated_corrupt_padding_and_trailing_forms() {
        let bundle_manifest = manifest(&[("pack.yaml", b"pack")]);
        let archive = build_archive(&[(b"pack.yaml", b'0', b"pack")]);

        let mut truncated = archive.clone();
        truncated.pop();
        let truncated_source = UstarCandidateSource::new(truncated).unwrap();
        assert_eq!(
            truncated_source.load(&bundle_manifest),
            Err(ArchiveCandidateError::TruncatedArchive)
        );

        let mut corrupt = archive.clone();
        corrupt[0] ^= 1;
        let corrupt_source = UstarCandidateSource::new(corrupt).unwrap();
        assert_eq!(
            corrupt_source.load(&bundle_manifest),
            Err(ArchiveCandidateError::HeaderChecksumMismatch)
        );

        let mut nonzero_padding = archive.clone();
        nonzero_padding[USTAR_BLOCK_BYTES + 4] = 1;
        let padding_source = UstarCandidateSource::new(nonzero_padding).unwrap();
        assert_eq!(
            padding_source.load(&bundle_manifest),
            Err(ArchiveCandidateError::UnsupportedMetadata)
        );

        let mut trailing = archive.clone();
        trailing.extend_from_slice(&[1_u8; USTAR_BLOCK_BYTES]);
        let trailing_source = UstarCandidateSource::new(trailing).unwrap();
        assert_eq!(
            trailing_source.load(&bundle_manifest),
            Err(ArchiveCandidateError::TrailingData)
        );

        let mut concatenated = archive;
        concatenated.extend_from_slice(&build_archive(&[(b"pack.yaml", b'0', b"pack")]));
        let concatenated_source = UstarCandidateSource::new(concatenated).unwrap();
        assert_eq!(
            concatenated_source.load(&bundle_manifest),
            Err(ArchiveCandidateError::TrailingData)
        );
    }
}
