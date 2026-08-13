use invokrum_acquisition::{CandidateFile, CandidateVerificationError, verify_candidate};
use invokrum_digest::sha256_lower_hex;
use invokrum_distribution::{
    BUNDLE_FORMAT, BundleFile, BundleLimits, BundleManifest, BundlePath, MAX_BUNDLE_EXPANDED_BYTES,
    MAX_BUNDLE_FILE_BYTES, MAX_BUNDLE_FILES, SHA256_ALGORITHM, Sha256Digest,
};

fn path(value: &str) -> BundlePath {
    BundlePath::parse(value).expect("fixture path should be valid")
}

fn digest(bytes: &[u8]) -> Sha256Digest {
    Sha256Digest::parse(sha256_lower_hex(bytes)).expect("fixture digest should be valid")
}

fn subject(character: char) -> Sha256Digest {
    Sha256Digest::parse(character.to_string().repeat(64)).expect("subject should be valid")
}

fn candidate(candidate_path: &str, bytes: Vec<u8>) -> CandidateFile {
    CandidateFile::new(path(candidate_path), bytes)
        .expect("fixture candidate should fit the v1 per-file bound")
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

#[test]
fn public_api_preserves_exact_verified_candidate_bytes() {
    let manifest = manifest(
        &[("pack.yaml", b"pack"), ("overlays/core.md", b"core")],
        "pack.yaml",
    );
    let expected = subject('a');
    let verified = verify_candidate(
        &manifest,
        &expected,
        &expected,
        vec![
            candidate("pack.yaml", b"pack".to_vec()),
            candidate("overlays/core.md", b"core".to_vec()),
        ],
    )
    .expect("candidate should verify");

    assert_eq!(verified.subject(), &expected);
    assert_eq!(verified.entry_point().as_str(), "pack.yaml");
    assert_eq!(verified.files()[0].path().as_str(), "overlays/core.md");
    assert_eq!(verified.files()[0].bytes(), b"core");
    assert_eq!(verified.files()[0].digest(), &digest(b"core"));
    assert_eq!(verified.files()[1].path().as_str(), "pack.yaml");
    assert_eq!(verified.files()[1].bytes(), b"pack");
}

#[test]
fn public_api_rejects_missing_extra_and_digest_drift_deterministically() {
    let manifest = manifest(
        &[("pack.yaml", b"pack"), ("overlays/core.md", b"core")],
        "pack.yaml",
    );
    let expected = subject('a');

    assert_eq!(
        verify_candidate(
            &manifest,
            &expected,
            &expected,
            vec![candidate("pack.yaml", b"pack".to_vec())],
        ),
        Err(CandidateVerificationError::MissingFile {
            path: path("overlays/core.md")
        })
    );

    assert_eq!(
        verify_candidate(
            &manifest,
            &expected,
            &expected,
            vec![
                candidate("pack.yaml", b"pack".to_vec()),
                candidate("overlays/core.md", b"core".to_vec()),
                candidate("a-extra.md", b"extra".to_vec()),
            ],
        ),
        Err(CandidateVerificationError::UndeclaredFile {
            path: path("a-extra.md")
        })
    );

    assert_eq!(
        verify_candidate(
            &manifest,
            &expected,
            &expected,
            vec![
                candidate("pack.yaml", b"pack".to_vec()),
                candidate("overlays/core.md", b"xxxx".to_vec()),
            ],
        ),
        Err(CandidateVerificationError::DigestMismatch {
            path: path("overlays/core.md")
        })
    );
}

#[test]
fn candidate_file_is_bounded_before_verification() {
    let candidate_path = path("pack.yaml");
    let oversized =
        vec![0_u8; usize::try_from(MAX_BUNDLE_FILE_BYTES + 1).expect("limit fits usize")];

    assert_eq!(
        CandidateFile::new(candidate_path.clone(), oversized),
        Err(CandidateVerificationError::FileTooLarge {
            path: candidate_path
        })
    );
}

#[test]
fn v1_candidate_set_resource_limits_fail_closed() {
    let expected = subject('a');
    let one_byte_manifest = manifest(&[("pack.yaml", b"x")], "pack.yaml");

    let too_many = (0..=MAX_BUNDLE_FILES)
        .map(|index| candidate(&format!("{index:03}.md"), Vec::new()))
        .collect();
    assert_eq!(
        verify_candidate(&one_byte_manifest, &expected, &expected, too_many,),
        Err(CandidateVerificationError::TooManyFiles)
    );

    let declared_byte = b"x";
    let aggregate_manifest = BundleManifest::new(
        BUNDLE_FORMAT,
        SHA256_ALGORITHM,
        path("00.md"),
        (0..33)
            .map(|index| BundleFile::new(path(&format!("{index:02}.md")), 1, digest(declared_byte)))
            .collect(),
        BundleLimits::default(),
    )
    .expect("33 one-byte declarations are within the manifest bound");
    let full_file = vec![b'x'; usize::try_from(MAX_BUNDLE_FILE_BYTES).expect("limit fits usize")];
    let candidates = (0..33)
        .map(|index| candidate(&format!("{index:02}.md"), full_file.clone()))
        .collect();

    assert_eq!(
        verify_candidate(&aggregate_manifest, &expected, &expected, candidates,),
        Err(CandidateVerificationError::BundleTooLarge)
    );
    assert_eq!(MAX_BUNDLE_EXPANDED_BYTES, 32 * 1024 * 1024);
}
