#![cfg(target_os = "linux")]

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use invokrum_core::{OverlaySource, PackRelativePath, SourceFailure, SourceFailureKind};
use invokrum_fs::{LocalPackSource, LocalPackSourceError};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("invokrum-fs-{}-{sequence}", std::process::id()));
        fs::create_dir_all(&path).expect("test directory should be created");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn path(value: &str) -> PackRelativePath {
    PackRelativePath::parse(value).expect("test path should be valid")
}

fn failure(value: &str, kind: SourceFailureKind) -> SourceFailure {
    SourceFailure::new(path(value), kind)
}

#[test]
fn regular_file_is_loaded_from_canonical_root() {
    let root = TestDirectory::new();
    fs::create_dir(root.path().join("overlays")).expect("overlay directory should be created");
    fs::write(root.path().join("overlays/core.md"), b"core").expect("overlay should be written");

    let source = LocalPackSource::open(root.path()).expect("root should be accepted");
    assert!(source.root().is_absolute());
    assert_eq!(
        source
            .load(&path("overlays/core.md"), 16)
            .expect("file should load"),
        b"core"
    );
}

#[test]
fn file_size_limit_is_enforced_during_read() {
    let root = TestDirectory::new();
    fs::write(root.path().join("large.md"), b"12345").expect("file should be written");
    let source = LocalPackSource::open(root.path()).expect("root should be accepted");

    assert_eq!(
        source.load(&path("large.md"), 4),
        Err(failure("large.md", SourceFailureKind::TooLarge))
    );
}

#[test]
fn replacing_the_pack_root_is_rejected() {
    let parent = TestDirectory::new();
    let root = parent.path().join("pack");
    let original = parent.path().join("original-pack");
    fs::create_dir(&root).expect("pack root should be created");
    fs::write(root.join("overlay.md"), b"trusted").expect("overlay should be written");
    let source = LocalPackSource::open(&root).expect("root should be accepted");

    fs::rename(&root, &original).expect("original root should be moved");
    fs::create_dir(&root).expect("replacement root should be created");
    fs::write(root.join("overlay.md"), b"replacement").expect("replacement should be written");

    assert_eq!(
        source.load(&path("overlay.md"), 32),
        Err(failure("overlay.md", SourceFailureKind::ChangedDuringRead))
    );
}

#[test]
fn symbolic_links_are_rejected_at_root_and_below_root() {
    let parent = TestDirectory::new();
    let real_root = parent.path().join("real");
    fs::create_dir(&real_root).expect("real root should be created");
    fs::write(real_root.join("target.md"), b"target").expect("target should be written");

    let linked_root = parent.path().join("linked-root");
    symlink(&real_root, &linked_root).expect("root symlink should be created");
    assert!(matches!(
        LocalPackSource::open(&linked_root),
        Err(LocalPackSourceError::RootSymbolicLink)
    ));

    let linked_file = real_root.join("linked.md");
    symlink(real_root.join("target.md"), &linked_file).expect("file symlink should be created");
    let source = LocalPackSource::open(&real_root).expect("real root should be accepted");
    assert_eq!(
        source.load(&path("linked.md"), 32),
        Err(failure("linked.md", SourceFailureKind::SymbolicLink))
    );
}

#[test]
fn hard_links_are_rejected() {
    let root = TestDirectory::new();
    let original = root.path().join("original.md");
    fs::write(&original, b"same inode").expect("file should be written");
    fs::hard_link(&original, root.path().join("linked.md")).expect("hard link should be created");
    let source = LocalPackSource::open(root.path()).expect("root should be accepted");

    assert_eq!(
        source.load(&path("original.md"), 32),
        Err(failure("original.md", SourceFailureKind::HardLink))
    );
}

#[test]
fn directories_are_not_overlay_sources() {
    let root = TestDirectory::new();
    fs::create_dir(root.path().join("directory.md")).expect("directory should be created");
    let source = LocalPackSource::open(root.path()).expect("root should be accepted");

    assert_eq!(
        source.load(&path("directory.md"), 32),
        Err(failure("directory.md", SourceFailureKind::NotRegularFile))
    );
}
