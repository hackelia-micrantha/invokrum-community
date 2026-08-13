//! Fail-closed local filesystem adapter for Invokrum overlay sources.
//!
//! The v0.1 adapter supports Linux only. It assumes the host supplies a stable
//! mount namespace for the duration of composition. It rejects symbolic links,
//! hard links, device-boundary crossings, non-regular files, root escapes, and
//! files whose identity or metadata changes during the read.

#![forbid(unsafe_code)]

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use invokrum_core::{OverlaySource, PackRelativePath, SourceFailure, SourceFailureKind};

#[cfg(target_os = "linux")]
use std::fs::File;
#[cfg(target_os = "linux")]
use std::io::{Read, Take};
#[cfg(target_os = "linux")]
use std::os::fd::AsRawFd;
#[cfg(target_os = "linux")]
use std::os::unix::fs::MetadataExt;

/// Stable failure categories while establishing a local pack root.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalPackSourceError {
    UnsupportedPlatform,
    ProcUnavailable,
    RootUnavailable,
    RootSymbolicLink,
    RootNotDirectory,
}

impl fmt::Display for LocalPackSourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnsupportedPlatform => "local pack sources are supported on Linux only",
            Self::ProcUnavailable => "Linux procfs file-descriptor inspection is unavailable",
            Self::RootUnavailable => "pack root is unavailable",
            Self::RootSymbolicLink => "pack root must not be a symbolic link",
            Self::RootNotDirectory => "pack root must be a directory",
        })
    }
}

impl std::error::Error for LocalPackSourceError {}

/// A local source adapter rooted at one canonical directory.
#[derive(Clone, Debug)]
pub struct LocalPackSource {
    root: PathBuf,
    #[cfg(target_os = "linux")]
    root_device: u64,
    #[cfg(target_os = "linux")]
    root_inode: u64,
}

impl LocalPackSource {
    /// Establishes a canonical, non-symlink directory as the pack root.
    ///
    /// # Errors
    ///
    /// Returns [`LocalPackSourceError`] when the platform is unsupported or the
    /// root cannot be proven to be a real directory under the current namespace.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, LocalPackSourceError> {
        open_root(root.as_ref())
    }

    /// Returns the canonical root used by this adapter.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }
}

#[cfg(target_os = "linux")]
fn open_root(root: &Path) -> Result<LocalPackSource, LocalPackSourceError> {
    let lexical = fs::symlink_metadata(root).map_err(|_| LocalPackSourceError::RootUnavailable)?;
    if lexical.file_type().is_symlink() {
        return Err(LocalPackSourceError::RootSymbolicLink);
    }
    if !lexical.is_dir() {
        return Err(LocalPackSourceError::RootNotDirectory);
    }

    let canonical = fs::canonicalize(root).map_err(|_| LocalPackSourceError::RootUnavailable)?;
    let metadata = fs::metadata(&canonical).map_err(|_| LocalPackSourceError::RootUnavailable)?;
    if !metadata.is_dir() {
        return Err(LocalPackSourceError::RootNotDirectory);
    }

    let proc_descriptors =
        fs::metadata("/proc/self/fd").map_err(|_| LocalPackSourceError::ProcUnavailable)?;
    if !proc_descriptors.is_dir() {
        return Err(LocalPackSourceError::ProcUnavailable);
    }

    Ok(LocalPackSource {
        root: canonical,
        root_device: metadata.dev(),
        root_inode: metadata.ino(),
    })
}

#[cfg(not(target_os = "linux"))]
fn open_root(_root: &Path) -> Result<LocalPackSource, LocalPackSourceError> {
    Err(LocalPackSourceError::UnsupportedPlatform)
}

impl OverlaySource for LocalPackSource {
    fn load(
        &self,
        source: &PackRelativePath,
        maximum_bytes: usize,
    ) -> Result<Vec<u8>, SourceFailure> {
        load_source(self, source, maximum_bytes)
    }
}

#[cfg(target_os = "linux")]
fn load_source(
    adapter: &LocalPackSource,
    source: &PackRelativePath,
    maximum_bytes: usize,
) -> Result<Vec<u8>, SourceFailure> {
    verify_root(adapter, source)?;
    let candidate = adapter.root.join(source.as_str());
    verify_components(adapter, source, &candidate)?;

    let before = fs::metadata(&candidate).map_err(|error| map_io(source, &error))?;
    verify_file_metadata(adapter, source, &before)?;

    let file = File::open(&candidate).map_err(|error| map_io(source, &error))?;
    let opened = file.metadata().map_err(|error| map_io(source, &error))?;
    verify_file_metadata(adapter, source, &opened)?;
    if !same_identity(&before, &opened) {
        return Err(reject(source, SourceFailureKind::ChangedDuringRead));
    }

    let opened_path = fs::read_link(format!("/proc/self/fd/{}", file.as_raw_fd()))
        .map_err(|error| map_io(source, &error))?;
    if !opened_path.starts_with(&adapter.root) {
        return Err(reject(source, SourceFailureKind::RootEscape));
    }

    let limit = u64::try_from(maximum_bytes)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let mut reader: Take<File> = file.take(limit);
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|error| map_io(source, &error))?;
    if bytes.len() > maximum_bytes {
        return Err(reject(source, SourceFailureKind::TooLarge));
    }

    let file = reader.into_inner();
    let after = file.metadata().map_err(|error| map_io(source, &error))?;
    if !same_snapshot(&opened, &after) {
        return Err(reject(source, SourceFailureKind::ChangedDuringRead));
    }

    let current =
        fs::symlink_metadata(&candidate).map_err(|error| map_post_read_io(source, &error))?;
    if current.file_type().is_symlink() {
        return Err(reject(source, SourceFailureKind::SymbolicLink));
    }
    verify_file_metadata(adapter, source, &current)?;
    if !same_identity(&opened, &current) {
        return Err(reject(source, SourceFailureKind::ChangedDuringRead));
    }
    verify_root(adapter, source)?;

    Ok(bytes)
}

#[cfg(not(target_os = "linux"))]
fn load_source(
    _adapter: &LocalPackSource,
    source: &PackRelativePath,
    _maximum_bytes: usize,
) -> Result<Vec<u8>, SourceFailure> {
    Err(reject(source, SourceFailureKind::UnsupportedPlatform))
}

#[cfg(target_os = "linux")]
fn verify_root(adapter: &LocalPackSource, source: &PackRelativePath) -> Result<(), SourceFailure> {
    let metadata =
        fs::symlink_metadata(&adapter.root).map_err(|error| map_post_read_io(source, &error))?;
    if metadata.file_type().is_symlink() {
        return Err(reject(source, SourceFailureKind::SymbolicLink));
    }
    if !metadata.is_dir()
        || metadata.dev() != adapter.root_device
        || metadata.ino() != adapter.root_inode
    {
        return Err(reject(source, SourceFailureKind::ChangedDuringRead));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn verify_components(
    adapter: &LocalPackSource,
    source: &PackRelativePath,
    candidate: &Path,
) -> Result<(), SourceFailure> {
    let mut current = adapter.root.clone();
    let segment_count = source.as_str().split('/').count();

    for (index, segment) in source.as_str().split('/').enumerate() {
        current.push(segment);
        let metadata = fs::symlink_metadata(&current).map_err(|error| map_io(source, &error))?;
        if metadata.file_type().is_symlink() {
            return Err(reject(source, SourceFailureKind::SymbolicLink));
        }
        if metadata.dev() != adapter.root_device {
            return Err(reject(source, SourceFailureKind::DeviceBoundary));
        }
        if index + 1 < segment_count && !metadata.is_dir() {
            return Err(reject(source, SourceFailureKind::NotRegularFile));
        }
    }

    let canonical = fs::canonicalize(candidate).map_err(|error| map_io(source, &error))?;
    if !canonical.starts_with(&adapter.root) {
        return Err(reject(source, SourceFailureKind::RootEscape));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn verify_file_metadata(
    adapter: &LocalPackSource,
    source: &PackRelativePath,
    metadata: &fs::Metadata,
) -> Result<(), SourceFailure> {
    if !metadata.is_file() {
        return Err(reject(source, SourceFailureKind::NotRegularFile));
    }
    if metadata.dev() != adapter.root_device {
        return Err(reject(source, SourceFailureKind::DeviceBoundary));
    }
    if metadata.nlink() != 1 {
        return Err(reject(source, SourceFailureKind::HardLink));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn same_identity(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    left.dev() == right.dev() && left.ino() == right.ino()
}

#[cfg(target_os = "linux")]
fn same_snapshot(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    same_identity(left, right)
        && left.len() == right.len()
        && left.mtime() == right.mtime()
        && left.mtime_nsec() == right.mtime_nsec()
        && left.ctime() == right.ctime()
        && left.ctime_nsec() == right.ctime_nsec()
}

fn reject(source: &PackRelativePath, kind: SourceFailureKind) -> SourceFailure {
    SourceFailure::new(source.clone(), kind)
}

fn map_io(source: &PackRelativePath, error: &std::io::Error) -> SourceFailure {
    let kind = match error.kind() {
        std::io::ErrorKind::NotFound => SourceFailureKind::NotFound,
        std::io::ErrorKind::PermissionDenied => SourceFailureKind::PermissionDenied,
        _ => SourceFailureKind::Io,
    };
    reject(source, kind)
}

fn map_post_read_io(source: &PackRelativePath, error: &std::io::Error) -> SourceFailure {
    if error.kind() == std::io::ErrorKind::NotFound {
        reject(source, SourceFailureKind::ChangedDuringRead)
    } else {
        map_io(source, error)
    }
}
