#![cfg(target_os = "linux")]

use std::ffi::OsString;
use std::fs;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::symlink;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use invokrum_acquisition::verify_candidate;
use invokrum_acquisition_linux::{LinuxLocalCandidateSource, LocalCandidateError};
use invokrum_digest::sha256_lower_hex;
use invokrum_distribution::{
    BUNDLE_FORMAT, BundleFile, BundleLimits, BundleManifest, BundlePath, MAX_BUNDLE_FILE_BYTES,
    SHA256_ALGORITHM, Sha256Digest,
};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

struct TempTree(PathBuf);

impl TempTree {
    fn new(label: &str) -> Self {
        let index = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "invokrum-acquisition-linux-{label}-{}-{index}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempTree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn path(value: &str) -> BundlePath {
    BundlePath::parse(value).unwrap()
}

fn digest(bytes: &[u8]) -> Sha256Digest {
    Sha256Digest::parse(sha256_lower_hex(bytes)).unwrap()
}

fn subject() -> Sha256Digest {
    Sha256Digest::parse("a".repeat(64)).unwrap()
}

fn manifest(records: &[(&str, &[u8])], entry: &str) -> BundleManifest {
    BundleManifest::new(
        BUNDLE_FORMAT,
        SHA256_ALGORITHM,
        path(entry),
        records
            .iter()
            .map(|(name, bytes)| {
                BundleFile::new(
                    path(name),
                    u64::try_from(bytes.len()).unwrap(),
                    digest(bytes),
                )
            })
            .collect(),
        BundleLimits::default(),
    )
    .unwrap()
}

fn write(root: &Path, relative: &str, bytes: &[u8]) {
    let destination = root.join(relative);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(destination, bytes).unwrap();
}

#[test]
fn valid_nested_tree_returns_exact_owned_bytes_for_verification() {
    let root = TempTree::new("valid");
    write(root.path(), "pack.yaml", b"pack");
    write(root.path(), "overlays/core.md", b"core");
    let bundle_manifest = manifest(
        &[("pack.yaml", b"pack"), ("overlays/core.md", b"core")],
        "pack.yaml",
    );
    let source = LinuxLocalCandidateSource::open(root.path().to_path_buf()).unwrap();

    let candidates = source.load(&bundle_manifest).unwrap();
    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].path(), &path("overlays/core.md"));
    assert_eq!(candidates[0].bytes(), b"core");
    assert_eq!(candidates[1].path(), &path("pack.yaml"));
    assert_eq!(candidates[1].bytes(), b"pack");

    let immutable_subject = subject();
    verify_candidate(
        &bundle_manifest,
        &immutable_subject,
        &immutable_subject,
        candidates,
    )
    .expect("adapter bytes should feed the existing verifier");
}

#[test]
fn symlink_hardlink_special_and_undeclared_entries_fail_closed() {
    let symlink_root = TempTree::new("symlink");
    write(symlink_root.path(), "target.md", b"target");
    symlink("target.md", symlink_root.path().join("pack.yaml")).unwrap();
    let source = LinuxLocalCandidateSource::open(symlink_root.path().to_path_buf()).unwrap();
    assert_eq!(
        source.load(&manifest(&[("pack.yaml", b"target")], "pack.yaml")),
        Err(LocalCandidateError::SymbolicLink {
            path: path("pack.yaml")
        })
    );

    let hardlink_root = TempTree::new("hardlink");
    write(hardlink_root.path(), "pack.yaml", b"pack");
    fs::hard_link(
        hardlink_root.path().join("pack.yaml"),
        hardlink_root.path().join("other.md"),
    )
    .unwrap();
    let source = LinuxLocalCandidateSource::open(hardlink_root.path().to_path_buf()).unwrap();
    assert!(matches!(
        source.load(&manifest(&[("pack.yaml", b"pack")], "pack.yaml")),
        Err(LocalCandidateError::HardLinkedFile { .. })
    ));

    let socket_root = TempTree::new("socket");
    write(socket_root.path(), "pack.yaml", b"pack");
    let _listener = UnixListener::bind(socket_root.path().join("socket")).unwrap();
    let source = LinuxLocalCandidateSource::open(socket_root.path().to_path_buf()).unwrap();
    assert_eq!(
        source.load(&manifest(&[("pack.yaml", b"pack")], "pack.yaml")),
        Err(LocalCandidateError::NonRegularFile {
            path: path("socket")
        })
    );

    let extra_file_root = TempTree::new("extra-file");
    write(extra_file_root.path(), "pack.yaml", b"pack");
    write(extra_file_root.path(), "extra.md", b"extra");
    let source = LinuxLocalCandidateSource::open(extra_file_root.path().to_path_buf()).unwrap();
    assert_eq!(
        source.load(&manifest(&[("pack.yaml", b"pack")], "pack.yaml")),
        Err(LocalCandidateError::UndeclaredEntry {
            path: path("extra.md")
        })
    );

    let extra_directory_root = TempTree::new("extra-directory");
    write(extra_directory_root.path(), "pack.yaml", b"pack");
    fs::create_dir(extra_directory_root.path().join("empty-extra")).unwrap();
    let source =
        LinuxLocalCandidateSource::open(extra_directory_root.path().to_path_buf()).unwrap();
    assert_eq!(
        source.load(&manifest(&[("pack.yaml", b"pack")], "pack.yaml")),
        Err(LocalCandidateError::UndeclaredEntry {
            path: path("empty-extra")
        })
    );
}

#[test]
fn missing_non_ascii_and_case_colliding_paths_fail_closed() {
    let missing_root = TempTree::new("missing");
    let source = LinuxLocalCandidateSource::open(missing_root.path().to_path_buf()).unwrap();
    assert_eq!(
        source.load(&manifest(&[("pack.yaml", b"pack")], "pack.yaml")),
        Err(LocalCandidateError::MissingDeclaredFile {
            path: path("pack.yaml")
        })
    );

    let invalid_root = TempTree::new("invalid-name");
    write(invalid_root.path(), "pack.yaml", b"pack");
    let invalid_name = OsString::from_vec(vec![0xff, b'.', b'm', b'd']);
    fs::write(invalid_root.path().join(invalid_name), b"extra").unwrap();
    let source = LinuxLocalCandidateSource::open(invalid_root.path().to_path_buf()).unwrap();
    assert_eq!(
        source.load(&manifest(&[("pack.yaml", b"pack")], "pack.yaml")),
        Err(LocalCandidateError::InvalidEntryName)
    );

    let collision_root = TempTree::new("collision");
    write(collision_root.path(), "A.md", b"upper");
    write(collision_root.path(), "a.md", b"lower");
    let source = LinuxLocalCandidateSource::open(collision_root.path().to_path_buf()).unwrap();
    assert_eq!(
        source.load(&manifest(&[("A.md", b"upper"), ("a.md", b"lower")], "A.md",)),
        Err(LocalCandidateError::LogicalPathCollision)
    );
}

#[test]
fn one_explicit_installer_metadata_file_can_be_verified_without_becoming_bundle_content() {
    let root = TempTree::new("allowed-metadata");
    write(root.path(), "pack.yaml", b"pack");
    write(
        root.path(),
        ".invokrum-installation.json",
        b"{\"format\":\"invokrum.installation/v1\"}\n",
    );
    let bundle_manifest = manifest(&[("pack.yaml", b"pack")], "pack.yaml");
    let record = path(".invokrum-installation.json");
    let source = LinuxLocalCandidateSource::open(root.path().to_path_buf()).unwrap();

    let candidates = source
        .load_with_allowed_file(&bundle_manifest, &record, MAX_BUNDLE_FILE_BYTES)
        .unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].path(), &path("pack.yaml"));
    assert_eq!(
        source.read_exact(&record).unwrap(),
        b"{\"format\":\"invokrum.installation/v1\"}\n"
    );
}

#[test]
fn root_symlink_is_rejected_at_open() {
    let target = TempTree::new("root-target");
    let holder = TempTree::new("root-holder");
    let link = holder.path().join("candidate");
    symlink(target.path(), &link).unwrap();
    assert!(matches!(
        LinuxLocalCandidateSource::open(link),
        Err(LocalCandidateError::RootSymlink)
    ));
}
