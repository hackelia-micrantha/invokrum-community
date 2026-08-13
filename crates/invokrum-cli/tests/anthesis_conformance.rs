#![cfg(target_os = "linux")]

use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

use serde_json::Value;

fn fixture_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/anthesis-conformance")
        .join(relative)
}

fn invoke(arguments: impl IntoIterator<Item = OsString>) -> Output {
    Command::new(env!("CARGO_BIN_EXE_invokrum"))
        .args(arguments)
        .output()
        .expect("invokrum binary should execute")
}

fn argument(value: impl Into<OsString>) -> OsString {
    value.into()
}

#[test]
fn conformance_pack_matches_pinned_reference_contract() {
    let reference: Value = serde_json::from_slice(
        &fs::read(fixture_path("reference-contract.json"))
            .expect("reference contract should be readable"),
    )
    .expect("reference contract should be valid JSON");
    assert_eq!(
        reference["anthesis_revision"],
        "116ad125f790fdfc592f186b546ac0dad4ffe148"
    );

    let pack = invokrum_schema::parse_yaml(
        &fs::read_to_string(fixture_path("pack.yaml")).expect("pack should be readable"),
    )
    .expect("conformance pack should validate");

    let actual_order: Vec<_> = pack
        .classes()
        .iter()
        .map(|class| class.id.as_str())
        .collect();
    let expected_order: Vec<_> = reference["class_order"]
        .as_array()
        .expect("class_order should be an array")
        .iter()
        .map(|value| value.as_str().expect("class id should be a string"))
        .collect();
    assert_eq!(actual_order, expected_order);

    for class_id in ["environment", "mode"] {
        let expected = &reference["fixture_cardinality"][class_id];
        let class = pack
            .classes()
            .iter()
            .find(|class| class.id.as_str() == class_id)
            .expect("reference class should exist");
        assert_eq!(
            u64::from(class.cardinality.minimum()),
            expected["minimum"]
                .as_u64()
                .expect("minimum should be numeric")
        );
        assert_eq!(
            class.cardinality.maximum().map(u64::from),
            Some(
                expected["maximum"]
                    .as_u64()
                    .expect("maximum should be numeric")
            )
        );
    }

    let read_only = pack
        .overlays()
        .iter()
        .find(|overlay| overlay.id.as_str() == "governance-read-only")
        .expect("read-only overlay should exist");
    assert!(
        read_only
            .incompatible_with
            .iter()
            .any(|overlay| overlay.as_str() == "mode-implementation")
    );
}

#[test]
fn synthetic_anthesis_audit_profile_matches_reference_bytes() {
    let result = invoke([
        argument("compose"),
        argument("--pack"),
        fixture_path("pack.yaml").into_os_string(),
        argument("--profile"),
        argument("audit-ci"),
    ]);

    assert!(result.status.success());
    assert!(result.stderr.is_empty());
    assert_eq!(
        result.stdout,
        fs::read(fixture_path("expected/audit-ci-context.md"))
            .expect("golden context should be readable")
    );
}

#[test]
fn read_only_governance_rejects_implementation_mode() {
    let result = invoke([
        argument("compose"),
        argument("--pack"),
        fixture_path("pack.yaml").into_os_string(),
        argument("--profile"),
        argument("invalid-read-only-implementation"),
    ]);

    assert_eq!(result.status.code(), Some(invokrum_cli::EXIT_VALIDATION));
    assert!(result.stdout.is_empty());
    let stderr = String::from_utf8(result.stderr).expect("stderr should be UTF-8");
    assert!(
        stderr
            .contains("overlay `governance-read-only` is incompatible with `mode-implementation`")
    );
}

#[test]
fn multiple_modes_fail_for_cardinality() {
    assert_cardinality_failure("invalid/multiple-mode.yaml", "mode", 2);
}

#[test]
fn multiple_environments_fail_for_cardinality() {
    assert_cardinality_failure("invalid/multiple-environment.yaml", "environment", 2);
}

fn assert_cardinality_failure(relative_pack: &str, class: &str, count: usize) {
    let result = invoke([
        argument("validate"),
        argument("--pack"),
        fixture_path(relative_pack).into_os_string(),
        argument("--profile"),
        argument("invalid"),
    ]);

    assert_eq!(result.status.code(), Some(invokrum_cli::EXIT_VALIDATION));
    assert!(result.stdout.is_empty());
    let stderr = String::from_utf8(result.stderr).expect("stderr should be UTF-8");
    assert!(stderr.contains("CardinalityViolation"), "stderr: {stderr}");
    assert!(stderr.contains(class), "stderr: {stderr}");
    assert!(
        stderr.contains(&format!("count: {count}")),
        "stderr: {stderr}"
    );
}
