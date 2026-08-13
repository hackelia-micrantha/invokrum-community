use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Value, json};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("invokrum-rpc-{}-{sequence}", std::process::id()));
        fs::create_dir_all(&path).expect("test directory should be created");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fixture() -> TestDirectory {
    let directory = TestDirectory::new();
    fs::write(
        directory.path().join("pack.yaml"),
        r"schema: invokrum.dev/v1
id: example
classes:
  - id: core
    order: 10
    minimum: 1
    maximum: 1
  - id: mode
    order: 20
    minimum: 1
    maximum: 1
overlays:
  - id: core-default
    class: core
    source: core.md
  - id: review
    class: mode
    source: review.md
profiles:
  - id: default
    selections:
      core:
        - core-default
      mode:
        - review
variables: []
",
    )
    .expect("pack should be written");
    fs::write(directory.path().join("core.md"), b"core").expect("core should be written");
    fs::write(directory.path().join("review.md"), b"review").expect("review should be written");
    directory
}

fn invoke(request: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_invokrum"))
        .arg("rpc")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("invokrum should start");
    child
        .stdin
        .as_mut()
        .expect("stdin should be available")
        .write_all(request)
        .expect("request should be written");
    child.wait_with_output().expect("invokrum should finish")
}

fn response(output: &Output) -> Value {
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    assert_eq!(output.stdout.last(), Some(&b'\n'));
    serde_json::from_slice(&output.stdout).expect("stdout should be one JSON response")
}

fn request(operation: &Value) -> Vec<u8> {
    serde_json::to_vec(operation).expect("request should encode")
}

#[test]
fn capabilities_are_explicitly_read_only() {
    let output = invoke(&request(&json!({
        "protocol": "invokrum.host/v1",
        "request_id": "cap-1",
        "operation": "capabilities"
    })));
    assert!(output.status.success());
    let response = response(&output);
    assert_eq!(response["format"], "invokrum.host/v1");
    assert_eq!(response["request_id"], "cap-1");
    assert_eq!(response["result"]["network_access"], false);
    assert_eq!(response["result"]["persistent_writes"], false);
    assert_eq!(response["result"]["runtime_invocation"], false);
}

#[test]
fn resolve_returns_exact_context_and_canonical_lock_bytes() {
    let directory = fixture();
    let output = invoke(&request(&json!({
        "protocol": "invokrum.host/v1",
        "request_id": "resolve-1",
        "operation": "resolve",
        "pack": directory.path().join("pack.yaml"),
        "profile": "default"
    })));
    assert!(output.status.success());
    let response = response(&output);
    assert_eq!(response["ok"], true);
    assert_eq!(response["operation"], "resolve");
    assert_eq!(response["result"]["context_base64"], "Y29yZQoKcmV2aWV3");
    assert_eq!(response["result"]["manifest"]["output_bytes"], 12);
    assert!(
        response["result"]["lock_base64"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );
    assert_eq!(
        response["result"]["output_digest"].as_str().map(str::len),
        Some(64)
    );
}

#[test]
fn verify_reports_no_drift_and_then_ordered_content_drift() {
    let directory = fixture();
    let resolve = invoke(&request(&json!({
        "protocol": "invokrum.host/v1",
        "request_id": "resolve-2",
        "operation": "resolve",
        "pack": directory.path().join("pack.yaml"),
        "profile": "default"
    })));
    assert!(resolve.status.success());
    let resolved = response(&resolve);
    let expected_lock = resolved["result"]["lock_base64"]
        .as_str()
        .expect("lock should be present");

    let verify = |request_id: &str| {
        invoke(&request(&json!({
            "protocol": "invokrum.host/v1",
            "request_id": request_id,
            "operation": "verify",
            "pack": directory.path().join("pack.yaml"),
            "profile": "default",
            "expected_lock_base64": expected_lock
        })))
    };

    let unchanged = verify("verify-1");
    assert!(unchanged.status.success());
    let unchanged = response(&unchanged);
    assert_eq!(unchanged["result"]["verified"], true);
    assert_eq!(unchanged["result"]["drifts"], json!([]));

    fs::write(directory.path().join("review.md"), b"changed").expect("overlay should change");
    let changed = verify("verify-2");
    assert!(changed.status.success());
    let changed = response(&changed);
    assert_eq!(changed["result"]["verified"], false);
    assert_eq!(
        changed["result"]["drifts"],
        json!([
            { "index": 1, "kind": "overlay_content" },
            { "kind": "rendered_output" }
        ])
    );
}

#[test]
fn duplicate_or_unknown_request_fields_fail_as_one_machine_response() {
    for request in [
        br#"{"protocol":"invokrum.host/v1","protocol":"invokrum.host/v1","request_id":"dup","operation":"capabilities"}"#.as_slice(),
        br#"{"protocol":"invokrum.host/v1","request_id":"unknown","operation":"capabilities","write":true}"#,
    ] {
        let output = invoke(request);
        assert_eq!(output.status.code(), Some(3));
        let response = response(&output);
        assert_eq!(response["ok"], false);
        assert_eq!(response["error"]["code"], "request");
    }
}

#[test]
fn unsupported_protocol_and_malformed_base64_fail_closed() {
    let directory = fixture();
    let unsupported = invoke(&request(&json!({
        "protocol": "invokrum.host/v2",
        "request_id": "future",
        "operation": "capabilities"
    })));
    assert_eq!(unsupported.status.code(), Some(3));
    assert_eq!(response(&unsupported)["error"]["code"], "request");

    let malformed = invoke(&request(&json!({
        "protocol": "invokrum.host/v1",
        "request_id": "bad-lock",
        "operation": "verify",
        "pack": directory.path().join("pack.yaml"),
        "profile": "default",
        "expected_lock_base64": "AB=="
    })));
    assert_eq!(malformed.status.code(), Some(3));
    assert_eq!(response(&malformed)["error"]["code"], "request");
}
