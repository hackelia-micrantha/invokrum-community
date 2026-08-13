#![cfg(target_os = "linux")]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use invokrum_acquisition::{CandidateFile, verify_candidate};
use invokrum_digest::sha256_lower_hex;
use invokrum_distribution::{
    BUNDLE_FORMAT, BundleFile, BundleLimits, BundleManifest, BundlePath, MAX_BUNDLE_FILE_BYTES,
    MAX_BUNDLE_FILES, SHA256_ALGORITHM, Sha256Digest,
};
use invokrum_install::VerifiedBundleStore;
use invokrum_install_linux::{LinuxInstallStore, MAX_INSTALLATION_RECORD_BYTES};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

struct TempTree(PathBuf);

impl TempTree {
    fn new(label: &str) -> Self {
        let index = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "invokrum-install-record-limit-{label}-{}-{index}",
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

fn bundle_path(value: String) -> BundlePath {
    BundlePath::parse(value).expect("fixture path should satisfy the v1 path contract")
}

fn empty_digest() -> Sha256Digest {
    Sha256Digest::parse(sha256_lower_hex(&[])).expect("SHA-256 digest should parse")
}

fn subject() -> Sha256Digest {
    Sha256Digest::parse("a".repeat(64)).expect("fixture subject should parse")
}

fn quote_heavy_path(index: usize) -> BundlePath {
    let directory = "\"".repeat(200);
    let filename = format!("{}-{index:03}.md", "\"".repeat(200));
    bundle_path(format!(
        "{directory}/{directory}/{directory}/{directory}/{filename}"
    ))
}

#[test]
fn maximum_valid_file_set_can_persist_record_larger_than_one_bundle_file() {
    let paths = (0..MAX_BUNDLE_FILES)
        .map(quote_heavy_path)
        .collect::<Vec<_>>();
    let digest = empty_digest();
    let files = paths
        .iter()
        .map(|path| BundleFile::new(path.clone(), 0, digest.clone()))
        .collect::<Vec<_>>();
    let manifest = BundleManifest::new(
        BUNDLE_FORMAT,
        SHA256_ALGORITHM,
        paths[0].clone(),
        files,
        BundleLimits::default(),
    )
    .expect("maximum-count quote-heavy manifest should remain valid");
    let candidates = paths
        .iter()
        .map(|path| CandidateFile::new(path.clone(), Vec::new()).expect("empty file should fit"))
        .collect::<Vec<_>>();
    let bundle_subject = subject();
    let verified = verify_candidate(&manifest, &bundle_subject, &bundle_subject, candidates)
        .expect("valid maximum-count candidate should verify");

    let store_root = TempTree::new("store");
    let store = LinuxInstallStore::open(store_root.path()).expect("store root should open");
    let installed = VerifiedBundleStore::install(&store, &verified)
        .expect("valid bundle should install even when its evidence exceeds one bundle-file cap");

    let record = fs::read(installed.record()).expect("installation record should be readable");
    let record_length = u64::try_from(record.len()).expect("record length should fit u64");
    assert!(
        record_length > MAX_BUNDLE_FILE_BYTES,
        "fixture must reproduce the original >1 MiB evidence boundary"
    );
    assert!(record_length <= MAX_INSTALLATION_RECORD_BYTES);

    let reused = VerifiedBundleStore::install(&store, &verified)
        .expect("bounded record should also pass existing-install verification");
    assert!(reused.reused());
}
