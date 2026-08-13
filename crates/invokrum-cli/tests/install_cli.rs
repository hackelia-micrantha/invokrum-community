#![cfg(target_os = "linux")]

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

const PACK_BYTES: &[u8] = b"schema: invokrum.dev/v1\nid: install-fixture\n";
const BUNDLE_BYTES: &[u8] = br#"{"format":"invokrum.pack-bundle/v1","digest_algorithm":"sha256","entry_point":"pack.yaml","files":[{"path":"pack.yaml","byte_length":44,"digest":"5a8f876eca865b02f2640988d4b513a5f6e10a02cdca0bc149cf8707e377e912"}]}"#;
const SUBJECT: &str = "f5641dbec32c6c22f2e17a3f5fb532a18a0dcd9a6dd27301ab1e330810516bbb";
const PUBLIC_KEY: &str = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
const SIGNATURE: &str = "ce3060b09399734a23397e51615c37017ed9ff8f8b6597eeee5292b0f2a10eb2c621e24a86f3a3fb63dc9d4ca5ea6c61eed7a3f8e4b02ed7b015133b60b5720a";
const KEY_FINGERPRINT: &str = "21fe31dfa154a261626bf854046fd2271b7bed4b6abe45aa58877ef47f9721b9";
const BLOCK_BYTES: usize = 512;
static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

struct TempTree(PathBuf);

impl TempTree {
    fn new(label: &str) -> Self {
        let sequence = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "invokrum-install-cli-{label}-{}-{sequence}",
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

fn invoke(arguments: Vec<OsString>) -> Output {
    Command::new(env!("CARGO_BIN_EXE_invokrum"))
        .args(arguments)
        .output()
        .expect("invokrum binary should execute")
}

fn invoke_with_xdg(arguments: Vec<OsString>, data_home: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_invokrum"))
        .args(arguments)
        .env("XDG_DATA_HOME", data_home)
        .output()
        .expect("invokrum binary should execute")
}

fn argument(value: impl Into<OsString>) -> OsString {
    value.into()
}

fn setup_directory_candidate(temp: &TempTree) -> (PathBuf, PathBuf) {
    let candidate = temp.path().join("candidate");
    fs::create_dir(&candidate).expect("candidate root should be creatable");
    fs::write(candidate.join("pack.yaml"), PACK_BYTES).expect("candidate should be writable");
    let manifest = temp.path().join("bundle.json");
    fs::write(&manifest, BUNDLE_BYTES).expect("bundle manifest should be writable");
    (candidate, manifest)
}

fn subject_argument() -> String {
    format!("sha256:{SUBJECT}")
}

fn base_arguments(candidate: &Path, manifest: &Path, store: Option<&Path>) -> Vec<OsString> {
    let mut arguments = vec![
        argument("install"),
        candidate.as_os_str().to_owned(),
        argument("--bundle-manifest"),
        manifest.as_os_str().to_owned(),
        argument("--subject"),
        argument(subject_argument()),
    ];
    if let Some(store) = store {
        arguments.extend([argument("--store"), store.as_os_str().to_owned()]);
    }
    arguments.extend([argument("--format"), argument("json")]);
    arguments
}

fn json_output(output: &Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "install failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).expect("install result should be JSON")
}

#[test]
fn digest_only_directory_install_uses_xdg_store_and_reuses_exact_root() {
    let temp = TempTree::new("directory");
    let (candidate, manifest) = setup_directory_candidate(&temp);
    let data_home = temp.path().join("data");

    let first = invoke_with_xdg(base_arguments(&candidate, &manifest, None), &data_home);
    let first_json = json_output(&first);
    assert_eq!(first_json["format"], "invokrum.cli/v1");
    assert_eq!(first_json["command"], "install");
    assert_eq!(
        first_json["installation_format"],
        "invokrum.installation/v1"
    );
    assert_eq!(first_json["subject"], subject_argument());
    assert_eq!(first_json["publisher_authentication"], "not-provided");
    assert_eq!(first_json["reused"], false);

    let expected_root = data_home
        .join("invokrum")
        .join("store")
        .join("sha256")
        .join(SUBJECT);
    assert_eq!(
        PathBuf::from(first_json["root"].as_str().expect("root should be text")),
        expected_root
    );
    assert_eq!(
        fs::read(expected_root.join("pack.yaml")).expect("installed pack should exist"),
        PACK_BYTES
    );

    let second = invoke_with_xdg(base_arguments(&candidate, &manifest, None), &data_home);
    let second_json = json_output(&second);
    assert_eq!(second_json["root"], first_json["root"]);
    assert_eq!(second_json["record"], first_json["record"]);
    assert_eq!(second_json["reused"], true);
}

#[test]
fn bounded_ustar_install_uses_same_digest_only_evidence() {
    let temp = TempTree::new("ustar");
    let archive_path = temp.path().join("candidate.tar");
    fs::write(&archive_path, ustar_single_file(b"pack.yaml", PACK_BYTES))
        .expect("archive should be writable");
    let manifest = temp.path().join("bundle.json");
    fs::write(&manifest, BUNDLE_BYTES).expect("bundle manifest should be writable");
    let store = temp.path().join("store");

    let mut arguments = base_arguments(&archive_path, &manifest, Some(&store));
    let format_index = arguments.len() - 2;
    arguments.splice(
        format_index..format_index,
        [argument("--candidate-format"), argument("ustar")],
    );

    let result = json_output(&invoke(arguments));
    assert_eq!(result["installation_format"], "invokrum.installation/v1");
    assert_eq!(result["publisher_authentication"], "not-provided");
    let root = PathBuf::from(result["root"].as_str().expect("root should be text"));
    assert_eq!(
        fs::read(root.join("pack.yaml")).expect("archive pack should be installed"),
        PACK_BYTES
    );
}

#[test]
fn authenticated_directory_install_records_verified_and_authorized_v2_evidence() {
    let temp = TempTree::new("authenticated");
    let (candidate, manifest) = setup_directory_candidate(&temp);
    let store = temp.path().join("store");
    let key_path = temp.path().join("publisher.key");
    let signature_path = temp.path().join("publisher.sig");
    fs::write(&key_path, decode_hex(PUBLIC_KEY)).expect("public key should be writable");
    fs::write(&signature_path, decode_hex(SIGNATURE)).expect("signature should be writable");

    let mut arguments = base_arguments(&candidate, &manifest, Some(&store));
    let format_index = arguments.len() - 2;
    arguments.splice(
        format_index..format_index,
        [
            argument("--publisher-public-key"),
            key_path.as_os_str().to_owned(),
            argument("--publisher-signature"),
            signature_path.as_os_str().to_owned(),
            argument("--trusted-key-sha256"),
            argument(KEY_FINGERPRINT),
        ],
    );

    let result = json_output(&invoke(arguments));
    assert_eq!(result["installation_format"], "invokrum.installation/v2");
    assert_eq!(
        result["publisher_authentication"]["status"],
        "verified-and-authorized"
    );
    assert_eq!(
        result["publisher_authentication"]["mechanism"],
        "ed25519-subject-v1"
    );
    assert_eq!(
        result["publisher_authentication"]["identity"]["key.sha256"],
        KEY_FINGERPRINT
    );

    let record = PathBuf::from(result["record"].as_str().expect("record should be text"));
    let evidence: serde_json::Value =
        serde_json::from_slice(&fs::read(record).expect("installation record should be readable"))
            .expect("installation record should be JSON");
    assert_eq!(evidence["format"], "invokrum.installation/v2");
    assert_eq!(
        evidence["publisher_authentication"]["identity"]["key.sha256"],
        KEY_FINGERPRINT
    );
}

#[test]
fn unauthorized_publisher_fails_before_candidate_or_store_access() {
    let temp = TempTree::new("unauthorized");
    let manifest = temp.path().join("bundle.json");
    fs::write(&manifest, BUNDLE_BYTES).expect("bundle manifest should be writable");
    let missing_candidate = temp.path().join("missing-candidate");
    let store = temp.path().join("store");
    let key_path = temp.path().join("publisher.key");
    let signature_path = temp.path().join("publisher.sig");
    fs::write(&key_path, decode_hex(PUBLIC_KEY)).expect("public key should be writable");
    fs::write(&signature_path, decode_hex(SIGNATURE)).expect("signature should be writable");

    let mut arguments = base_arguments(&missing_candidate, &manifest, Some(&store));
    let format_index = arguments.len() - 2;
    arguments.splice(
        format_index..format_index,
        [
            argument("--publisher-public-key"),
            key_path.as_os_str().to_owned(),
            argument("--publisher-signature"),
            signature_path.as_os_str().to_owned(),
            argument("--trusted-key-sha256"),
            argument("0".repeat(64)),
        ],
    );

    let output = invoke(arguments);
    assert_eq!(output.status.code(), Some(4));
    let stderr = String::from_utf8(output.stderr).expect("diagnostic should be UTF-8");
    assert!(stderr.contains("publisher authorization failed"));
    assert!(!missing_candidate.exists());
    assert!(!store.exists());
}

#[test]
fn subject_mismatch_fails_before_candidate_or_store_access() {
    let temp = TempTree::new("subject-mismatch");
    let manifest = temp.path().join("bundle.json");
    fs::write(&manifest, BUNDLE_BYTES).expect("bundle manifest should be writable");
    let missing_candidate = temp.path().join("missing-candidate");
    let store = temp.path().join("store");

    let mut arguments = base_arguments(&missing_candidate, &manifest, Some(&store));
    let subject_index = arguments
        .iter()
        .position(|argument| argument == "--subject")
        .expect("subject option should exist");
    arguments[subject_index + 1] = argument("0".repeat(64));

    let output = invoke(arguments);
    assert_eq!(output.status.code(), Some(4));
    let stderr = String::from_utf8(output.stderr).expect("diagnostic should be UTF-8");
    assert!(stderr.contains("bundle subject mismatch"));
    assert!(!missing_candidate.exists());
    assert!(!store.exists());
}

fn decode_hex(value: &str) -> Vec<u8> {
    assert_eq!(value.len() % 2, 0);
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| (hex_nibble(pair[0]) << 4) | hex_nibble(pair[1]))
        .collect()
}

fn hex_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => panic!("fixture contains non-hex byte"),
    }
}

fn ustar_single_file(name: &[u8], data: &[u8]) -> Vec<u8> {
    let mut archive = Vec::new();
    archive.extend_from_slice(&ustar_header(
        name,
        u64::try_from(data.len()).expect("fixture length should fit"),
    ));
    archive.extend_from_slice(data);
    let padded = data.len().div_ceil(BLOCK_BYTES) * BLOCK_BYTES;
    archive.resize(archive.len() + padded - data.len(), 0);
    archive.resize(archive.len() + BLOCK_BYTES * 2, 0);
    archive
}

fn ustar_header(name: &[u8], size: u64) -> [u8; BLOCK_BYTES] {
    let mut block = [0_u8; BLOCK_BYTES];
    block[..name.len()].copy_from_slice(name);
    write_octal(&mut block[100..108], 0o644);
    write_octal(&mut block[108..116], 0);
    write_octal(&mut block[116..124], 0);
    write_octal(&mut block[124..136], size);
    write_octal(&mut block[136..148], 0);
    block[148..156].fill(b' ');
    block[156] = b'0';
    block[257..263].copy_from_slice(b"ustar\0");
    block[263..265].copy_from_slice(b"00");
    write_octal(&mut block[329..337], 0);
    write_octal(&mut block[337..345], 0);
    let checksum = block.iter().map(|byte| u64::from(*byte)).sum::<u64>();
    let checksum_text = format!("{checksum:06o}\0 ");
    block[148..156].copy_from_slice(checksum_text.as_bytes());
    block
}

fn write_octal(field: &mut [u8], value: u64) {
    field.fill(0);
    let width = field.len() - 1;
    let text = format!("{value:0width$o}");
    assert_eq!(text.len(), width);
    field[..width].copy_from_slice(text.as_bytes());
}
