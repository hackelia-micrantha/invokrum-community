#![cfg(target_os = "linux")]

use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

fn example_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/governed-code-review")
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

fn parse_json(output: &Output) -> serde_json::Value {
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid JSON");
    assert_eq!(value["format"], "invokrum.cli/v1");
    value
}

#[test]
fn governed_code_review_example_matches_committed_contracts() {
    let pack = example_path("pack.yaml");
    let expected_context = example_path("expected/context.md");
    let expected_inspect = example_path("expected/inspect.json");
    let expected_lock = example_path("expected/invokrum.lock");

    let validate = invoke([
        argument("validate"),
        argument("--pack"),
        pack.as_os_str().to_owned(),
        argument("--profile"),
        argument("governed-review"),
        argument("--format"),
        argument("json"),
    ]);
    let validation = parse_json(&validate);
    assert_eq!(validation["command"], "validate");
    assert_eq!(validation["ok"], true);

    let compose = invoke([
        argument("compose"),
        argument("--pack"),
        pack.as_os_str().to_owned(),
        argument("--profile"),
        argument("governed-review"),
    ]);
    assert!(compose.status.success());
    assert!(compose.stderr.is_empty());
    assert_eq!(
        compose.stdout,
        fs::read(expected_context).expect("expected context should be readable")
    );

    let inspect = invoke([
        argument("inspect"),
        argument("--pack"),
        pack.as_os_str().to_owned(),
        argument("--profile"),
        argument("governed-review"),
        argument("--format"),
        argument("json"),
    ]);
    assert!(inspect.status.success());
    assert!(inspect.stderr.is_empty());
    assert_eq!(
        inspect.stdout,
        fs::read(expected_inspect).expect("expected inspect output should be readable")
    );

    let lock = invoke([
        argument("lock"),
        argument("--pack"),
        pack.as_os_str().to_owned(),
        argument("--profile"),
        argument("governed-review"),
    ]);
    assert!(lock.status.success());
    assert!(lock.stderr.is_empty());
    assert_eq!(
        lock.stdout,
        fs::read(&expected_lock).expect("expected lock should be readable")
    );

    let verify = invoke([
        argument("verify"),
        argument("--lock"),
        expected_lock.as_os_str().to_owned(),
        argument("--pack"),
        pack.as_os_str().to_owned(),
        argument("--profile"),
        argument("governed-review"),
        argument("--format"),
        argument("json"),
    ]);
    let verification = parse_json(&verify);
    assert_eq!(verification["command"], "verify");
    assert_eq!(verification["verified"], true);
    assert_eq!(verification["drifts"], serde_json::json!([]));

    let diff = invoke([
        argument("diff"),
        expected_lock.as_os_str().to_owned(),
        expected_lock.as_os_str().to_owned(),
        argument("--format"),
        argument("json"),
    ]);
    let difference = parse_json(&diff);
    assert_eq!(difference["command"], "diff");
    assert_eq!(difference["different"], false);
    assert_eq!(difference["changes"], serde_json::json!([]));
}

#[test]
fn invalid_example_profile_fails_on_declared_incompatibility() {
    let result = invoke([
        argument("compose"),
        argument("--pack"),
        example_path("pack.yaml").into_os_string(),
        argument("--profile"),
        argument("invalid-read-only-implementation"),
    ]);

    assert_eq!(result.status.code(), Some(invokrum_cli::EXIT_VALIDATION));
    assert!(result.stdout.is_empty());
    let stderr = String::from_utf8(result.stderr).expect("stderr should be UTF-8");
    assert!(stderr.contains("overlay `implementation` is incompatible with `read-only`"));
}
