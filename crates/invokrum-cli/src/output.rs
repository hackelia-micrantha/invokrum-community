use std::fmt;
use std::path::Path;

#[cfg(target_os = "linux")]
use std::fs::{self, File, Metadata, OpenOptions};
#[cfg(target_os = "linux")]
use std::io::Write;
#[cfg(target_os = "linux")]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
#[cfg(target_os = "linux")]
use std::path::{Component, PathBuf};
#[cfg(target_os = "linux")]
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(target_os = "linux")]
static NEXT_TEMP: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OutputError {
    InvalidPath,
    ParentTraversal,
    ParentSymbolicLink,
    ParentNotDirectory,
    ParentChanged,
    TargetExists,
    TargetSymbolicLink,
    TargetNotRegular,
    #[cfg(not(target_os = "linux"))]
    UnsupportedPlatform,
    Io,
}

impl fmt::Display for OutputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidPath => "output path must name a UTF-8 file below an existing directory",
            Self::ParentTraversal => "output path must not contain parent-directory traversal",
            Self::ParentSymbolicLink => "output parent path must not contain symbolic links",
            Self::ParentNotDirectory => "output parent component is not a directory",
            Self::ParentChanged => "output parent changed during the write",
            Self::TargetExists => "output already exists; use --force for explicit replacement",
            Self::TargetSymbolicLink => "output target must not be a symbolic link",
            Self::TargetNotRegular => "output target must be a regular file",
            #[cfg(not(target_os = "linux"))]
            Self::UnsupportedPlatform => "safe output persistence is supported on Linux only",
            Self::Io => "output operation failed",
        })
    }
}

pub(crate) fn write_atomic(path: &Path, bytes: &[u8], force: bool) -> Result<(), OutputError> {
    write_platform(path, bytes, force)
}

#[cfg(target_os = "linux")]
fn write_platform(path: &Path, bytes: &[u8], force: bool) -> Result<(), OutputError> {
    let file_name = validated_file_name(path)?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    reject_parent_links(parent)?;
    let canonical_parent = fs::canonicalize(parent).map_err(|_| OutputError::Io)?;
    let pinned_parent = pin_parent(&canonical_parent)?;
    let target = canonical_parent.join(file_name);
    check_target(&target, force)?;

    let temporary = temporary_path(&canonical_parent, file_name);
    let mut temporary_guard = FileGuard::new(temporary.clone());
    let staged = stage_file(&temporary, bytes)?;
    verify_parent_identity(&canonical_parent, &pinned_parent)?;

    if force {
        commit_replace(&temporary, &target, &staged)?;
    } else {
        commit_no_clobber(&temporary, &target, &staged)?;
    }
    temporary_guard.disarm();

    verify_parent_identity(&canonical_parent, &pinned_parent)?;
    sync_directory(&canonical_parent)?;
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn write_platform(_path: &Path, _bytes: &[u8], _force: bool) -> Result<(), OutputError> {
    Err(OutputError::UnsupportedPlatform)
}

#[cfg(target_os = "linux")]
fn validated_file_name(path: &Path) -> Result<&str, OutputError> {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .ok_or(OutputError::InvalidPath)
}

#[cfg(target_os = "linux")]
fn reject_parent_links(parent: &Path) -> Result<(), OutputError> {
    let mut current = if parent.is_absolute() {
        PathBuf::from("/")
    } else {
        std::env::current_dir().map_err(|_| OutputError::Io)?
    };
    for component in parent.components() {
        match component {
            Component::RootDir | Component::CurDir => {}
            Component::ParentDir => return Err(OutputError::ParentTraversal),
            Component::Normal(segment) => {
                current.push(segment);
                let metadata = fs::symlink_metadata(&current).map_err(|_| OutputError::Io)?;
                if metadata.file_type().is_symlink() {
                    return Err(OutputError::ParentSymbolicLink);
                }
                if !metadata.is_dir() {
                    return Err(OutputError::ParentNotDirectory);
                }
            }
            Component::Prefix(_) => return Err(OutputError::InvalidPath),
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn pin_parent(parent: &Path) -> Result<Metadata, OutputError> {
    let metadata = fs::symlink_metadata(parent).map_err(|_| OutputError::Io)?;
    if metadata.file_type().is_symlink() {
        return Err(OutputError::ParentSymbolicLink);
    }
    if !metadata.is_dir() {
        return Err(OutputError::ParentNotDirectory);
    }
    Ok(metadata)
}

#[cfg(target_os = "linux")]
fn verify_parent_identity(parent: &Path, pinned: &Metadata) -> Result<(), OutputError> {
    let current = fs::symlink_metadata(parent).map_err(|_| OutputError::ParentChanged)?;
    if current.file_type().is_symlink() || !current.is_dir() || !same_identity(&current, pinned) {
        Err(OutputError::ParentChanged)
    } else {
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn check_target(target: &Path, force: bool) -> Result<(), OutputError> {
    match fs::symlink_metadata(target) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                return Err(OutputError::TargetSymbolicLink);
            }
            if !metadata.is_file() {
                return Err(OutputError::TargetNotRegular);
            }
            if !force {
                return Err(OutputError::TargetExists);
            }
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(OutputError::Io),
    }
}

#[cfg(target_os = "linux")]
fn temporary_path(parent: &Path, file_name: &str) -> PathBuf {
    let sequence = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
    parent.join(format!(
        ".{file_name}.invokrum-{}-{sequence}.tmp",
        std::process::id()
    ))
}

#[cfg(target_os = "linux")]
fn stage_file(path: &Path, bytes: &[u8]) -> Result<Metadata, OutputError> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(path)
        .map_err(|_| OutputError::Io)?;
    file.write_all(bytes).map_err(|_| OutputError::Io)?;
    file.sync_all().map_err(|_| OutputError::Io)?;
    let metadata = file.metadata().map_err(|_| OutputError::Io)?;
    validate_staged_metadata(&metadata)?;
    Ok(metadata)
}

#[cfg(target_os = "linux")]
fn validate_staged_metadata(metadata: &Metadata) -> Result<(), OutputError> {
    if !metadata.is_file() {
        return Err(OutputError::TargetNotRegular);
    }
    if metadata.mode() & 0o777 != 0o600 || metadata.nlink() != 1 {
        return Err(OutputError::Io);
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn commit_replace(temporary: &Path, target: &Path, staged: &Metadata) -> Result<(), OutputError> {
    check_target(target, true)?;
    fs::rename(temporary, target).map_err(|_| OutputError::Io)?;
    validate_committed_target(target, staged)
}

#[cfg(target_os = "linux")]
fn commit_no_clobber(
    temporary: &Path,
    target: &Path,
    staged: &Metadata,
) -> Result<(), OutputError> {
    match fs::hard_link(temporary, target) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(OutputError::TargetExists);
        }
        Err(_) => return Err(OutputError::Io),
    }

    let mut committed_guard = FileGuard::new_if_identity(target.to_path_buf(), staged);
    fs::remove_file(temporary).map_err(|_| OutputError::Io)?;
    validate_committed_target(target, staged)?;
    committed_guard.disarm();
    Ok(())
}

#[cfg(target_os = "linux")]
fn validate_committed_target(target: &Path, staged: &Metadata) -> Result<(), OutputError> {
    let metadata = fs::symlink_metadata(target).map_err(|_| OutputError::Io)?;
    if metadata.file_type().is_symlink() {
        return Err(OutputError::TargetSymbolicLink);
    }
    if !metadata.is_file() || !same_identity(&metadata, staged) {
        return Err(OutputError::TargetNotRegular);
    }
    if metadata.mode() & 0o777 != 0o600 || metadata.nlink() != 1 {
        return Err(OutputError::Io);
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn same_identity(left: &Metadata, right: &Metadata) -> bool {
    left.dev() == right.dev() && left.ino() == right.ino()
}

#[cfg(target_os = "linux")]
fn sync_directory(parent: &Path) -> Result<(), OutputError> {
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| OutputError::Io)
}

#[cfg(target_os = "linux")]
struct FileGuard {
    path: Option<PathBuf>,
    identity: Option<(u64, u64)>,
}

#[cfg(target_os = "linux")]
impl FileGuard {
    fn new(path: PathBuf) -> Self {
        Self {
            path: Some(path),
            identity: None,
        }
    }

    fn new_if_identity(path: PathBuf, metadata: &Metadata) -> Self {
        Self {
            path: Some(path),
            identity: Some((metadata.dev(), metadata.ino())),
        }
    }

    fn disarm(&mut self) {
        self.path = None;
    }
}

#[cfg(target_os = "linux")]
impl Drop for FileGuard {
    fn drop(&mut self) {
        let Some(path) = self.path.take() else {
            return;
        };
        if let Some((device, inode)) = self.identity {
            let matches = fs::symlink_metadata(&path)
                .map(|metadata| metadata.dev() == device && metadata.ino() == inode)
                .unwrap_or(false);
            if !matches {
                return;
            }
        }
        let _ = fs::remove_file(path);
    }
}
