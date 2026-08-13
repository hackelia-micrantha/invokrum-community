use std::fmt;
use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
use invokrum_acquisition::verify_candidate;
use invokrum_acquisition::{CandidateVerificationError, VerifiedBundle};
use invokrum_acquisition_linux::{LinuxLocalCandidateSource, LocalCandidateError};
use invokrum_digest::sha256_lower_hex;
use invokrum_distribution::{
    BUNDLE_FORMAT, BundleFile, BundleLimits, BundleManifest, BundlePath, SHA256_ALGORITHM,
    Sha256Digest,
};
use invokrum_install::{
    AuthenticatedVerifiedBundleStore, AuthorizedPublisher, VerifiedBundleStore,
};
use serde::Serialize;

#[cfg(target_os = "linux")]
use std::fs::{self, DirBuilder, File, OpenOptions};
#[cfg(target_os = "linux")]
use std::io::Write;
#[cfg(target_os = "linux")]
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};

pub const INSTALLATION_RECORD_FORMAT: &str = "invokrum.installation/v1";
pub const AUTHENTICATED_INSTALLATION_RECORD_FORMAT: &str = "invokrum.installation/v2";
pub const INSTALLATION_RECORD_PATH: &str = ".invokrum-installation.json";
pub const INSTALLED_TREE_DIGEST_FORMAT: &str = "invokrum.installed-tree/v1";
/// Hard byte bound for canonical installer-owned evidence.
///
/// This is intentionally independent from the v1 per-bundle-file limit because
/// one installation record deterministically summarizes the complete bounded
/// bundle file set and may therefore exceed any individual bundle file.
pub const MAX_INSTALLATION_RECORD_BYTES: u64 = 2_097_152;
const PUBLISHER_AUTHENTICATION_NOT_PROVIDED: &str = "not-provided";
const PUBLISHER_AUTHENTICATION_VERIFIED_AND_AUTHORIZED: &str = "verified-and-authorized";
const MAX_STAGING_ATTEMPTS: u32 = 1024;
const PRIVATE_DIRECTORY_MODE: u32 = 0o700;
const PRIVATE_FILE_MODE: u32 = 0o600;

/// Stable receipt for one verified content-addressed installation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledBundle {
    subject: Sha256Digest,
    root: PathBuf,
    record: PathBuf,
    reused: bool,
}

impl InstalledBundle {
    #[must_use]
    pub const fn subject(&self) -> &Sha256Digest {
        &self.subject
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    #[must_use]
    pub fn record(&self) -> &Path {
        &self.record
    }

    #[must_use]
    pub const fn reused(&self) -> bool {
        self.reused
    }
}

/// Stable failure categories for the Linux content-addressed install store.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LinuxInstallStoreError {
    UnsupportedPlatform,
    ProcUnavailable,
    RootUnavailable,
    RootSymbolicLink,
    RootNotDirectory,
    StoreChanged,
    StorePathUnsafe,
    ReservedMetadataPath,
    SubjectLocked,
    StagingUnavailable,
    MaterializationConflict { path: BundlePath },
    ManifestReconstruction,
    RecordSerialization,
    RecordTooLarge,
    PublisherSubjectMismatch,
    StageVerification(LocalCandidateError),
    ContentVerification(CandidateVerificationError),
    RecordMismatch,
    ExistingInstallationMismatch,
    Io,
}

impl fmt::Display for LinuxInstallStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform => {
                formatter.write_str("local installation is supported on Linux only")
            }
            Self::ProcUnavailable => {
                formatter.write_str("Linux procfs file-descriptor inspection is unavailable")
            }
            Self::RootUnavailable => formatter.write_str("installation-store root is unavailable"),
            Self::RootSymbolicLink => {
                formatter.write_str("installation-store root must not be a symbolic link")
            }
            Self::RootNotDirectory => {
                formatter.write_str("installation-store root must be a directory")
            }
            Self::StoreChanged => {
                formatter.write_str("installation-store root changed during installation")
            }
            Self::StorePathUnsafe => {
                formatter.write_str("installation-store path or permissions are unsafe")
            }
            Self::ReservedMetadataPath => formatter
                .write_str("bundle collides with the reserved installation-record path"),
            Self::SubjectLocked => {
                formatter.write_str("bundle subject is already being installed")
            }
            Self::StagingUnavailable => {
                formatter.write_str("no private quarantine directory could be created")
            }
            Self::MaterializationConflict { path } => write!(
                formatter,
                "verified bundle paths conflict during materialization: {path}"
            ),
            Self::ManifestReconstruction => formatter
                .write_str("verified bundle could not be reconstructed as a bundle manifest"),
            Self::RecordSerialization => {
                formatter.write_str("installation record could not be serialized")
            }
            Self::RecordTooLarge => {
                formatter.write_str("installation record exceeds its bounded evidence limit")
            }
            Self::PublisherSubjectMismatch => formatter.write_str(
                "authorized publisher subject does not match verified bundle subject",
            ),
            Self::StageVerification(error) => {
                write!(formatter, "staged installation failed verification: {error}")
            }
            Self::ContentVerification(error) => {
                write!(formatter, "staged bundle content failed verification: {error}")
            }
            Self::RecordMismatch => {
                formatter.write_str("installation record differs from expected evidence")
            }
            Self::ExistingInstallationMismatch => formatter.write_str(
                "existing subject installation is missing, unsafe, or does not match the verified bundle",
            ),
            Self::Io => formatter.write_str("installation-store filesystem operation failed"),
        }
    }
}

impl std::error::Error for LinuxInstallStoreError {}

/// Linux store rooted at one protected directory supplied by the host.
#[derive(Clone, Debug)]
pub struct LinuxInstallStore {
    root: PathBuf,
    #[cfg(target_os = "linux")]
    root_device: u64,
    #[cfg(target_os = "linux")]
    root_inode: u64,
}

impl LinuxInstallStore {
    /// Opens an existing protected store root and creates private internal directories.
    ///
    /// # Errors
    ///
    /// Fails when the platform or root cannot satisfy the Linux store preconditions.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, LinuxInstallStoreError> {
        open_store(root.as_ref())
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    #[must_use]
    pub fn subject_root(&self, subject: &Sha256Digest) -> PathBuf {
        self.root.join("sha256").join(subject.as_str())
    }
}

impl VerifiedBundleStore for LinuxInstallStore {
    type Receipt = InstalledBundle;
    type Error = LinuxInstallStoreError;

    fn install(&self, bundle: &VerifiedBundle) -> Result<Self::Receipt, Self::Error> {
        install_bundle_with_publisher(self, bundle, None)
    }
}

impl AuthenticatedVerifiedBundleStore for LinuxInstallStore {
    type Receipt = InstalledBundle;
    type Error = LinuxInstallStoreError;

    fn install_authenticated(
        &self,
        bundle: &VerifiedBundle,
        publisher: &AuthorizedPublisher,
    ) -> Result<Self::Receipt, Self::Error> {
        install_bundle_with_publisher(self, bundle, Some(publisher))
    }
}

#[cfg(target_os = "linux")]
struct SubjectLock {
    path: PathBuf,
    directory: PathBuf,
    _file: File,
}

#[cfg(target_os = "linux")]
impl Drop for SubjectLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
        let _ = sync_directory(&self.directory);
    }
}

#[cfg(target_os = "linux")]
fn open_store(root: &Path) -> Result<LinuxInstallStore, LinuxInstallStoreError> {
    let lexical =
        fs::symlink_metadata(root).map_err(|_| LinuxInstallStoreError::RootUnavailable)?;
    if lexical.file_type().is_symlink() {
        return Err(LinuxInstallStoreError::RootSymbolicLink);
    }
    if !lexical.is_dir() {
        return Err(LinuxInstallStoreError::RootNotDirectory);
    }

    let canonical = fs::canonicalize(root).map_err(|_| LinuxInstallStoreError::RootUnavailable)?;
    let metadata = fs::metadata(&canonical).map_err(|_| LinuxInstallStoreError::RootUnavailable)?;
    if !metadata.is_dir() || !protected_root_mode(metadata.mode()) {
        return Err(LinuxInstallStoreError::StorePathUnsafe);
    }

    let proc_descriptors =
        fs::metadata("/proc/self/fd").map_err(|_| LinuxInstallStoreError::ProcUnavailable)?;
    if !proc_descriptors.is_dir() {
        return Err(LinuxInstallStoreError::ProcUnavailable);
    }

    let store = LinuxInstallStore {
        root: canonical,
        root_device: metadata.dev(),
        root_inode: metadata.ino(),
    };
    ensure_store_root(&store)?;
    ensure_private_directory(&store, &store.root.join(".staging"))?;
    ensure_private_directory(&store, &store.root.join(".locks"))?;
    ensure_private_directory(&store, &store.root.join("sha256"))?;
    Ok(store)
}

#[cfg(not(target_os = "linux"))]
fn open_store(_root: &Path) -> Result<LinuxInstallStore, LinuxInstallStoreError> {
    Err(LinuxInstallStoreError::UnsupportedPlatform)
}

#[cfg(target_os = "linux")]
fn install_bundle_with_publisher(
    store: &LinuxInstallStore,
    bundle: &VerifiedBundle,
    publisher: Option<&AuthorizedPublisher>,
) -> Result<InstalledBundle, LinuxInstallStoreError> {
    if publisher.is_some_and(|publisher| publisher.subject() != bundle.subject()) {
        return Err(LinuxInstallStoreError::PublisherSubjectMismatch);
    }

    ensure_store_root(store)?;
    ensure_private_directory(store, &store.root.join(".staging"))?;
    ensure_private_directory(store, &store.root.join(".locks"))?;
    ensure_private_directory(store, &store.root.join("sha256"))?;
    reject_reserved_paths(bundle)?;
    let _subject_lock = acquire_subject_lock(store, bundle.subject())?;

    let final_root = store.subject_root(bundle.subject());
    if path_exists_no_follow(&final_root)? {
        verify_existing_installation(store, &final_root, bundle, publisher)?;
        return Ok(receipt(bundle, final_root, true));
    }

    let staging = create_staging(store, bundle.subject())?;
    let result = stage_bundle(store, &staging, bundle, publisher).and_then(|()| {
        ensure_store_root(store)?;
        ensure_private_directory(store, &store.root.join("sha256"))?;
        match fs::rename(&staging, &final_root) {
            Ok(()) => {
                sync_directory(&store.root.join("sha256"))?;
                verify_existing_installation(store, &final_root, bundle, publisher)?;
                Ok(receipt(bundle, final_root.clone(), false))
            }
            Err(_) if path_exists_no_follow(&final_root)? => {
                cleanup_staging(&staging);
                verify_existing_installation(store, &final_root, bundle, publisher)?;
                Ok(receipt(bundle, final_root.clone(), true))
            }
            Err(_) => Err(LinuxInstallStoreError::Io),
        }
    });

    if result.is_err() {
        cleanup_staging(&staging);
    }
    result
}

#[cfg(not(target_os = "linux"))]
fn install_bundle_with_publisher(
    _store: &LinuxInstallStore,
    _bundle: &VerifiedBundle,
    _publisher: Option<&AuthorizedPublisher>,
) -> Result<InstalledBundle, LinuxInstallStoreError> {
    Err(LinuxInstallStoreError::UnsupportedPlatform)
}

#[cfg(target_os = "linux")]
fn stage_bundle(
    store: &LinuxInstallStore,
    staging: &Path,
    bundle: &VerifiedBundle,
    publisher: Option<&AuthorizedPublisher>,
) -> Result<(), LinuxInstallStoreError> {
    verify_private_directory(store, staging)?;
    for file in bundle.files() {
        materialize_file(store, staging, file.path(), file.bytes())?;
    }
    sync_directory(staging)?;

    let manifest = manifest_from_verified(bundle)?;
    let source = LinuxLocalCandidateSource::open(staging.to_path_buf())
        .map_err(LinuxInstallStoreError::StageVerification)?;
    let candidates = source
        .load(&manifest)
        .map_err(LinuxInstallStoreError::StageVerification)?;
    verify_candidate(&manifest, bundle.subject(), bundle.subject(), candidates)
        .map_err(LinuxInstallStoreError::ContentVerification)?;

    let record_path = installation_record_path();
    let record = installation_record_bytes(bundle, publisher)?;
    materialize_file(store, staging, &record_path, &record)?;
    let persisted = source
        .read_exact_with_limit(&record_path, MAX_INSTALLATION_RECORD_BYTES)
        .map_err(LinuxInstallStoreError::StageVerification)?;
    if persisted != record {
        return Err(LinuxInstallStoreError::RecordMismatch);
    }
    verify_installation_permissions(store, staging, bundle)?;
    sync_directory(staging)?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn verify_existing_installation(
    store: &LinuxInstallStore,
    root: &Path,
    bundle: &VerifiedBundle,
    publisher: Option<&AuthorizedPublisher>,
) -> Result<(), LinuxInstallStoreError> {
    verify_installation_permissions(store, root, bundle)
        .map_err(|_| LinuxInstallStoreError::ExistingInstallationMismatch)?;

    let manifest = manifest_from_verified(bundle)
        .map_err(|_| LinuxInstallStoreError::ExistingInstallationMismatch)?;
    let record_path = installation_record_path();
    let source = LinuxLocalCandidateSource::open(root.to_path_buf())
        .map_err(|_| LinuxInstallStoreError::ExistingInstallationMismatch)?;
    let candidates = source
        .load_with_allowed_file(&manifest, &record_path, MAX_INSTALLATION_RECORD_BYTES)
        .map_err(|_| LinuxInstallStoreError::ExistingInstallationMismatch)?;
    verify_candidate(&manifest, bundle.subject(), bundle.subject(), candidates)
        .map_err(|_| LinuxInstallStoreError::ExistingInstallationMismatch)?;

    let actual_record = source
        .read_exact_with_limit(&record_path, MAX_INSTALLATION_RECORD_BYTES)
        .map_err(|_| LinuxInstallStoreError::ExistingInstallationMismatch)?;
    let expected_record = installation_record_bytes(bundle, publisher)
        .map_err(|_| LinuxInstallStoreError::ExistingInstallationMismatch)?;
    if actual_record != expected_record {
        return Err(LinuxInstallStoreError::ExistingInstallationMismatch);
    }

    verify_installation_permissions(store, root, bundle)
        .map_err(|_| LinuxInstallStoreError::ExistingInstallationMismatch)?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn materialize_file(
    store: &LinuxInstallStore,
    root: &Path,
    path: &BundlePath,
    bytes: &[u8],
) -> Result<(), LinuxInstallStoreError> {
    let destination = root.join(path.as_str());
    if let Some(parent) = destination.parent() {
        ensure_relative_directories(store, root, parent)?;
    }

    let mut options = OpenOptions::new();
    options.write(true).create_new(true).mode(PRIVATE_FILE_MODE);
    let mut file = options
        .open(&destination)
        .map_err(|_| LinuxInstallStoreError::MaterializationConflict { path: path.clone() })?;
    file.write_all(bytes)
        .map_err(|_| LinuxInstallStoreError::Io)?;
    file.sync_all().map_err(|_| LinuxInstallStoreError::Io)?;
    fs::set_permissions(&destination, fs::Permissions::from_mode(PRIVATE_FILE_MODE))
        .map_err(|_| LinuxInstallStoreError::Io)?;

    let metadata = file.metadata().map_err(|_| LinuxInstallStoreError::Io)?;
    if !safe_private_file(&metadata, store.root_device) {
        return Err(LinuxInstallStoreError::StorePathUnsafe);
    }
    if let Some(parent) = destination.parent() {
        sync_directory(parent)?;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn ensure_relative_directories(
    store: &LinuxInstallStore,
    root: &Path,
    parent: &Path,
) -> Result<(), LinuxInstallStoreError> {
    let relative = parent
        .strip_prefix(root)
        .map_err(|_| LinuxInstallStoreError::StorePathUnsafe)?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component);
        ensure_private_directory(store, &current)?;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn ensure_private_directory(
    store: &LinuxInstallStore,
    directory: &Path,
) -> Result<(), LinuxInstallStoreError> {
    match fs::symlink_metadata(directory) {
        Ok(metadata) => {
            if !safe_private_directory(&metadata, store.root_device) {
                return Err(LinuxInstallStoreError::StorePathUnsafe);
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let mut builder = DirBuilder::new();
            builder.mode(PRIVATE_DIRECTORY_MODE);
            builder
                .create(directory)
                .map_err(|_| LinuxInstallStoreError::Io)?;
            fs::set_permissions(
                directory,
                fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE),
            )
            .map_err(|_| LinuxInstallStoreError::Io)?;
            let metadata =
                fs::symlink_metadata(directory).map_err(|_| LinuxInstallStoreError::Io)?;
            if !safe_private_directory(&metadata, store.root_device) {
                return Err(LinuxInstallStoreError::StorePathUnsafe);
            }
            if let Some(parent) = directory.parent() {
                sync_directory(parent)?;
            }
        }
        Err(_) => return Err(LinuxInstallStoreError::Io),
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn verify_private_directory(
    store: &LinuxInstallStore,
    directory: &Path,
) -> Result<(), LinuxInstallStoreError> {
    let metadata =
        fs::symlink_metadata(directory).map_err(|_| LinuxInstallStoreError::StorePathUnsafe)?;
    if safe_private_directory(&metadata, store.root_device) {
        Ok(())
    } else {
        Err(LinuxInstallStoreError::StorePathUnsafe)
    }
}

#[cfg(target_os = "linux")]
fn verify_installation_permissions(
    store: &LinuxInstallStore,
    root: &Path,
    bundle: &VerifiedBundle,
) -> Result<(), LinuxInstallStoreError> {
    verify_private_directory(store, root)?;
    for file in bundle.files() {
        verify_installed_path_permissions(store, root, file.path())?;
    }
    verify_installed_path_permissions(store, root, &installation_record_path())?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn verify_installed_path_permissions(
    store: &LinuxInstallStore,
    root: &Path,
    path: &BundlePath,
) -> Result<(), LinuxInstallStoreError> {
    let mut current = root.to_path_buf();
    let components = path.as_str().split('/').collect::<Vec<_>>();
    for component in &components[..components.len().saturating_sub(1)] {
        current.push(component);
        verify_private_directory(store, &current)?;
    }

    current.push(
        components
            .last()
            .expect("bundle paths have at least one component"),
    );
    let metadata =
        fs::symlink_metadata(&current).map_err(|_| LinuxInstallStoreError::StorePathUnsafe)?;
    if safe_private_file(&metadata, store.root_device) {
        Ok(())
    } else {
        Err(LinuxInstallStoreError::StorePathUnsafe)
    }
}

#[cfg(target_os = "linux")]
fn acquire_subject_lock(
    store: &LinuxInstallStore,
    subject: &Sha256Digest,
) -> Result<SubjectLock, LinuxInstallStoreError> {
    let directory = store.root.join(".locks");
    ensure_private_directory(store, &directory)?;
    let path = directory.join(format!("{}.lock", subject.as_str()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true).mode(PRIVATE_FILE_MODE);
    let file = match options.open(&path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(LinuxInstallStoreError::SubjectLocked);
        }
        Err(_) => return Err(LinuxInstallStoreError::Io),
    };
    file.sync_all().map_err(|_| LinuxInstallStoreError::Io)?;
    let metadata = file.metadata().map_err(|_| LinuxInstallStoreError::Io)?;
    if !safe_private_file(&metadata, store.root_device) {
        let _ = fs::remove_file(&path);
        return Err(LinuxInstallStoreError::StorePathUnsafe);
    }
    sync_directory(&directory)?;
    Ok(SubjectLock {
        path,
        directory,
        _file: file,
    })
}

#[cfg(target_os = "linux")]
fn create_staging(
    store: &LinuxInstallStore,
    subject: &Sha256Digest,
) -> Result<PathBuf, LinuxInstallStoreError> {
    let staging_root = store.root.join(".staging");
    ensure_private_directory(store, &staging_root)?;
    for attempt in 0..MAX_STAGING_ATTEMPTS {
        let candidate = staging_root.join(format!("{}.{}", subject.as_str(), attempt));
        let mut builder = DirBuilder::new();
        builder.mode(PRIVATE_DIRECTORY_MODE);
        match builder.create(&candidate) {
            Ok(()) => {
                fs::set_permissions(
                    &candidate,
                    fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE),
                )
                .map_err(|_| LinuxInstallStoreError::Io)?;
                verify_private_directory(store, &candidate)?;
                sync_directory(&staging_root)?;
                return Ok(candidate);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err(LinuxInstallStoreError::Io),
        }
    }
    Err(LinuxInstallStoreError::StagingUnavailable)
}

#[cfg(target_os = "linux")]
fn ensure_store_root(store: &LinuxInstallStore) -> Result<(), LinuxInstallStoreError> {
    let metadata =
        fs::symlink_metadata(&store.root).map_err(|_| LinuxInstallStoreError::StoreChanged)?;
    if metadata.file_type().is_symlink()
        || !metadata.is_dir()
        || metadata.dev() != store.root_device
        || metadata.ino() != store.root_inode
        || !protected_root_mode(metadata.mode())
    {
        return Err(LinuxInstallStoreError::StoreChanged);
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn protected_root_mode(mode: u32) -> bool {
    mode & 0o700 == 0o700 && mode & 0o022 == 0
}

#[cfg(target_os = "linux")]
fn safe_private_directory(metadata: &fs::Metadata, device: u64) -> bool {
    !metadata.file_type().is_symlink()
        && metadata.is_dir()
        && metadata.dev() == device
        && metadata.mode() & 0o777 == PRIVATE_DIRECTORY_MODE
}

#[cfg(target_os = "linux")]
fn safe_private_file(metadata: &fs::Metadata, device: u64) -> bool {
    !metadata.file_type().is_symlink()
        && metadata.is_file()
        && metadata.dev() == device
        && metadata.nlink() == 1
        && metadata.mode() & 0o777 == PRIVATE_FILE_MODE
}

#[cfg(target_os = "linux")]
fn path_exists_no_follow(path: &Path) -> Result<bool, LinuxInstallStoreError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err(LinuxInstallStoreError::Io),
    }
}

#[cfg(target_os = "linux")]
fn cleanup_staging(staging: &Path) {
    if fs::symlink_metadata(staging).is_ok() {
        let _ = fs::remove_dir_all(staging);
    }
}

#[cfg(target_os = "linux")]
fn sync_directory(directory: &Path) -> Result<(), LinuxInstallStoreError> {
    File::open(directory)
        .and_then(|file| file.sync_all())
        .map_err(|_| LinuxInstallStoreError::Io)
}

fn reject_reserved_paths(bundle: &VerifiedBundle) -> Result<(), LinuxInstallStoreError> {
    let reserved = INSTALLATION_RECORD_PATH.to_ascii_lowercase();
    let reserved_prefix = format!("{reserved}/");
    if bundle.files().iter().any(|file| {
        let path = file.path().as_str().to_ascii_lowercase();
        path == reserved || path.starts_with(&reserved_prefix)
    }) {
        Err(LinuxInstallStoreError::ReservedMetadataPath)
    } else {
        Ok(())
    }
}

fn installation_record_path() -> BundlePath {
    BundlePath::parse(INSTALLATION_RECORD_PATH)
        .expect("the reserved installation-record path is a valid bundle path")
}

fn manifest_from_verified(
    bundle: &VerifiedBundle,
) -> Result<BundleManifest, LinuxInstallStoreError> {
    let files = bundle
        .files()
        .iter()
        .map(|file| {
            let length = u64::try_from(file.bytes().len())
                .map_err(|_| LinuxInstallStoreError::ManifestReconstruction)?;
            Ok(BundleFile::new(
                file.path().clone(),
                length,
                file.digest().clone(),
            ))
        })
        .collect::<Result<Vec<_>, LinuxInstallStoreError>>()?;

    BundleManifest::new(
        BUNDLE_FORMAT,
        SHA256_ALGORITHM,
        bundle.entry_point().clone(),
        files,
        BundleLimits::default(),
    )
    .map_err(|_| LinuxInstallStoreError::ManifestReconstruction)
}

#[derive(Serialize)]
struct InstallationRecordV1<'a> {
    format: &'static str,
    bundle_subject: &'a str,
    entry_point: &'a str,
    root_identity: String,
    installed_tree_digest_format: &'static str,
    installed_tree_digest: String,
    publisher_authentication: &'static str,
    files: Vec<InstallationFileRecord<'a>>,
}

#[derive(Serialize)]
struct InstallationRecordV2<'a> {
    format: &'static str,
    bundle_subject: &'a str,
    entry_point: &'a str,
    root_identity: String,
    installed_tree_digest_format: &'static str,
    installed_tree_digest: String,
    publisher_authentication: AuthenticatedPublisherRecord<'a>,
    files: Vec<InstallationFileRecord<'a>>,
}

#[derive(Serialize)]
struct AuthenticatedPublisherRecord<'a> {
    status: &'static str,
    mechanism: &'a str,
    subject: &'a str,
    identity: &'a std::collections::BTreeMap<String, String>,
}

#[derive(Serialize)]
struct InstallationFileRecord<'a> {
    path: &'a str,
    byte_length: u64,
    sha256: &'a str,
}

fn installation_record_bytes(
    bundle: &VerifiedBundle,
    publisher: Option<&AuthorizedPublisher>,
) -> Result<Vec<u8>, LinuxInstallStoreError> {
    if publisher.is_some_and(|publisher| publisher.subject() != bundle.subject()) {
        return Err(LinuxInstallStoreError::PublisherSubjectMismatch);
    }

    let files = bundle
        .files()
        .iter()
        .map(|file| {
            Ok(InstallationFileRecord {
                path: file.path().as_str(),
                byte_length: u64::try_from(file.bytes().len())
                    .map_err(|_| LinuxInstallStoreError::RecordSerialization)?,
                sha256: file.digest().as_str(),
            })
        })
        .collect::<Result<Vec<_>, LinuxInstallStoreError>>()?;
    let root_identity = format!("sha256/{}", bundle.subject().as_str());
    let installed_tree_digest = installed_tree_digest(bundle)?;
    let mut bytes = match publisher {
        None => serde_json::to_vec(&InstallationRecordV1 {
            format: INSTALLATION_RECORD_FORMAT,
            bundle_subject: bundle.subject().as_str(),
            entry_point: bundle.entry_point().as_str(),
            root_identity,
            installed_tree_digest_format: INSTALLED_TREE_DIGEST_FORMAT,
            installed_tree_digest,
            publisher_authentication: PUBLISHER_AUTHENTICATION_NOT_PROVIDED,
            files,
        }),
        Some(publisher) => serde_json::to_vec(&InstallationRecordV2 {
            format: AUTHENTICATED_INSTALLATION_RECORD_FORMAT,
            bundle_subject: bundle.subject().as_str(),
            entry_point: bundle.entry_point().as_str(),
            root_identity,
            installed_tree_digest_format: INSTALLED_TREE_DIGEST_FORMAT,
            installed_tree_digest,
            publisher_authentication: AuthenticatedPublisherRecord {
                status: PUBLISHER_AUTHENTICATION_VERIFIED_AND_AUTHORIZED,
                mechanism: publisher.mechanism().as_str(),
                subject: publisher.subject().as_str(),
                identity: publisher.identity().attributes(),
            },
            files,
        }),
    }
    .map_err(|_| LinuxInstallStoreError::RecordSerialization)?;
    bytes.push(b'\n');
    if u64::try_from(bytes.len()).map_or(true, |length| length > MAX_INSTALLATION_RECORD_BYTES) {
        return Err(LinuxInstallStoreError::RecordTooLarge);
    }
    Ok(bytes)
}

fn installed_tree_digest(bundle: &VerifiedBundle) -> Result<String, LinuxInstallStoreError> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(INSTALLED_TREE_DIGEST_FORMAT.as_bytes());
    bytes.push(0);
    for file in bundle.files() {
        bytes.extend_from_slice(file.path().as_str().as_bytes());
        bytes.push(0);
        let length = u64::try_from(file.bytes().len())
            .map_err(|_| LinuxInstallStoreError::RecordSerialization)?;
        bytes.extend_from_slice(length.to_string().as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(file.digest().as_str().as_bytes());
        bytes.push(b'\n');
    }
    Ok(sha256_lower_hex(&bytes))
}

fn receipt(bundle: &VerifiedBundle, root: PathBuf, reused: bool) -> InstalledBundle {
    InstalledBundle {
        subject: bundle.subject().clone(),
        record: root.join(INSTALLATION_RECORD_PATH),
        root,
        reused,
    }
}
