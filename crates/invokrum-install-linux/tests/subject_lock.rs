#![cfg(target_os = "linux")]

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use invokrum_acquisition::{CandidateFile, verify_candidate};
use invokrum_digest::sha256_lower_hex;
use invokrum_distribution::{
    BUNDLE_FORMAT, BundleFile, BundleLimits, BundleManifest, BundlePath, SHA256_ALGORITHM,
    Sha256Digest,
};
use invokrum_install::VerifiedBundleStore;
use invokrum_install_linux::{LinuxInstallStore, LinuxInstallStoreError};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

struct TempTree(PathBuf);

impl TempTree {
    fn new(label: &str) -> Self {
        let index = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "invokrum-install-lock-{label}-{}-{index}",
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
    Sha256Digest::parse("c".repeat(64)).unwrap()
}

fn verified_bundle() -> invokrum_acquisition::VerifiedBundle {
    let bytes = b"pack";
    let manifest = BundleManifest::new(
        BUNDLE_FORMAT,
        SHA256_ALGORITHM,
        path("pack.yaml"),
        vec![BundleFile::new(path("pack.yaml"), 4, digest(bytes))],
        BundleLimits::default(),
    )
    .unwrap();
    let bundle_subject = subject();
    verify_candidate(
        &manifest,
        &bundle_subject,
        &bundle_subject,
        vec![CandidateFile::new(path("pack.yaml"), bytes.to_vec()).unwrap()],
    )
    .unwrap()
}

#[test]
fn existing_subject_lock_refuses_concurrent_promotion() {
    let store_root = TempTree::new("held");
    let store = LinuxInstallStore::open(store_root.path()).unwrap();
    let bundle = verified_bundle();
    let lock = store_root
        .path()
        .join(".locks")
        .join(format!("{}.lock", bundle.subject().as_str()));
    fs::write(&lock, b"").unwrap();

    assert_eq!(
        VerifiedBundleStore::install(&store, &bundle),
        Err(LinuxInstallStoreError::SubjectLocked)
    );
    assert!(!store.subject_root(bundle.subject()).exists());
}

#[test]
fn symlink_collision_at_final_subject_is_never_replaced() {
    let store_root = TempTree::new("final-symlink");
    let outside = TempTree::new("outside");
    let store = LinuxInstallStore::open(store_root.path()).unwrap();
    let bundle = verified_bundle();
    let final_root = store.subject_root(bundle.subject());
    symlink(outside.path(), &final_root).unwrap();

    assert_eq!(
        VerifiedBundleStore::install(&store, &bundle),
        Err(LinuxInstallStoreError::ExistingInstallationMismatch)
    );
    assert!(
        fs::symlink_metadata(&final_root)
            .unwrap()
            .file_type()
            .is_symlink()
    );
}
