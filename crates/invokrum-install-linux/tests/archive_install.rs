#![cfg(target_os = "linux")]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use invokrum_acquisition::{CandidateFile, verify_candidate};
use invokrum_acquisition_archive::UstarCandidateSource;
use invokrum_core::{CompositionLimits, Identifier, compose};
use invokrum_digest::sha256_lower_hex;
use invokrum_distribution::{
    BUNDLE_FORMAT, BundleFile, BundleLimits, BundleManifest, BundlePath, PublisherAssertion,
    PublisherIdentity, SHA256_ALGORITHM, Sha256Digest, TrustPolicy, TrustRule,
    VerificationMechanism,
};
use invokrum_fs::LocalPackSource;
use invokrum_install::{
    CandidateLoader, PublisherVerifier, install_authenticated_candidate, install_candidate,
};
use invokrum_install_linux::{AUTHENTICATED_INSTALLATION_RECORD_FORMAT, LinuxInstallStore};
use invokrum_verifier_ed25519::{
    Ed25519SubjectVerifier, IDENTITY_KEY_SHA256, VERIFICATION_MECHANISM, VerificationError,
};

const BLOCK_BYTES: usize = 512;
const GOLDEN_PUBLIC_KEY: &str = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
const GOLDEN_SIGNATURE: &str = "9d2736ae5a0df81b19cfe74242ccdf98808c9bac8db90f33264e7d894ac33d5c522e7b54fe621b8e1919ab6bf6a04f81ab688ac8ab17e90abd22a45abde40f08";
const GOLDEN_FINGERPRINT: &str = "21fe31dfa154a261626bf854046fd2271b7bed4b6abe45aa58877ef47f9721b9";
static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

struct TempTree(PathBuf);

impl TempTree {
    fn new(label: &str) -> Self {
        let index = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "invokrum-archive-install-{label}-{}-{index}",
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

struct ArchiveLoader(UstarCandidateSource);

impl CandidateLoader for ArchiveLoader {
    type Error = invokrum_acquisition_archive::ArchiveCandidateError;

    fn load(&self, manifest: &BundleManifest) -> Result<Vec<CandidateFile>, Self::Error> {
        self.0.load(manifest)
    }
}

struct ConcreteVerifier {
    public_key: [u8; 32],
    signature: [u8; 64],
}

impl PublisherVerifier for ConcreteVerifier {
    type Error = VerificationError;

    fn verify(&self, subject: &Sha256Digest) -> Result<PublisherAssertion, Self::Error> {
        Ed25519SubjectVerifier::new().verify(subject, &self.public_key, &self.signature)
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

fn policy(fingerprint: &str) -> TrustPolicy {
    TrustPolicy::new(vec![TrustRule::new(
        VerificationMechanism::parse(VERIFICATION_MECHANISM)
            .expect("fixture mechanism should be valid"),
        PublisherIdentity::new(vec![(
            IDENTITY_KEY_SHA256.to_owned(),
            fingerprint.to_owned(),
        )])
        .expect("fixture identity should be valid"),
    )])
    .expect("fixture policy should be valid")
}

fn decode_hex<const N: usize>(value: &str) -> [u8; N] {
    assert_eq!(value.len(), N * 2, "fixture hex must have exact length");
    let mut bytes = [0_u8; N];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = hex_nibble(pair[0]);
        let low = hex_nibble(pair[1]);
        bytes[index] = (high << 4) | low;
    }
    bytes
}

fn hex_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => panic!("fixture contains non-hex byte"),
    }
}

fn write_octal(field: &mut [u8], value: u64) {
    field.fill(0);
    let width = field.len() - 1;
    let text = format!("{value:0width$o}");
    assert_eq!(text.len(), width);
    field[..width].copy_from_slice(text.as_bytes());
}

fn header(name: &[u8], kind: u8, size: u64) -> [u8; BLOCK_BYTES] {
    let mut block = [0_u8; BLOCK_BYTES];
    block[..name.len()].copy_from_slice(name);
    write_octal(&mut block[100..108], 0o644);
    write_octal(&mut block[108..116], 0);
    write_octal(&mut block[116..124], 0);
    write_octal(&mut block[124..136], size);
    write_octal(&mut block[136..148], 0);
    block[148..156].fill(b' ');
    block[156] = kind;
    block[257..263].copy_from_slice(b"ustar\0");
    block[263..265].copy_from_slice(b"00");
    write_octal(&mut block[329..337], 0);
    write_octal(&mut block[337..345], 0);
    let checksum = block.iter().map(|byte| u64::from(*byte)).sum::<u64>();
    let checksum_text = format!("{checksum:06o}\0 ");
    block[148..156].copy_from_slice(checksum_text.as_bytes());
    block
}

fn archive(entries: &[(&[u8], u8, &[u8])]) -> Vec<u8> {
    let mut archive = Vec::new();
    for (name, kind, data) in entries {
        archive.extend_from_slice(&header(
            name,
            *kind,
            u64::try_from(data.len()).expect("fixture length should fit"),
        ));
        archive.extend_from_slice(data);
        let padded = data.len().div_ceil(BLOCK_BYTES) * BLOCK_BYTES;
        archive.resize(archive.len() + padded - data.len(), 0);
    }
    archive.resize(archive.len() + BLOCK_BYTES * 2, 0);
    archive
}

fn fixture_pack() -> (&'static [u8], &'static [u8]) {
    (
        b"schema: invokrum.dev/v1\nid: archived-pack\nclasses:\n  - id: core\n    order: 10\n    minimum: 1\n    maximum: 1\noverlays:\n  - id: core-overlay\n    class: core\n    source: overlays/core.md\nprofiles:\n  - id: default\n    selections:\n      core:\n        - core-overlay\nvariables: []\n",
        b"# Archived overlay\nExact verified bytes.",
    )
}

fn assert_composes(root: &Path, expected: &[u8]) {
    let pack = invokrum_schema::parse_yaml(
        &fs::read_to_string(root.join("pack.yaml")).expect("installed pack should be readable"),
    )
    .expect("installed entry point should parse through the existing schema adapter");
    let local = LocalPackSource::open(root).expect("installed root should open");
    let composition = compose(
        &pack,
        &Identifier::parse("default").expect("fixture profile should parse"),
        &local,
        CompositionLimits::default(),
    )
    .expect("installed archive should compose through the unchanged runtime path");

    assert_eq!(composition.normalized_context(), expected);
}

#[test]
fn verified_archive_installs_and_composes_from_existing_store() {
    let store_root = TempTree::new("store");
    let (pack_bytes, overlay_bytes) = fixture_pack();
    let bundle_manifest = manifest(&[
        ("pack.yaml", pack_bytes),
        ("overlays/core.md", overlay_bytes),
    ]);
    let bundle_subject = subject('a');
    let source = UstarCandidateSource::new(archive(&[
        (b"overlays/", b'5', b""),
        (b"overlays/core.md", b'0', overlay_bytes),
        (b"pack.yaml", b'0', pack_bytes),
    ]))
    .expect("fixture archive should satisfy the bounded ustar profile");

    let verified = verify_candidate(
        &bundle_manifest,
        &bundle_subject,
        &bundle_subject,
        source
            .load(&bundle_manifest)
            .expect("archive adapter should return exact candidates"),
    )
    .expect("existing verifier should accept the archive bytes");
    assert_eq!(verified.subject(), &bundle_subject);

    let loader = ArchiveLoader(source);
    let store = LinuxInstallStore::open(store_root.path()).expect("store should open");
    let installed = install_candidate(
        &loader,
        &store,
        &bundle_manifest,
        &bundle_subject,
        &bundle_subject,
    )
    .expect("archive candidate should install through the existing store");

    assert_eq!(
        fs::read(installed.root().join("pack.yaml")).expect("pack should exist"),
        pack_bytes
    );
    assert_eq!(
        fs::read(installed.root().join("overlays/core.md")).expect("overlay should exist"),
        overlay_bytes
    );
    assert_composes(installed.root(), overlay_bytes);
}

#[test]
fn concrete_ed25519_archive_install_reaches_v2_store_and_existing_composition() {
    let store_root = TempTree::new("authenticated-store");
    let (pack_bytes, overlay_bytes) = fixture_pack();
    let bundle_manifest = manifest(&[
        ("pack.yaml", pack_bytes),
        ("overlays/core.md", overlay_bytes),
    ]);
    let bundle_subject = subject('0');
    let source = UstarCandidateSource::new(archive(&[
        (b"overlays/", b'5', b""),
        (b"overlays/core.md", b'0', overlay_bytes),
        (b"pack.yaml", b'0', pack_bytes),
    ]))
    .expect("fixture archive should satisfy the bounded ustar profile");
    let loader = ArchiveLoader(source);
    let verifier = ConcreteVerifier {
        public_key: decode_hex(GOLDEN_PUBLIC_KEY),
        signature: decode_hex(GOLDEN_SIGNATURE),
    };
    let store = LinuxInstallStore::open(store_root.path()).expect("store should open");

    let installed = install_authenticated_candidate(
        &verifier,
        &policy(GOLDEN_FINGERPRINT),
        &loader,
        &store,
        &bundle_manifest,
        &bundle_subject,
        &bundle_subject,
    )
    .expect("concretely verified archive should install through the existing store");

    let record: serde_json::Value = serde_json::from_slice(
        &fs::read(installed.record()).expect("installation record should be readable"),
    )
    .expect("installation record should be valid JSON");
    assert_eq!(record["format"], AUTHENTICATED_INSTALLATION_RECORD_FORMAT);
    assert_eq!(
        record["publisher_authentication"]["mechanism"],
        VERIFICATION_MECHANISM
    );
    assert_eq!(
        record["publisher_authentication"]["subject"],
        bundle_subject.as_str()
    );
    assert_eq!(
        record["publisher_authentication"]["identity"][IDENTITY_KEY_SHA256],
        GOLDEN_FINGERPRINT
    );
    assert_composes(installed.root(), overlay_bytes);
}
