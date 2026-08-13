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
use invokrum_install::{InstallWorkflowError, VerifiedBundleStore, install_candidate};
use invokrum_install_linux::{
    INSTALLATION_RECORD_PATH, LinuxCandidateLoader, LinuxInstallStore, LinuxInstallStoreError,
};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

struct TempTree(PathBuf);

impl TempTree {
    fn new(label: &str) -> Self {
        let index = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "invokrum-install-adversarial-{label}-{}-{index}",
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

fn subject(character: char) -> Sha256Digest {
    Sha256Digest::parse(character.to_string().repeat(64)).unwrap()
}

fn manifest(records: &[(&str, &[u8])], entry_point: &str) -> BundleManifest {
    BundleManifest::new(
        BUNDLE_FORMAT,
        SHA256_ALGORITHM,
        path(entry_point),
        records
            .iter()
            .map(|(record_path, bytes)| {
                BundleFile::new(
                    path(record_path),
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
fn reserved_installation_record_path_cannot_be_bundle_content() {
    let store_root = TempTree::new("reserved-record-store");
    let store = LinuxInstallStore::open(store_root.path()).unwrap();
    let bytes = b"candidate-controlled evidence";
    let bundle_manifest = manifest(
        &[(INSTALLATION_RECORD_PATH, bytes)],
        INSTALLATION_RECORD_PATH,
    );
    let bundle_subject = subject('a');
    let verified = verify_candidate(
        &bundle_manifest,
        &bundle_subject,
        &bundle_subject,
        vec![CandidateFile::new(path(INSTALLATION_RECORD_PATH), bytes.to_vec()).unwrap()],
    )
    .unwrap();

    assert_eq!(
        VerifiedBundleStore::install(&store, &verified),
        Err(LinuxInstallStoreError::ReservedMetadataPath)
    );
    assert!(!store.subject_root(&bundle_subject).exists());
}

#[test]
fn tampered_installation_record_prevents_subject_reuse() {
    let candidate = TempTree::new("record-candidate");
    let store_root = TempTree::new("record-store");
    write(candidate.path(), "pack.yaml", b"pack");
    let bundle_manifest = manifest(&[("pack.yaml", b"pack")], "pack.yaml");
    let bundle_subject = subject('b');
    let loader = LinuxCandidateLoader::open(candidate.path()).unwrap();
    let store = LinuxInstallStore::open(store_root.path()).unwrap();
    let installed = install_candidate(
        &loader,
        &store,
        &bundle_manifest,
        &bundle_subject,
        &bundle_subject,
    )
    .unwrap();

    fs::write(installed.record(), b"{}\n").unwrap();
    assert_eq!(
        install_candidate(
            &loader,
            &store,
            &bundle_manifest,
            &bundle_subject,
            &bundle_subject,
        ),
        Err(InstallWorkflowError::Store(
            LinuxInstallStoreError::ExistingInstallationMismatch
        ))
    );
    assert_eq!(fs::read(installed.record()).unwrap(), b"{}\n");
}

#[test]
fn symlinked_internal_store_directory_is_rejected() {
    let candidate = TempTree::new("internal-link-candidate");
    let store_root = TempTree::new("internal-link-store");
    let outside = TempTree::new("internal-link-outside");
    write(candidate.path(), "pack.yaml", b"pack");
    let bundle_manifest = manifest(&[("pack.yaml", b"pack")], "pack.yaml");
    let bundle_subject = subject('c');
    let loader = LinuxCandidateLoader::open(candidate.path()).unwrap();
    let store = LinuxInstallStore::open(store_root.path()).unwrap();

    fs::remove_dir(store_root.path().join(".staging")).unwrap();
    symlink(outside.path(), store_root.path().join(".staging")).unwrap();

    assert_eq!(
        install_candidate(
            &loader,
            &store,
            &bundle_manifest,
            &bundle_subject,
            &bundle_subject,
        ),
        Err(InstallWorkflowError::Store(
            LinuxInstallStoreError::StorePathUnsafe
        ))
    );
    assert!(!store.subject_root(&bundle_subject).exists());
}
