#![cfg(target_os = "linux")]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use invokrum_digest::sha256_lower_hex;
use invokrum_distribution::{
    BUNDLE_FORMAT, BundleFile, BundleLimits, BundleManifest, BundlePath, PublisherAssertion,
    PublisherIdentity, SHA256_ALGORITHM, Sha256Digest, TrustPolicy, TrustRule,
    VerificationMechanism,
};
use invokrum_install::{
    AuthenticatedInstallWorkflowError, InstallWorkflowError, PublisherVerifier,
    install_authenticated_candidate, install_candidate,
};
use invokrum_install_linux::{
    AUTHENTICATED_INSTALLATION_RECORD_FORMAT, INSTALLATION_RECORD_FORMAT, LinuxCandidateLoader,
    LinuxInstallStore, LinuxInstallStoreError,
};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
const MECHANISM: &str = "ed25519-subject-v1";
const IDENTITY_KEY: &str = "key.sha256";

struct TempTree(PathBuf);

impl TempTree {
    fn new(label: &str) -> Self {
        let index = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "invokrum-auth-install-{label}-{}-{index}",
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

struct FixedVerifier {
    assertion: PublisherAssertion,
}

impl PublisherVerifier for FixedVerifier {
    type Error = std::io::Error;

    fn verify(&self, _subject: &Sha256Digest) -> Result<PublisherAssertion, Self::Error> {
        Ok(self.assertion.clone())
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
                    u64::try_from(bytes.len()).expect("fixture length should fit"),
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

fn assertion(bundle_subject: &Sha256Digest, fingerprint: &str) -> PublisherAssertion {
    PublisherAssertion::new(
        VerificationMechanism::parse(MECHANISM).expect("fixture mechanism should be valid"),
        bundle_subject.clone(),
        PublisherIdentity::new(vec![(IDENTITY_KEY.to_owned(), fingerprint.to_owned())])
            .expect("fixture identity should be valid"),
    )
}

fn policy(fingerprints: &[&str]) -> TrustPolicy {
    TrustPolicy::new(
        fingerprints
            .iter()
            .map(|fingerprint| {
                TrustRule::new(
                    VerificationMechanism::parse(MECHANISM)
                        .expect("fixture mechanism should be valid"),
                    PublisherIdentity::new(vec![(
                        IDENTITY_KEY.to_owned(),
                        (*fingerprint).to_owned(),
                    )])
                    .expect("fixture identity should be valid"),
                )
            })
            .collect(),
    )
    .expect("fixture policy should be valid")
}

#[test]
fn authorized_local_install_persists_v2_evidence_and_reuses_exactly() {
    let candidate = TempTree::new("authorized-candidate");
    let store_root = TempTree::new("authorized-store");
    let pack = b"pack";
    write_candidate(candidate.path(), "pack.yaml", pack);
    let bundle_manifest = manifest(&[("pack.yaml", pack)]);
    let bundle_subject = subject('a');
    let fingerprint = "1".repeat(64);
    let verifier = FixedVerifier {
        assertion: assertion(&bundle_subject, &fingerprint),
    };
    let trust = policy(&[&fingerprint]);
    let loader = LinuxCandidateLoader::open(candidate.path()).expect("candidate should open");
    let store = LinuxInstallStore::open(store_root.path()).expect("store should open");

    let installed = install_authenticated_candidate(
        &verifier,
        &trust,
        &loader,
        &store,
        &bundle_manifest,
        &bundle_subject,
        &bundle_subject,
    )
    .expect("authorized candidate should install");
    assert!(!installed.reused());

    let record_bytes = fs::read(installed.record()).expect("record should exist");
    let record: serde_json::Value =
        serde_json::from_slice(&record_bytes).expect("record should be valid JSON");
    assert_eq!(record["format"], AUTHENTICATED_INSTALLATION_RECORD_FORMAT);
    assert_eq!(record["bundle_subject"], bundle_subject.as_str());
    assert_eq!(
        record["publisher_authentication"]["status"],
        "verified-and-authorized"
    );
    assert_eq!(record["publisher_authentication"]["mechanism"], MECHANISM);
    assert_eq!(
        record["publisher_authentication"]["subject"],
        bundle_subject.as_str()
    );
    assert_eq!(
        record["publisher_authentication"]["identity"][IDENTITY_KEY],
        fingerprint
    );

    let reused = install_authenticated_candidate(
        &verifier,
        &trust,
        &loader,
        &store,
        &bundle_manifest,
        &bundle_subject,
        &bundle_subject,
    )
    .expect("identical authenticated evidence should reuse exact root");
    assert!(reused.reused());
    assert_eq!(
        fs::read(reused.record()).expect("reused record should exist"),
        record_bytes
    );
}

#[test]
fn existing_unsigned_root_cannot_be_upgraded_in_place() {
    let candidate = TempTree::new("unsigned-first-candidate");
    let store_root = TempTree::new("unsigned-first-store");
    write_candidate(candidate.path(), "pack.yaml", b"pack");
    let bundle_manifest = manifest(&[("pack.yaml", b"pack")]);
    let bundle_subject = subject('b');
    let fingerprint = "2".repeat(64);
    let verifier = FixedVerifier {
        assertion: assertion(&bundle_subject, &fingerprint),
    };
    let trust = policy(&[&fingerprint]);
    let loader = LinuxCandidateLoader::open(candidate.path()).expect("candidate should open");
    let store = LinuxInstallStore::open(store_root.path()).expect("store should open");

    let unsigned = install_candidate(
        &loader,
        &store,
        &bundle_manifest,
        &bundle_subject,
        &bundle_subject,
    )
    .expect("unsigned candidate should install");
    let before = fs::read(unsigned.record()).expect("v1 record should exist");
    let before_json: serde_json::Value =
        serde_json::from_slice(&before).expect("v1 record should be JSON");
    assert_eq!(before_json["format"], INSTALLATION_RECORD_FORMAT);
    assert_eq!(before_json["publisher_authentication"], "not-provided");

    let result = install_authenticated_candidate(
        &verifier,
        &trust,
        &loader,
        &store,
        &bundle_manifest,
        &bundle_subject,
        &bundle_subject,
    );
    assert!(matches!(
        result,
        Err(AuthenticatedInstallWorkflowError::Store(
            LinuxInstallStoreError::ExistingInstallationMismatch
        ))
    ));
    assert_eq!(
        fs::read(unsigned.record()).expect("v1 record should remain"),
        before
    );
}

#[test]
fn existing_authenticated_root_cannot_be_downgraded_in_place() {
    let candidate = TempTree::new("authenticated-first-candidate");
    let store_root = TempTree::new("authenticated-first-store");
    write_candidate(candidate.path(), "pack.yaml", b"pack");
    let bundle_manifest = manifest(&[("pack.yaml", b"pack")]);
    let bundle_subject = subject('c');
    let fingerprint = "3".repeat(64);
    let verifier = FixedVerifier {
        assertion: assertion(&bundle_subject, &fingerprint),
    };
    let trust = policy(&[&fingerprint]);
    let loader = LinuxCandidateLoader::open(candidate.path()).expect("candidate should open");
    let store = LinuxInstallStore::open(store_root.path()).expect("store should open");

    let authenticated = install_authenticated_candidate(
        &verifier,
        &trust,
        &loader,
        &store,
        &bundle_manifest,
        &bundle_subject,
        &bundle_subject,
    )
    .expect("authenticated candidate should install");
    let before = fs::read(authenticated.record()).expect("v2 record should exist");

    let result = install_candidate(
        &loader,
        &store,
        &bundle_manifest,
        &bundle_subject,
        &bundle_subject,
    );
    assert_eq!(
        result,
        Err(InstallWorkflowError::Store(
            LinuxInstallStoreError::ExistingInstallationMismatch
        ))
    );
    assert_eq!(
        fs::read(authenticated.record()).expect("v2 record should remain"),
        before
    );
}

#[test]
fn existing_authenticated_root_cannot_change_publisher_identity() {
    let candidate = TempTree::new("different-signer-candidate");
    let store_root = TempTree::new("different-signer-store");
    write_candidate(candidate.path(), "pack.yaml", b"pack");
    let bundle_manifest = manifest(&[("pack.yaml", b"pack")]);
    let bundle_subject = subject('d');
    let fingerprint_a = "4".repeat(64);
    let fingerprint_b = "5".repeat(64);
    let trust = policy(&[&fingerprint_a, &fingerprint_b]);
    let verifier_a = FixedVerifier {
        assertion: assertion(&bundle_subject, &fingerprint_a),
    };
    let verifier_b = FixedVerifier {
        assertion: assertion(&bundle_subject, &fingerprint_b),
    };
    let loader = LinuxCandidateLoader::open(candidate.path()).expect("candidate should open");
    let store = LinuxInstallStore::open(store_root.path()).expect("store should open");

    let installed = install_authenticated_candidate(
        &verifier_a,
        &trust,
        &loader,
        &store,
        &bundle_manifest,
        &bundle_subject,
        &bundle_subject,
    )
    .expect("first authenticated publisher should install");
    let before = fs::read(installed.record()).expect("v2 record should exist");

    let result = install_authenticated_candidate(
        &verifier_b,
        &trust,
        &loader,
        &store,
        &bundle_manifest,
        &bundle_subject,
        &bundle_subject,
    );
    assert!(matches!(
        result,
        Err(AuthenticatedInstallWorkflowError::Store(
            LinuxInstallStoreError::ExistingInstallationMismatch
        ))
    ));
    assert_eq!(
        fs::read(installed.record()).expect("original v2 record should remain"),
        before
    );
}
