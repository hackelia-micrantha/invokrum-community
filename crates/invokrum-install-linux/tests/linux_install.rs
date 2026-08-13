#![cfg(target_os = "linux")]

use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use invokrum_acquisition::{CandidateFile, verify_candidate};
use invokrum_core::{CompositionLimits, Identifier, compose};
use invokrum_digest::sha256_lower_hex;
use invokrum_distribution::{
    BUNDLE_FORMAT, BundleFile, BundleLimits, BundleManifest, BundlePath, SHA256_ALGORITHM,
    Sha256Digest,
};
use invokrum_fs::LocalPackSource;
use invokrum_install::{InstallWorkflowError, VerifiedBundleStore, install_candidate};
use invokrum_install_linux::{
    INSTALLATION_RECORD_FORMAT, INSTALLATION_RECORD_PATH, INSTALLED_TREE_DIGEST_FORMAT,
    LinuxCandidateLoader, LinuxInstallStore, LinuxInstallStoreError,
};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

struct TempTree(PathBuf);

impl TempTree {
    fn new(label: &str) -> Self {
        let index = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "invokrum-install-{label}-{}-{index}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir(&path).expect("temporary root should be creatable");
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
    BundlePath::parse(value).expect("fixture path should be valid")
}

fn digest(bytes: &[u8]) -> Sha256Digest {
    Sha256Digest::parse(sha256_lower_hex(bytes)).expect("fixture digest should be valid")
}

fn subject(character: char) -> Sha256Digest {
    Sha256Digest::parse(character.to_string().repeat(64)).expect("fixture subject should be valid")
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
                    u64::try_from(bytes.len()).expect("fixture length should fit u64"),
                    digest(bytes),
                )
            })
            .collect(),
        BundleLimits::default(),
    )
    .expect("fixture manifest should be valid")
}

fn write_candidate(root: &Path, relative: &str, bytes: &[u8]) {
    let destination = root.join(relative);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).expect("fixture parent should be creatable");
    }
    fs::write(destination, bytes).expect("fixture file should be writable");
}

fn install_one(
    label: &str,
    character: char,
) -> (
    TempTree,
    TempTree,
    LinuxCandidateLoader,
    LinuxInstallStore,
    invokrum_distribution::BundleManifest,
    Sha256Digest,
    invokrum_install_linux::InstalledBundle,
) {
    let candidate = TempTree::new(&format!("candidate-{label}"));
    let store_root = TempTree::new(&format!("store-{label}"));
    write_candidate(candidate.path(), "pack.yaml", b"pack");
    let bundle_manifest = manifest(&[("pack.yaml", b"pack")], "pack.yaml");
    let bundle_subject = subject(character);
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
    (
        candidate,
        store_root,
        loader,
        store,
        bundle_manifest,
        bundle_subject,
        installed,
    )
}

#[test]
fn valid_local_candidate_installs_and_reuses_exact_subject_root() {
    let candidate = TempTree::new("candidate-valid");
    let store_root = TempTree::new("store-valid");
    write_candidate(candidate.path(), "pack.yaml", b"pack");
    write_candidate(candidate.path(), "overlays/core.md", b"core");
    let bundle_manifest = manifest(
        &[("pack.yaml", b"pack"), ("overlays/core.md", b"core")],
        "pack.yaml",
    );
    let bundle_subject = subject('a');
    let loader = LinuxCandidateLoader::open(candidate.path()).expect("candidate root should open");
    let store = LinuxInstallStore::open(store_root.path()).expect("store root should open");

    let installed = install_candidate(
        &loader,
        &store,
        &bundle_manifest,
        &bundle_subject,
        &bundle_subject,
    )
    .expect("valid candidate should install");

    assert!(!installed.reused());
    assert_eq!(installed.subject(), &bundle_subject);
    assert_eq!(installed.root(), store.subject_root(&bundle_subject));
    assert_eq!(
        fs::read(installed.root().join("pack.yaml")).unwrap(),
        b"pack"
    );
    assert_eq!(
        fs::read(installed.root().join("overlays/core.md")).unwrap(),
        b"core"
    );

    let record: serde_json::Value = serde_json::from_slice(
        &fs::read(installed.record()).expect("installation record should exist"),
    )
    .expect("installation record should be JSON");
    assert_eq!(record["format"], INSTALLATION_RECORD_FORMAT);
    assert_eq!(record["bundle_subject"], bundle_subject.as_str());
    assert_eq!(record["publisher_authentication"], "not-provided");
    assert_eq!(
        record["root_identity"],
        format!("sha256/{}", bundle_subject.as_str())
    );
    assert_eq!(
        record["installed_tree_digest_format"],
        INSTALLED_TREE_DIGEST_FORMAT
    );
    assert_eq!(
        record["installed_tree_digest"]
            .as_str()
            .expect("tree digest should be a string")
            .len(),
        64
    );

    assert_eq!(
        fs::metadata(installed.root()).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(installed.root().join("pack.yaml"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert_eq!(
        fs::metadata(installed.record())
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );

    let reused = install_candidate(
        &loader,
        &store,
        &bundle_manifest,
        &bundle_subject,
        &bundle_subject,
    )
    .expect("identical existing subject should be reusable");
    assert!(reused.reused());
    assert_eq!(reused.root(), installed.root());
}

#[test]
fn tampered_existing_bytes_are_never_reused_or_mutated() {
    let (_candidate, _store_root, loader, store, bundle_manifest, bundle_subject, installed) =
        install_one("tampered-bytes", 'd');

    fs::write(installed.root().join("pack.yaml"), b"evil").unwrap();
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
    assert_eq!(
        fs::read(installed.root().join("pack.yaml")).unwrap(),
        b"evil"
    );
}

#[test]
fn relaxed_existing_permissions_fail_closed() {
    let (_candidate, _store_root, loader, store, bundle_manifest, bundle_subject, installed) =
        install_one("permissions", '6');

    fs::set_permissions(
        installed.root().join("pack.yaml"),
        fs::Permissions::from_mode(0o644),
    )
    .unwrap();
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
    fs::set_permissions(
        installed.root().join("pack.yaml"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();

    fs::set_permissions(installed.record(), fs::Permissions::from_mode(0o644)).unwrap();
    assert!(matches!(
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
    ));
    fs::set_permissions(installed.record(), fs::Permissions::from_mode(0o600)).unwrap();

    fs::set_permissions(installed.root(), fs::Permissions::from_mode(0o755)).unwrap();
    assert!(matches!(
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
    ));
}

#[test]
fn unsafe_internal_store_permissions_are_rejected_before_promotion() {
    for internal in [".staging", ".locks", "sha256"] {
        let candidate = TempTree::new(&format!("candidate-internal-{internal}"));
        let store_root = TempTree::new(&format!("store-internal-{internal}"));
        write_candidate(candidate.path(), "pack.yaml", b"pack");
        let bundle_manifest = manifest(&[("pack.yaml", b"pack")], "pack.yaml");
        let bundle_subject = subject('7');
        let loader = LinuxCandidateLoader::open(candidate.path()).unwrap();
        let store = LinuxInstallStore::open(store_root.path()).unwrap();
        fs::set_permissions(
            store_root.path().join(internal),
            fs::Permissions::from_mode(0o777),
        )
        .unwrap();

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
}

#[test]
fn group_or_world_writable_store_root_is_rejected() {
    let store_root = TempTree::new("unsafe-store-root");
    fs::set_permissions(store_root.path(), fs::Permissions::from_mode(0o777)).unwrap();
    assert!(matches!(
        LinuxInstallStore::open(store_root.path()),
        Err(LinuxInstallStoreError::StorePathUnsafe)
    ));
}

#[test]
fn failed_materialization_cleans_private_quarantine() {
    let store_root = TempTree::new("store-cleanup");
    let store = LinuxInstallStore::open(store_root.path()).unwrap();
    let bundle_manifest = manifest(&[("a", b"a"), ("a/b", b"b")], "a");
    let bundle_subject = subject('e');
    let verified = verify_candidate(
        &bundle_manifest,
        &bundle_subject,
        &bundle_subject,
        vec![
            CandidateFile::new(path("a"), b"a".to_vec()).unwrap(),
            CandidateFile::new(path("a/b"), b"b".to_vec()).unwrap(),
        ],
    )
    .expect("in-memory verifier does not own filesystem path-prefix policy");

    assert!(VerifiedBundleStore::install(&store, &verified).is_err());
    assert_eq!(
        fs::read_dir(store_root.path().join(".staging"))
            .unwrap()
            .count(),
        0
    );
    assert!(!store.subject_root(&bundle_subject).exists());
}

#[test]
fn installed_pack_root_is_consumable_by_existing_composition_path() {
    let candidate = TempTree::new("candidate-compose");
    let store_root = TempTree::new("store-compose");
    let pack_bytes = b"schema: invokrum.dev/v1\nid: installed-pack\nclasses:\n  - id: core\n    order: 10\n    minimum: 1\n    maximum: 1\noverlays:\n  - id: core-overlay\n    class: core\n    source: overlays/core.md\nprofiles:\n  - id: default\n    selections:\n      core:\n        - core-overlay\nvariables: []\n";
    let overlay_bytes = b"# Installed overlay\nExact verified bytes.";
    write_candidate(candidate.path(), "pack.yaml", pack_bytes);
    write_candidate(candidate.path(), "overlays/core.md", overlay_bytes);
    let bundle_manifest = manifest(
        &[
            ("pack.yaml", pack_bytes),
            ("overlays/core.md", overlay_bytes),
        ],
        "pack.yaml",
    );
    let bundle_subject = subject('f');
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

    let pack = invokrum_schema::parse_yaml(
        &fs::read_to_string(installed.root().join("pack.yaml")).unwrap(),
    )
    .expect("installed entry point should remain an ordinary pack");
    let source =
        LocalPackSource::open(installed.root()).expect("installed root should be a local source");
    let profile = Identifier::parse("default").unwrap();
    let composition = compose(&pack, &profile, &source, CompositionLimits::default())
        .expect("installed root should compose through the existing local adapter");

    assert_eq!(composition.normalized_context(), overlay_bytes);
    assert!(fs::read(installed.root().join(INSTALLATION_RECORD_PATH)).is_ok());
}

#[test]
fn installed_files_remain_single_link_private_regular_files() {
    let (_candidate, _store_root, _loader, _store, _manifest, _subject, installed) =
        install_one("metadata", '1');
    let metadata = fs::metadata(installed.root().join("pack.yaml")).unwrap();
    assert!(metadata.is_file());
    assert_eq!(metadata.nlink(), 1);
    assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
}
