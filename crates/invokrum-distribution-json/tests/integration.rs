use invokrum_distribution::{
    BUNDLE_FORMAT, BundleFile, BundleLimits, BundleManifest, BundlePath, DistributionError,
    SHA256_ALGORITHM, Sha256Digest,
};
use invokrum_distribution_json::{
    BundleJsonError, bundle_subject_digest, decode_bundle_manifest, encode_bundle_manifest,
};

const GOLDEN: &[u8] = include_bytes!("../../../tests/fixtures/bundle/minimal-bundle.json");
const GOLDEN_SUBJECT: &str = "7e908eda57397f79992a6236bcfb6f2bef38fd9f28f51b4ef767d81357753f86";
const ZERO_DIGEST: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const ONE_DIGEST: &str = "1111111111111111111111111111111111111111111111111111111111111111";

fn digest(value: &str) -> Sha256Digest {
    Sha256Digest::parse(value).expect("fixture digest should be valid")
}

fn path(value: &str) -> BundlePath {
    BundlePath::parse(value).expect("fixture path should be valid")
}

#[test]
fn golden_manifest_round_trips_and_has_stable_subject_digest() {
    let manifest = decode_bundle_manifest(GOLDEN, BundleLimits::default())
        .expect("golden manifest should decode");

    assert_eq!(
        encode_bundle_manifest(&manifest).expect("manifest should encode"),
        GOLDEN
    );
    assert_eq!(
        bundle_subject_digest(&manifest)
            .expect("subject digest should be derived")
            .as_str(),
        GOLDEN_SUBJECT
    );
}

#[test]
fn equivalent_domain_input_normalizes_to_golden_bytes() {
    let manifest = BundleManifest::new(
        BUNDLE_FORMAT,
        SHA256_ALGORITHM,
        path("pack.yaml"),
        vec![
            BundleFile::new(path("pack.yaml"), 42, digest(ZERO_DIGEST)),
            BundleFile::new(path("overlays/core.md"), 12, digest(ONE_DIGEST)),
        ],
        BundleLimits::default(),
    )
    .expect("domain manifest should be valid");

    assert_eq!(
        encode_bundle_manifest(&manifest).expect("manifest should encode"),
        GOLDEN
    );
}

#[test]
fn unknown_fields_and_unsupported_versions_fail_closed() {
    let unknown = format!(
        "{{\"format\":\"{BUNDLE_FORMAT}\",\"digest_algorithm\":\"{SHA256_ALGORITHM}\",\"entry_point\":\"pack.yaml\",\"files\":[{{\"path\":\"pack.yaml\",\"byte_length\":1,\"digest\":\"{ZERO_DIGEST}\",\"publisher\":\"self\"}}]}}"
    );
    assert_eq!(
        decode_bundle_manifest(unknown.as_bytes(), BundleLimits::default()),
        Err(BundleJsonError::MalformedJson)
    );

    let unsupported = format!(
        "{{\"format\":\"invokrum.pack-bundle/v2\",\"digest_algorithm\":\"{SHA256_ALGORITHM}\",\"entry_point\":\"pack.yaml\",\"files\":[{{\"path\":\"pack.yaml\",\"byte_length\":1,\"digest\":\"{ZERO_DIGEST}\"}}]}}"
    );
    assert_eq!(
        decode_bundle_manifest(unsupported.as_bytes(), BundleLimits::default()),
        Err(BundleJsonError::Validation(
            DistributionError::UnsupportedBundleFormat
        ))
    );
}

#[test]
fn malformed_digest_missing_entry_point_and_duplicate_path_fail_closed() {
    let malformed_digest = format!(
        "{{\"format\":\"{BUNDLE_FORMAT}\",\"digest_algorithm\":\"{SHA256_ALGORITHM}\",\"entry_point\":\"pack.yaml\",\"files\":[{{\"path\":\"pack.yaml\",\"byte_length\":1,\"digest\":\"ABC\"}}]}}"
    );
    assert_eq!(
        decode_bundle_manifest(malformed_digest.as_bytes(), BundleLimits::default()),
        Err(BundleJsonError::Validation(
            DistributionError::InvalidDigest
        ))
    );

    let missing_entry = format!(
        "{{\"format\":\"{BUNDLE_FORMAT}\",\"digest_algorithm\":\"{SHA256_ALGORITHM}\",\"entry_point\":\"pack.yaml\",\"files\":[{{\"path\":\"other.yaml\",\"byte_length\":1,\"digest\":\"{ZERO_DIGEST}\"}}]}}"
    );
    assert_eq!(
        decode_bundle_manifest(missing_entry.as_bytes(), BundleLimits::default()),
        Err(BundleJsonError::Validation(
            DistributionError::EntryPointMissing
        ))
    );

    let duplicate = format!(
        "{{\"format\":\"{BUNDLE_FORMAT}\",\"digest_algorithm\":\"{SHA256_ALGORITHM}\",\"entry_point\":\"pack.yaml\",\"files\":[{{\"path\":\"pack.yaml\",\"byte_length\":1,\"digest\":\"{ZERO_DIGEST}\"}},{{\"path\":\"pack.yaml\",\"byte_length\":1,\"digest\":\"{ONE_DIGEST}\"}}]}}"
    );
    assert_eq!(
        decode_bundle_manifest(duplicate.as_bytes(), BundleLimits::default()),
        Err(BundleJsonError::Validation(
            DistributionError::DuplicatePath
        ))
    );
}

#[test]
fn limits_are_checked_before_domain_aggregate_construction() {
    let document = format!(
        "{{\"format\":\"{BUNDLE_FORMAT}\",\"digest_algorithm\":\"{SHA256_ALGORITHM}\",\"entry_point\":\"pack.yaml\",\"files\":[{{\"path\":\"pack.yaml\",\"byte_length\":2,\"digest\":\"{ZERO_DIGEST}\"}}]}}"
    );
    assert_eq!(
        decode_bundle_manifest(document.as_bytes(), BundleLimits::new(1, 1, 8)),
        Err(BundleJsonError::Validation(DistributionError::FileTooLarge))
    );
}

#[test]
fn noncanonical_file_order_is_rejected() {
    let document = format!(
        "{{\"format\":\"{BUNDLE_FORMAT}\",\"digest_algorithm\":\"{SHA256_ALGORITHM}\",\"entry_point\":\"pack.yaml\",\"files\":[{{\"path\":\"pack.yaml\",\"byte_length\":42,\"digest\":\"{ZERO_DIGEST}\"}},{{\"path\":\"overlays/core.md\",\"byte_length\":12,\"digest\":\"{ONE_DIGEST}\"}}]}}"
    );
    assert_eq!(
        decode_bundle_manifest(document.as_bytes(), BundleLimits::default()),
        Err(BundleJsonError::NonCanonical)
    );
}
