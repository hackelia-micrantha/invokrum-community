//! Linux filesystem adapters for Invokrum acquisition.
//!
//! The local-directory loader establishes one canonical candidate root, proves
//! that the visible tree contains exactly the bundle-declared files/directories,
//! and returns owned bytes read from stable opened descriptors. It performs no
//! hashing, bundle-content verification, installation, archive parsing, network
//! access, environment lookup, credential lookup, or signature verification.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use invokrum_acquisition::CandidateFile;
use invokrum_distribution::{BundleManifest, BundlePath, MAX_BUNDLE_FILE_BYTES};

#[cfg(target_os = "linux")]
use invokrum_distribution::MAX_BUNDLE_FILES;
#[cfg(target_os = "linux")]
use std::collections::{BTreeMap, BTreeSet};
#[cfg(target_os = "linux")]
use std::fs::{self, File, Metadata, OpenOptions};
#[cfg(target_os = "linux")]
use std::io::{Read, Take};
#[cfg(target_os = "linux")]
use std::os::fd::AsRawFd;
#[cfg(target_os = "linux")]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

#[cfg(target_os = "linux")]
const O_NOFOLLOW: i32 = 0o400_000;
#[cfg(target_os = "linux")]
const MAX_DIRECTORY_DEPTH: usize = 64;
#[cfg(target_os = "linux")]
const MAX_VISIBLE_ENTRIES: usize = 16_384;

/// Linux local-directory candidate source.
#[derive(Clone, Debug)]
pub struct LinuxLocalCandidateSource {
    root: PathBuf,
}

impl LinuxLocalCandidateSource {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Validates the candidate root immediately and returns a reusable source.
    ///
    /// # Errors
    ///
    /// Returns [`LocalCandidateError`] when the root cannot satisfy the platform
    /// and containment preconditions.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, LocalCandidateError> {
        let source = Self::new(root);
        validate_source_root(&source)?;
        Ok(source)
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Reads exact owned candidate bytes from one fail-closed local root.
    ///
    /// # Errors
    ///
    /// Fails when the root or tree violates the Linux containment/link policy,
    /// contains undeclared or unsupported entries, misses declared files, crosses
    /// filesystem devices, changes identity during a read, or exceeds adapter
    /// traversal/resource limits.
    pub fn load(
        &self,
        manifest: &BundleManifest,
    ) -> Result<Vec<CandidateFile>, LocalCandidateError> {
        load_candidate(self, manifest, None)
    }

    /// Reads exact bundle bytes while allowing one explicit non-bundle metadata file.
    ///
    /// The additional file remains part of exact-tree validation but is not
    /// returned as a [`CandidateFile`]. The caller supplies its independent byte
    /// bound because installer-owned evidence is not a v1 bundle file.
    ///
    /// # Errors
    ///
    /// Fails closed under the same policy as [`Self::load`], and also fails when
    /// the allowed file is missing or exceeds `maximum_allowed_file_bytes`.
    pub fn load_with_allowed_file(
        &self,
        manifest: &BundleManifest,
        allowed_file: &BundlePath,
        maximum_allowed_file_bytes: u64,
    ) -> Result<Vec<CandidateFile>, LocalCandidateError> {
        load_candidate(
            self,
            manifest,
            Some((allowed_file, maximum_allowed_file_bytes)),
        )
    }

    /// Reads one explicit v1 bundle-sized file through the same descriptor-stability policy.
    ///
    /// # Errors
    ///
    /// Fails closed when the platform, root, path, or file identity is unsafe.
    pub fn read_exact(&self, path: &BundlePath) -> Result<Vec<u8>, LocalCandidateError> {
        read_candidate_file(self, path, MAX_BUNDLE_FILE_BYTES)
    }

    /// Reads one explicit file through the descriptor-stability policy with a caller-owned bound.
    ///
    /// This is intended for outer-layer metadata such as installation evidence whose
    /// format has a separate resource contract from v1 bundle content.
    ///
    /// # Errors
    ///
    /// Fails closed when the platform, root, path, or file identity is unsafe, or
    /// when the file exceeds `maximum_bytes`.
    pub fn read_exact_with_limit(
        &self,
        path: &BundlePath,
        maximum_bytes: u64,
    ) -> Result<Vec<u8>, LocalCandidateError> {
        read_candidate_file(self, path, maximum_bytes)
    }
}

/// Stable fail-closed Linux candidate-loading failures.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocalCandidateError {
    UnsupportedPlatform,
    ProcUnavailable,
    RootUnavailable,
    RootSymlink,
    RootNotDirectory,
    RootChanged,
    TreeChanged,
    TraversalLimitExceeded,
    InvalidEntryName,
    LogicalPathCollision,
    DeviceBoundary,
    SymbolicLink { path: BundlePath },
    HardLinkedFile { path: BundlePath },
    NonRegularFile { path: BundlePath },
    UndeclaredEntry { path: BundlePath },
    MissingDeclaredFile { path: BundlePath },
    FileTooLarge { path: BundlePath },
    OpenFailed { path: BundlePath },
    ReadFailed { path: BundlePath },
    FileChanged { path: BundlePath },
    RootEscape { path: BundlePath },
}

impl std::fmt::Display for LocalCandidateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedPlatform => {
                formatter.write_str("local candidate loading is supported on Linux only")
            }
            Self::ProcUnavailable => {
                formatter.write_str("Linux procfs file-descriptor inspection is unavailable")
            }
            Self::RootUnavailable => formatter.write_str("candidate root is unavailable"),
            Self::RootSymlink => formatter.write_str("candidate root must not be a symbolic link"),
            Self::RootNotDirectory => formatter.write_str("candidate root must be a directory"),
            Self::RootChanged => formatter.write_str("candidate root identity changed"),
            Self::TreeChanged => formatter.write_str("candidate tree changed during inspection"),
            Self::TraversalLimitExceeded => {
                formatter.write_str("candidate tree exceeds adapter traversal limits")
            }
            Self::InvalidEntryName => {
                formatter.write_str("candidate tree contains an unsupported entry name")
            }
            Self::LogicalPathCollision => {
                formatter.write_str("candidate tree contains colliding logical paths")
            }
            Self::DeviceBoundary => {
                formatter.write_str("candidate tree crosses a filesystem device boundary")
            }
            Self::SymbolicLink { path } => write!(formatter, "symbolic link is prohibited: {path}"),
            Self::HardLinkedFile { path } => {
                write!(formatter, "hard-linked file is prohibited: {path}")
            }
            Self::NonRegularFile { path } => {
                write!(formatter, "non-regular entry is prohibited: {path}")
            }
            Self::UndeclaredEntry { path } => {
                write!(formatter, "undeclared candidate entry: {path}")
            }
            Self::MissingDeclaredFile { path } => {
                write!(formatter, "declared candidate file is missing: {path}")
            }
            Self::FileTooLarge { path } => {
                write!(
                    formatter,
                    "candidate file exceeds permitted size limit: {path}"
                )
            }
            Self::OpenFailed { path } => {
                write!(
                    formatter,
                    "candidate file could not be opened safely: {path}"
                )
            }
            Self::ReadFailed { path } => {
                write!(formatter, "candidate file could not be read safely: {path}")
            }
            Self::FileChanged { path } => {
                write!(formatter, "candidate file changed during read: {path}")
            }
            Self::RootEscape { path } => {
                write!(
                    formatter,
                    "opened candidate file escaped the pinned root: {path}"
                )
            }
        }
    }
}

impl std::error::Error for LocalCandidateError {}

#[cfg(target_os = "linux")]
#[derive(Clone, Debug)]
struct PinnedRoot {
    canonical: PathBuf,
    device: u64,
    inode: u64,
}

#[cfg(target_os = "linux")]
impl PinnedRoot {
    fn open(path: &Path) -> Result<Self, LocalCandidateError> {
        let lexical =
            fs::symlink_metadata(path).map_err(|_| LocalCandidateError::RootUnavailable)?;
        if lexical.file_type().is_symlink() {
            return Err(LocalCandidateError::RootSymlink);
        }
        if !lexical.is_dir() {
            return Err(LocalCandidateError::RootNotDirectory);
        }

        let canonical = fs::canonicalize(path).map_err(|_| LocalCandidateError::RootUnavailable)?;
        let metadata =
            fs::metadata(&canonical).map_err(|_| LocalCandidateError::RootUnavailable)?;
        if !metadata.is_dir() {
            return Err(LocalCandidateError::RootNotDirectory);
        }
        if lexical.dev() != metadata.dev() || lexical.ino() != metadata.ino() {
            return Err(LocalCandidateError::RootChanged);
        }

        let proc_descriptors =
            fs::metadata("/proc/self/fd").map_err(|_| LocalCandidateError::ProcUnavailable)?;
        if !proc_descriptors.is_dir() {
            return Err(LocalCandidateError::ProcUnavailable);
        }

        Ok(Self {
            canonical,
            device: metadata.dev(),
            inode: metadata.ino(),
        })
    }

    fn revalidate(&self) -> Result<(), LocalCandidateError> {
        let metadata =
            fs::symlink_metadata(&self.canonical).map_err(|_| LocalCandidateError::RootChanged)?;
        if metadata.file_type().is_symlink()
            || !metadata.is_dir()
            || metadata.dev() != self.device
            || metadata.ino() != self.inode
        {
            return Err(LocalCandidateError::RootChanged);
        }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
#[derive(Clone, Debug)]
struct ExpectedTree {
    files: BTreeSet<BundlePath>,
    directories: BTreeSet<String>,
    allowed_file: Option<BundlePath>,
    allowed_file_maximum_bytes: Option<u64>,
}

#[cfg(target_os = "linux")]
impl ExpectedTree {
    fn from_manifest(
        manifest: &BundleManifest,
        allowed_file: Option<(&BundlePath, u64)>,
    ) -> Result<Self, LocalCandidateError> {
        if manifest.files().len() > MAX_BUNDLE_FILES {
            return Err(LocalCandidateError::TraversalLimitExceeded);
        }

        let mut files = BTreeSet::new();
        let mut directories = BTreeSet::new();
        for file in manifest.files() {
            add_expected_path(file.path(), &mut files, &mut directories)?;
        }
        if let Some((path, _)) = allowed_file {
            add_expected_path(path, &mut files, &mut directories)?;
        }
        Ok(Self {
            files,
            directories,
            allowed_file: allowed_file.map(|(path, _)| path.clone()),
            allowed_file_maximum_bytes: allowed_file.map(|(_, maximum_bytes)| maximum_bytes),
        })
    }

    fn maximum_bytes_for(&self, path: &BundlePath) -> u64 {
        if self.allowed_file.as_ref() == Some(path) {
            self.allowed_file_maximum_bytes
                .unwrap_or(MAX_BUNDLE_FILE_BYTES)
        } else {
            MAX_BUNDLE_FILE_BYTES
        }
    }
}

#[cfg(target_os = "linux")]
fn add_expected_path(
    path: &BundlePath,
    files: &mut BTreeSet<BundlePath>,
    directories: &mut BTreeSet<String>,
) -> Result<(), LocalCandidateError> {
    files.insert(path.clone());
    let components = path.as_str().split('/').collect::<Vec<_>>();
    if components.len() > MAX_DIRECTORY_DEPTH + 1 {
        return Err(LocalCandidateError::TraversalLimitExceeded);
    }
    for end in 1..components.len() {
        directories.insert(components[..end].join("/"));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
#[derive(Default)]
struct ObservedTree {
    files: BTreeSet<BundlePath>,
    logical_paths: BTreeMap<String, BundlePath>,
    entries: usize,
}

#[cfg(target_os = "linux")]
fn validate_source_root(source: &LinuxLocalCandidateSource) -> Result<(), LocalCandidateError> {
    PinnedRoot::open(&source.root).map(|_| ())
}

#[cfg(not(target_os = "linux"))]
fn validate_source_root(_source: &LinuxLocalCandidateSource) -> Result<(), LocalCandidateError> {
    Err(LocalCandidateError::UnsupportedPlatform)
}

#[cfg(target_os = "linux")]
fn load_candidate(
    source: &LinuxLocalCandidateSource,
    manifest: &BundleManifest,
    allowed_file: Option<(&BundlePath, u64)>,
) -> Result<Vec<CandidateFile>, LocalCandidateError> {
    let pinned = PinnedRoot::open(&source.root)?;
    let expected = ExpectedTree::from_manifest(manifest, allowed_file)?;
    let observed = inspect_tree(&pinned, &expected)?;

    for file in manifest.files() {
        if !observed.files.contains(file.path()) {
            return Err(LocalCandidateError::MissingDeclaredFile {
                path: file.path().clone(),
            });
        }
    }
    if let Some(path) = &expected.allowed_file {
        if !observed.files.contains(path) {
            return Err(LocalCandidateError::MissingDeclaredFile { path: path.clone() });
        }
    }

    let mut candidates = Vec::with_capacity(manifest.files().len());
    for file in manifest.files() {
        pinned.revalidate()?;
        let bytes = read_declared_file(&pinned, file.path(), MAX_BUNDLE_FILE_BYTES)?;
        let candidate = CandidateFile::new(file.path().clone(), bytes).map_err(|_| {
            LocalCandidateError::FileTooLarge {
                path: file.path().clone(),
            }
        })?;
        candidates.push(candidate);
    }
    pinned.revalidate()?;
    Ok(candidates)
}

#[cfg(not(target_os = "linux"))]
fn load_candidate(
    _source: &LinuxLocalCandidateSource,
    _manifest: &BundleManifest,
    _allowed_file: Option<(&BundlePath, u64)>,
) -> Result<Vec<CandidateFile>, LocalCandidateError> {
    Err(LocalCandidateError::UnsupportedPlatform)
}

#[cfg(target_os = "linux")]
fn read_candidate_file(
    source: &LinuxLocalCandidateSource,
    path: &BundlePath,
    maximum_bytes: u64,
) -> Result<Vec<u8>, LocalCandidateError> {
    let pinned = PinnedRoot::open(&source.root)?;
    read_declared_file(&pinned, path, maximum_bytes)
}

#[cfg(not(target_os = "linux"))]
fn read_candidate_file(
    _source: &LinuxLocalCandidateSource,
    _path: &BundlePath,
    _maximum_bytes: u64,
) -> Result<Vec<u8>, LocalCandidateError> {
    Err(LocalCandidateError::UnsupportedPlatform)
}

#[cfg(target_os = "linux")]
fn inspect_tree(
    root: &PinnedRoot,
    expected: &ExpectedTree,
) -> Result<ObservedTree, LocalCandidateError> {
    let mut observed = ObservedTree::default();
    inspect_directory(root, expected, Path::new(""), 0, &mut observed)?;
    root.revalidate()?;
    Ok(observed)
}

#[cfg(target_os = "linux")]
fn inspect_directory(
    root: &PinnedRoot,
    expected: &ExpectedTree,
    relative_directory: &Path,
    depth: usize,
    observed: &mut ObservedTree,
) -> Result<(), LocalCandidateError> {
    if depth > MAX_DIRECTORY_DEPTH {
        return Err(LocalCandidateError::TraversalLimitExceeded);
    }

    let absolute = root.canonical.join(relative_directory);
    let before = fs::symlink_metadata(&absolute).map_err(|_| LocalCandidateError::TreeChanged)?;
    if before.file_type().is_symlink() || !before.is_dir() || before.dev() != root.device {
        return Err(LocalCandidateError::TreeChanged);
    }

    let directory = File::open(&absolute).map_err(|_| LocalCandidateError::TreeChanged)?;
    let opened = directory
        .metadata()
        .map_err(|_| LocalCandidateError::TreeChanged)?;
    if !opened.is_dir() || opened.dev() != root.device || !same_file_identity(&before, &opened) {
        return Err(LocalCandidateError::TreeChanged);
    }
    validate_opened_directory(root, relative_directory, &directory)?;

    let descriptor = PathBuf::from(format!("/proc/self/fd/{}", directory.as_raw_fd()));
    let mut entries = Vec::new();
    for entry in fs::read_dir(&descriptor).map_err(|_| LocalCandidateError::TreeChanged)? {
        observed.entries = observed
            .entries
            .checked_add(1)
            .ok_or(LocalCandidateError::TraversalLimitExceeded)?;
        if observed.entries > MAX_VISIBLE_ENTRIES {
            return Err(LocalCandidateError::TraversalLimitExceeded);
        }
        entries.push(entry.map_err(|_| LocalCandidateError::TreeChanged)?);
    }
    entries.sort_by_key(std::fs::DirEntry::file_name);

    for entry in entries {
        let name = entry.file_name();
        let name = name
            .to_str()
            .filter(|value| value.is_ascii())
            .ok_or(LocalCandidateError::InvalidEntryName)?;
        let relative = if relative_directory.as_os_str().is_empty() {
            PathBuf::from(name)
        } else {
            relative_directory.join(name)
        };
        let logical = path_to_bundle_string(&relative)?;
        record_logical_path(observed, &logical)?;
        let entry_path = entry.path();
        let metadata =
            fs::symlink_metadata(&entry_path).map_err(|_| LocalCandidateError::TreeChanged)?;

        if metadata.file_type().is_symlink() {
            return Err(LocalCandidateError::SymbolicLink { path: logical });
        }
        if metadata.dev() != root.device {
            return Err(LocalCandidateError::DeviceBoundary);
        }

        if metadata.is_dir() {
            if !expected.directories.contains(logical.as_str()) {
                return Err(LocalCandidateError::UndeclaredEntry { path: logical });
            }
            inspect_directory(root, expected, &relative, depth + 1, observed)?;
            continue;
        }

        if !metadata.is_file() {
            return Err(LocalCandidateError::NonRegularFile { path: logical });
        }
        if metadata.nlink() != 1 {
            return Err(LocalCandidateError::HardLinkedFile { path: logical });
        }
        if !expected.files.contains(&logical) {
            return Err(LocalCandidateError::UndeclaredEntry { path: logical });
        }
        if metadata.len() > expected.maximum_bytes_for(&logical) {
            return Err(LocalCandidateError::FileTooLarge { path: logical });
        }
        observed.files.insert(logical);
    }

    let after = directory
        .metadata()
        .map_err(|_| LocalCandidateError::TreeChanged)?;
    if !same_directory_snapshot(&opened, &after) {
        return Err(LocalCandidateError::TreeChanged);
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn record_logical_path(
    observed: &mut ObservedTree,
    path: &BundlePath,
) -> Result<(), LocalCandidateError> {
    let key = path.as_str().to_ascii_lowercase();
    if let Some(existing) = observed.logical_paths.get(&key) {
        if existing != path {
            return Err(LocalCandidateError::LogicalPathCollision);
        }
    } else {
        observed.logical_paths.insert(key, path.clone());
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn validate_opened_directory(
    root: &PinnedRoot,
    relative: &Path,
    directory: &File,
) -> Result<(), LocalCandidateError> {
    let target = fs::read_link(format!("/proc/self/fd/{}", directory.as_raw_fd()))
        .map_err(|_| LocalCandidateError::TreeChanged)?;
    let expected = root.canonical.join(relative);
    if target != expected || !target.starts_with(&root.canonical) {
        return Err(LocalCandidateError::TreeChanged);
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn read_declared_file(
    root: &PinnedRoot,
    relative: &BundlePath,
    maximum_bytes: u64,
) -> Result<Vec<u8>, LocalCandidateError> {
    validate_components(root, relative)?;
    let absolute = root.canonical.join(relative.as_str());
    let before =
        fs::symlink_metadata(&absolute).map_err(|_| LocalCandidateError::MissingDeclaredFile {
            path: relative.clone(),
        })?;
    validate_regular_metadata(root, relative, &before, maximum_bytes)?;

    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(O_NOFOLLOW)
        .open(&absolute)
        .map_err(|_| LocalCandidateError::OpenFailed {
            path: relative.clone(),
        })?;
    let opened = file
        .metadata()
        .map_err(|_| LocalCandidateError::TreeChanged)?;
    if !same_file_identity(&before, &opened) {
        return Err(LocalCandidateError::FileChanged {
            path: relative.clone(),
        });
    }
    validate_regular_metadata(root, relative, &opened, maximum_bytes)?;
    validate_opened_target(root, relative, &file)?;

    let mut reader: Take<&mut File> = file.by_ref().take(maximum_bytes.saturating_add(1));
    let mut bytes = Vec::with_capacity(
        usize::try_from(opened.len())
            .unwrap_or(0)
            .min(usize::try_from(maximum_bytes).unwrap_or(usize::MAX)),
    );
    reader
        .read_to_end(&mut bytes)
        .map_err(|_| LocalCandidateError::ReadFailed {
            path: relative.clone(),
        })?;
    if u64::try_from(bytes.len()).map_or(true, |length| length > maximum_bytes) {
        return Err(LocalCandidateError::FileTooLarge {
            path: relative.clone(),
        });
    }

    let after = file
        .metadata()
        .map_err(|_| LocalCandidateError::TreeChanged)?;
    if !same_stable_metadata(&opened, &after)
        || after.len() != u64::try_from(bytes.len()).unwrap_or(u64::MAX)
    {
        return Err(LocalCandidateError::FileChanged {
            path: relative.clone(),
        });
    }

    let path_after =
        fs::symlink_metadata(&absolute).map_err(|_| LocalCandidateError::FileChanged {
            path: relative.clone(),
        })?;
    if !same_file_identity(&after, &path_after) || path_after.file_type().is_symlink() {
        return Err(LocalCandidateError::FileChanged {
            path: relative.clone(),
        });
    }
    root.revalidate()?;
    Ok(bytes)
}

#[cfg(target_os = "linux")]
fn validate_components(
    root: &PinnedRoot,
    relative: &BundlePath,
) -> Result<(), LocalCandidateError> {
    let components = relative.as_str().split('/').collect::<Vec<_>>();
    if components.len() > MAX_DIRECTORY_DEPTH + 1 {
        return Err(LocalCandidateError::TraversalLimitExceeded);
    }

    let mut current = root.canonical.clone();
    for component in &components[..components.len().saturating_sub(1)] {
        current.push(component);
        let metadata = fs::symlink_metadata(&current).map_err(|_| {
            LocalCandidateError::MissingDeclaredFile {
                path: relative.clone(),
            }
        })?;
        if metadata.file_type().is_symlink() {
            return Err(LocalCandidateError::SymbolicLink {
                path: relative.clone(),
            });
        }
        if !metadata.is_dir() {
            return Err(LocalCandidateError::NonRegularFile {
                path: relative.clone(),
            });
        }
        if metadata.dev() != root.device {
            return Err(LocalCandidateError::DeviceBoundary);
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn validate_regular_metadata(
    root: &PinnedRoot,
    relative: &BundlePath,
    metadata: &Metadata,
    maximum_bytes: u64,
) -> Result<(), LocalCandidateError> {
    if !metadata.is_file() {
        return Err(LocalCandidateError::NonRegularFile {
            path: relative.clone(),
        });
    }
    if metadata.nlink() != 1 {
        return Err(LocalCandidateError::HardLinkedFile {
            path: relative.clone(),
        });
    }
    if metadata.dev() != root.device {
        return Err(LocalCandidateError::DeviceBoundary);
    }
    if metadata.len() > maximum_bytes {
        return Err(LocalCandidateError::FileTooLarge {
            path: relative.clone(),
        });
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn validate_opened_target(
    root: &PinnedRoot,
    relative: &BundlePath,
    file: &File,
) -> Result<(), LocalCandidateError> {
    let proc_path = PathBuf::from(format!("/proc/self/fd/{}", file.as_raw_fd()));
    let target = fs::read_link(proc_path).map_err(|_| LocalCandidateError::TreeChanged)?;
    let expected = root.canonical.join(relative.as_str());
    if target != expected || !target.starts_with(&root.canonical) {
        return Err(LocalCandidateError::RootEscape {
            path: relative.clone(),
        });
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn path_to_bundle_string(path: &Path) -> Result<BundlePath, LocalCandidateError> {
    let text = path
        .to_str()
        .filter(|value| value.is_ascii())
        .ok_or(LocalCandidateError::InvalidEntryName)?;
    BundlePath::parse(text.to_owned()).map_err(|_| LocalCandidateError::InvalidEntryName)
}

#[cfg(target_os = "linux")]
fn same_file_identity(left: &Metadata, right: &Metadata) -> bool {
    left.dev() == right.dev() && left.ino() == right.ino() && left.mode() == right.mode()
}

#[cfg(target_os = "linux")]
fn same_stable_metadata(left: &Metadata, right: &Metadata) -> bool {
    same_file_identity(left, right)
        && left.len() == right.len()
        && left.mtime() == right.mtime()
        && left.mtime_nsec() == right.mtime_nsec()
        && left.ctime() == right.ctime()
        && left.ctime_nsec() == right.ctime_nsec()
        && left.nlink() == right.nlink()
}

#[cfg(target_os = "linux")]
fn same_directory_snapshot(left: &Metadata, right: &Metadata) -> bool {
    left.dev() == right.dev()
        && left.ino() == right.ino()
        && left.mode() == right.mode()
        && left.mtime() == right.mtime()
        && left.mtime_nsec() == right.mtime_nsec()
        && left.ctime() == right.ctime()
        && left.ctime_nsec() == right.ctime_nsec()
}

#[cfg(all(test, not(target_os = "linux")))]
mod tests {
    use super::*;

    #[test]
    fn unsupported_platform_is_a_stable_runtime_error() {
        assert!(matches!(
            LinuxLocalCandidateSource::open("."),
            Err(LocalCandidateError::UnsupportedPlatform)
        ));
    }
}
