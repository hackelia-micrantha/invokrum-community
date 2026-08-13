use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(target_os = "linux")]
use std::os::unix::fs::{PermissionsExt, symlink};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("invokrum-cli-{}-{sequence}", std::process::id()));
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
variables:
  - name: api-token
    sensitivity: secret
",
    )
    .expect("pack should be written");
    fs::write(directory.path().join("core.md"), b"core").expect("core overlay should be written");
    fs::write(directory.path().join("review.md"), b"review")
        .expect("review overlay should be written");
    directory
}

fn invoke(arguments: Vec<OsString>) -> Output {
    Command::new(env!("CARGO_BIN_EXE_invokrum"))
        .args(arguments)
        .output()
        .expect("invokrum binary should execute")
}

fn argument(value: impl Into<OsString>) -> OsString {
    value.into()
}

fn pack_path(directory: &TestDirectory) -> PathBuf {
    directory.path().join("pack.yaml")
}

#[test]
fn binary_reports_its_version() {
    let output = invoke(Vec::new());

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert_eq!(stdout, format!("invokrum {}\n", env!("CARGO_PKG_VERSION")));
}

#[test]
fn validate_compose_and_inspect_run_fully_offline() {
    let directory = fixture();
    let pack = pack_path(&directory);

    let validate = invoke(vec![
        argument("validate"),
        argument("--pack"),
        pack.as_os_str().to_owned(),
        argument("--profile"),
        argument("default"),
        argument("--format"),
        argument("json"),
    ]);
    assert!(validate.status.success());
    assert!(validate.stderr.is_empty());
    let validation: serde_json::Value =
        serde_json::from_slice(&validate.stdout).expect("validation output should be JSON");
    assert_eq!(validation["ok"], true);
    assert_eq!(validation["pack"], "example");
    assert_eq!(validation["profile"], "default");

    let compose = invoke(vec![
        argument("compose"),
        argument("--pack"),
        pack.as_os_str().to_owned(),
        argument("--profile"),
        argument("default"),
    ]);
    assert!(compose.status.success());
    assert!(compose.stderr.is_empty());
    assert_eq!(compose.stdout, b"core\n\nreview");

    let inspect = invoke(vec![
        argument("inspect"),
        argument("--pack"),
        pack.as_os_str().to_owned(),
        argument("--profile"),
        argument("default"),
        argument("--format"),
        argument("json"),
    ]);
    assert!(inspect.status.success());
    assert!(inspect.stderr.is_empty());
    let manifest: serde_json::Value =
        serde_json::from_slice(&inspect.stdout).expect("inspect output should be JSON");
    assert_eq!(manifest["pack"], "example");
    assert_eq!(manifest["profile"], "default");
    assert_eq!(
        manifest["entries"]
            .as_array()
            .expect("entries should be an array")
            .len(),
        2
    );
}

#[test]
fn lock_verify_and_diff_distinguish_repository_drift() {
    let directory = fixture();
    let pack = pack_path(&directory);
    let baseline = directory.path().join("baseline.lock");
    let candidate = directory.path().join("candidate.lock");

    let lock = invoke(vec![
        argument("lock"),
        argument("--pack"),
        pack.as_os_str().to_owned(),
        argument("--profile"),
        argument("default"),
        argument("--output"),
        baseline.as_os_str().to_owned(),
    ]);
    assert!(lock.status.success());
    assert!(lock.stdout.is_empty());
    assert!(lock.stderr.is_empty());

    let verified = invoke(vec![
        argument("verify"),
        argument("--lock"),
        baseline.as_os_str().to_owned(),
        argument("--pack"),
        pack.as_os_str().to_owned(),
        argument("--profile"),
        argument("default"),
        argument("--format"),
        argument("json"),
    ]);
    assert!(verified.status.success());
    assert!(verified.stderr.is_empty());
    let report: serde_json::Value =
        serde_json::from_slice(&verified.stdout).expect("verification output should be JSON");
    assert_eq!(report["verified"], true);

    fs::write(directory.path().join("review.md"), b"changed")
        .expect("review overlay should change");
    let drift = invoke(vec![
        argument("verify"),
        argument("--lock"),
        baseline.as_os_str().to_owned(),
        argument("--pack"),
        pack.as_os_str().to_owned(),
        argument("--profile"),
        argument("default"),
        argument("--format"),
        argument("json"),
    ]);
    assert_eq!(drift.status.code(), Some(invokrum_cli::EXIT_DRIFT));
    assert!(drift.stderr.is_empty());
    let report: serde_json::Value =
        serde_json::from_slice(&drift.stdout).expect("drift output should be JSON");
    assert_eq!(report["verified"], false);
    assert_eq!(
        report["drifts"]
            .as_array()
            .expect("drifts should be an array")
            .len(),
        2
    );

    let candidate_lock = invoke(vec![
        argument("lock"),
        argument("--pack"),
        pack.as_os_str().to_owned(),
        argument("--profile"),
        argument("default"),
        argument("--output"),
        candidate.as_os_str().to_owned(),
    ]);
    assert!(candidate_lock.status.success());

    let diff = invoke(vec![
        argument("diff"),
        baseline.as_os_str().to_owned(),
        candidate.as_os_str().to_owned(),
        argument("--format"),
        argument("json"),
    ]);
    assert_eq!(diff.status.code(), Some(invokrum_cli::EXIT_DRIFT));
    assert!(diff.stderr.is_empty());
    let difference: serde_json::Value =
        serde_json::from_slice(&diff.stdout).expect("diff output should be JSON");
    assert_eq!(difference["different"], true);
}

#[cfg(target_os = "linux")]
#[test]
fn output_is_private_atomic_and_never_clobbered_implicitly() {
    let directory = fixture();
    let pack = pack_path(&directory);
    let output_path = directory.path().join("context.txt");

    let first = invoke(vec![
        argument("compose"),
        argument("--pack"),
        pack.as_os_str().to_owned(),
        argument("--profile"),
        argument("default"),
        argument("--output"),
        output_path.as_os_str().to_owned(),
    ]);
    assert!(first.status.success());
    assert!(first.stdout.is_empty());
    assert!(first.stderr.is_empty());
    assert_eq!(
        fs::read(&output_path).expect("output should exist"),
        b"core\n\nreview"
    );
    assert_eq!(
        fs::metadata(&output_path)
            .expect("output metadata should exist")
            .permissions()
            .mode()
            & 0o777,
        0o600
    );

    fs::write(directory.path().join("review.md"), b"changed")
        .expect("review overlay should change");
    let rejected = invoke(vec![
        argument("compose"),
        argument("--pack"),
        pack.as_os_str().to_owned(),
        argument("--profile"),
        argument("default"),
        argument("--output"),
        output_path.as_os_str().to_owned(),
    ]);
    assert_eq!(rejected.status.code(), Some(invokrum_cli::EXIT_OUTPUT));
    assert!(rejected.stdout.is_empty());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("error[output]"));
    assert_eq!(
        fs::read(&output_path).expect("output should remain"),
        b"core\n\nreview"
    );

    let replaced = invoke(vec![
        argument("compose"),
        argument("--pack"),
        pack.as_os_str().to_owned(),
        argument("--profile"),
        argument("default"),
        argument("--output"),
        output_path.as_os_str().to_owned(),
        argument("--force"),
    ]);
    assert!(replaced.status.success());
    assert_eq!(
        fs::read(&output_path).expect("output should be replaced"),
        b"core\n\nchanged"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn symbolic_link_output_targets_are_rejected_without_touching_the_target() {
    let directory = fixture();
    let pack = pack_path(&directory);
    let real_target = directory.path().join("real.txt");
    let linked_output = directory.path().join("linked.txt");
    fs::write(&real_target, b"protected").expect("real target should be written");
    symlink(&real_target, &linked_output).expect("output symlink should be created");

    let result = invoke(vec![
        argument("compose"),
        argument("--pack"),
        pack.as_os_str().to_owned(),
        argument("--profile"),
        argument("default"),
        argument("--output"),
        linked_output.as_os_str().to_owned(),
        argument("--force"),
    ]);
    assert_eq!(result.status.code(), Some(invokrum_cli::EXIT_OUTPUT));
    assert!(result.stdout.is_empty());
    assert_eq!(
        fs::read(&real_target).expect("target should remain"),
        b"protected"
    );
}

#[test]
fn diagnostics_escape_controls_and_never_pollute_stdout() {
    let result = invoke(vec![argument("bad\ncommand")]);

    assert_eq!(result.status.code(), Some(invokrum_cli::EXIT_USAGE));
    assert!(result.stdout.is_empty());
    let stderr = String::from_utf8(result.stderr).expect("stderr should be UTF-8");
    assert_eq!(stderr.lines().count(), 1);
    assert!(stderr.contains("bad\\ncommand"));
    assert!(!stderr.contains("\u{1b}["));
}

#[test]
fn command_help_is_successful_and_contains_no_ansi() {
    let result = invoke(vec![argument("compose"), argument("--help")]);

    assert!(result.status.success());
    assert!(result.stderr.is_empty());
    let stdout = String::from_utf8(result.stdout).expect("help should be UTF-8");
    assert!(stdout.contains("USAGE:"));
    assert!(!stdout.contains("\u{1b}["));
}
