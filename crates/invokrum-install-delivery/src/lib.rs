//! Outer CLI/subprocess delivery adapter for explicit local bundle installation.
//!
//! This crate is intentionally outside deterministic composition. It owns install
//! command parsing, XDG/store-root resolution, local directory/archive source selection,
//! concrete publisher-verifier wiring, explicit host trust-policy construction, and
//! stable install result rendering. Existing acquisition and installation use cases
//! remain the authority for verification and persistence policy.

#![forbid(unsafe_code)]

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::path::PathBuf;

#[cfg(target_os = "linux")]
use std::fs::{self, File};
#[cfg(target_os = "linux")]
use std::io::Read;
#[cfg(target_os = "linux")]
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
#[cfg(target_os = "linux")]
use std::path::Path;

#[cfg(target_os = "linux")]
use invokrum_acquisition::{CandidateFile, VerifiedBundle};
#[cfg(target_os = "linux")]
use invokrum_acquisition_archive::{ArchiveCandidateError, UstarCandidateSource};
#[cfg(target_os = "linux")]
use invokrum_distribution::Sha256Digest;
#[cfg(target_os = "linux")]
use invokrum_distribution::{
    BundleLimits, BundleManifest, PublisherAssertion, PublisherIdentity, TrustPolicy, TrustRule,
    VerificationMechanism,
};
#[cfg(target_os = "linux")]
use invokrum_distribution_json::{
    MAX_BUNDLE_MANIFEST_BYTES, bundle_subject_digest, decode_bundle_manifest,
};
#[cfg(target_os = "linux")]
use invokrum_install::{
    AuthenticatedInstallWorkflowError, AuthenticatedVerifiedBundleStore, CandidateLoader,
    InstallWorkflowError, PublisherVerifier, VerifiedBundleStore, install_authenticated_candidate,
    install_candidate,
};
#[cfg(target_os = "linux")]
use invokrum_install_linux::{
    AUTHENTICATED_INSTALLATION_RECORD_FORMAT, INSTALLATION_RECORD_FORMAT, InstalledBundle,
    LinuxCandidateLoader, LinuxInstallStore, LinuxInstallStoreError, LocalCandidateError,
};
#[cfg(target_os = "linux")]
use invokrum_verifier_ed25519::{
    Ed25519SubjectVerifier, IDENTITY_KEY_SHA256, PUBLIC_KEY_BYTES, SIGNATURE_BYTES,
    VERIFICATION_MECHANISM, VerificationError,
};
#[cfg(target_os = "linux")]
use serde_json::{Value, json};

#[cfg(target_os = "linux")]
const CLI_JSON_FORMAT: &str = "invokrum.cli/v1";
#[cfg(target_os = "linux")]
const PUBLISHER_AUTHENTICATION_NOT_PROVIDED: &str = "not-provided";
#[cfg(target_os = "linux")]
const PUBLISHER_AUTHENTICATION_VERIFIED: &str = "verified-and-authorized";
#[cfg(target_os = "linux")]
const PRIVATE_DIRECTORY_MODE: u32 = 0o700;
#[cfg(target_os = "linux")]
const MAX_CLI_ARCHIVE_BYTES: usize = 36 * 1024 * 1024;

pub const USAGE: &str = "Invokrum explicit local installation\n\nUSAGE:\n  invokrum install <candidate> --bundle-manifest <path> --subject <sha256> [--candidate-format directory|ustar] [--store <path>] [--format human|json]\n  invokrum install <candidate> --bundle-manifest <path> --subject <sha256> [--candidate-format directory|ustar] --publisher-public-key <path> --publisher-signature <path> --trusted-key-sha256 <sha256> [--store <path>] [--format human|json]\n\nAUTHENTICATION:\n  Omitting all publisher options performs digest-pinned installation and records publisher_authentication: not-provided.\n  Authenticated mode requires all three publisher options and uses ed25519-subject-v1 plus an explicit host trust rule for --trusted-key-sha256.\n\nCANDIDATES:\n  directory  Fail-closed Linux local-directory candidate (default).\n  ustar      Bounded uncompressed POSIX ustar candidate; no filesystem extraction.\n";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    Usage,
    Input,
    Validation,
    Output,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstallError {
    kind: ErrorKind,
    message: String,
}

impl InstallError {
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        self.kind
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    fn new(kind: ErrorKind, message: impl fmt::Display) -> Self {
        Self {
            kind,
            message: message.to_string(),
        }
    }

    fn usage(message: impl fmt::Display) -> Self {
        Self::new(ErrorKind::Usage, message)
    }

    fn input(message: impl fmt::Display) -> Self {
        Self::new(ErrorKind::Input, message)
    }

    #[cfg(target_os = "linux")]
    fn validation(message: impl fmt::Display) -> Self {
        Self::new(ErrorKind::Validation, message)
    }

    #[cfg(target_os = "linux")]
    fn output(message: impl fmt::Display) -> Self {
        Self::new(ErrorKind::Output, message)
    }

    #[cfg(target_os = "linux")]
    fn internal(message: impl fmt::Display) -> Self {
        Self::new(ErrorKind::Internal, message)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstallExecution {
    stdout: Vec<u8>,
}

impl InstallExecution {
    #[must_use]
    pub fn into_stdout(self) -> Vec<u8> {
        self.stdout
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OutputFormat {
    Human,
    Json,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CandidateFormat {
    Directory,
    Ustar,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PublisherInputs {
    public_key: PathBuf,
    signature: PathBuf,
    trusted_key_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct InstallArgs {
    candidate: PathBuf,
    bundle_manifest: PathBuf,
    subject: String,
    candidate_format: CandidateFormat,
    store: Option<PathBuf>,
    format: OutputFormat,
    publisher: Option<PublisherInputs>,
}

#[cfg(target_os = "linux")]
#[derive(Clone, Debug, Eq, PartialEq)]
enum PublisherOutcome {
    NotProvided,
    VerifiedAndAuthorized { key_sha256: String },
}

#[cfg(target_os = "linux")]
#[derive(Clone, Debug, Eq, PartialEq)]
struct InstallOutcome {
    installation_format: &'static str,
    subject: Sha256Digest,
    root: PathBuf,
    record: PathBuf,
    reused: bool,
    publisher: PublisherOutcome,
}

#[must_use]
pub fn is_install_command(arguments: &[OsString]) -> bool {
    arguments
        .first()
        .is_some_and(|argument| argument == OsStr::new("install"))
}

/// Parses and executes the install-specific argument tail after the `install` token.
///
/// # Errors
///
/// Returns a stable semantic error kind for usage, input, validation, output/store,
/// or internal delivery failures. The caller owns final CLI exit-code mapping.
pub fn execute(arguments: &[OsString]) -> Result<InstallExecution, InstallError> {
    if arguments.len() == 1 && matches!(arguments[0].to_str(), Some("--help" | "-h" | "help")) {
        return Ok(InstallExecution {
            stdout: USAGE.as_bytes().to_vec(),
        });
    }

    let args = parse(arguments)?;

    #[cfg(not(target_os = "linux"))]
    {
        let _ = args;
        Err(InstallError::input(
            "local installation is currently supported on Linux only",
        ))
    }

    #[cfg(target_os = "linux")]
    {
        let outcome = install(&args)?;
        let stdout = match args.format {
            OutputFormat::Human => render_human(&outcome).into_bytes(),
            OutputFormat::Json => render_json(&outcome)?,
        };
        Ok(InstallExecution { stdout })
    }
}

fn parse(arguments: &[OsString]) -> Result<InstallArgs, InstallError> {
    let mut cursor = Cursor::new(arguments);
    let mut candidate = None;
    let mut bundle_manifest = None;
    let mut subject = None;
    let mut candidate_format = CandidateFormat::Directory;
    let mut candidate_format_seen = false;
    let mut store = None;
    let mut format = OutputFormat::Human;
    let mut format_seen = false;
    let mut publisher_public_key = None;
    let mut publisher_signature = None;
    let mut trusted_key_sha256 = None;

    while let Some(argument) = cursor.next() {
        match argument.to_str() {
            Some("--bundle-manifest") => assign(
                &mut bundle_manifest,
                cursor.path_value("--bundle-manifest")?,
                "--bundle-manifest",
            )?,
            Some("--subject") => {
                assign(&mut subject, cursor.string_value("--subject")?, "--subject")?;
            }
            Some("--candidate-format") => {
                if candidate_format_seen {
                    return Err(InstallError::usage(
                        "option `--candidate-format` was supplied more than once",
                    ));
                }
                candidate_format =
                    parse_candidate_format(&cursor.string_value("--candidate-format")?)?;
                candidate_format_seen = true;
            }
            Some("--store") => assign(&mut store, cursor.path_value("--store")?, "--store")?,
            Some("--format") => {
                if format_seen {
                    return Err(InstallError::usage(
                        "option `--format` was supplied more than once",
                    ));
                }
                format = parse_format(&cursor.string_value("--format")?)?;
                format_seen = true;
            }
            Some("--publisher-public-key") => assign(
                &mut publisher_public_key,
                cursor.path_value("--publisher-public-key")?,
                "--publisher-public-key",
            )?,
            Some("--publisher-signature") => assign(
                &mut publisher_signature,
                cursor.path_value("--publisher-signature")?,
                "--publisher-signature",
            )?,
            Some("--trusted-key-sha256") => assign(
                &mut trusted_key_sha256,
                cursor.string_value("--trusted-key-sha256")?,
                "--trusted-key-sha256",
            )?,
            Some("--no-color") => {}
            Some("--help" | "-h" | "help") => {
                return Err(InstallError::usage(
                    "install help must be requested without other arguments",
                ));
            }
            Some(value) if value.starts_with('-') => {
                return Err(InstallError::usage(format!(
                    "unknown install option `{value}`"
                )));
            }
            _ => assign(
                &mut candidate,
                PathBuf::from(argument.as_os_str()),
                "candidate",
            )?,
        }
    }

    Ok(InstallArgs {
        candidate: required(candidate, "candidate")?,
        bundle_manifest: required(bundle_manifest, "--bundle-manifest")?,
        subject: required(subject, "--subject")?,
        candidate_format,
        store,
        format,
        publisher: publisher_inputs(
            publisher_public_key,
            publisher_signature,
            trusted_key_sha256,
        )?,
    })
}

fn publisher_inputs(
    public_key: Option<PathBuf>,
    signature: Option<PathBuf>,
    trusted_key_sha256: Option<String>,
) -> Result<Option<PublisherInputs>, InstallError> {
    match (public_key, signature, trusted_key_sha256) {
        (None, None, None) => Ok(None),
        (Some(public_key), Some(signature), Some(trusted_key_sha256)) => {
            Ok(Some(PublisherInputs {
                public_key,
                signature,
                trusted_key_sha256,
            }))
        }
        _ => Err(InstallError::usage(
            "authenticated install requires --publisher-public-key, --publisher-signature, and --trusted-key-sha256 together",
        )),
    }
}

fn parse_candidate_format(value: &str) -> Result<CandidateFormat, InstallError> {
    match value {
        "directory" => Ok(CandidateFormat::Directory),
        "ustar" => Ok(CandidateFormat::Ustar),
        _ => Err(InstallError::usage(format!(
            "unsupported candidate format `{value}`; expected directory or ustar"
        ))),
    }
}

fn parse_format(value: &str) -> Result<OutputFormat, InstallError> {
    match value {
        "human" => Ok(OutputFormat::Human),
        "json" => Ok(OutputFormat::Json),
        _ => Err(InstallError::usage(format!(
            "unsupported output format `{value}`"
        ))),
    }
}

fn assign<T>(slot: &mut Option<T>, value: T, option: &str) -> Result<(), InstallError> {
    if slot.replace(value).is_some() {
        Err(InstallError::usage(format!(
            "option `{option}` was supplied more than once"
        )))
    } else {
        Ok(())
    }
}

fn required<T>(value: Option<T>, option: &str) -> Result<T, InstallError> {
    value.ok_or_else(|| InstallError::usage(format!("missing required option `{option}`")))
}

struct Cursor<'a> {
    arguments: &'a [OsString],
    index: usize,
}

impl<'a> Cursor<'a> {
    const fn new(arguments: &'a [OsString]) -> Self {
        Self {
            arguments,
            index: 0,
        }
    }

    fn next(&mut self) -> Option<&'a OsString> {
        let value = self.arguments.get(self.index)?;
        self.index += 1;
        Some(value)
    }

    fn path_value(&mut self, option: &str) -> Result<PathBuf, InstallError> {
        self.next()
            .map(|value| PathBuf::from(value.as_os_str()))
            .ok_or_else(|| InstallError::usage(format!("option `{option}` requires a value")))
    }

    fn string_value(&mut self, option: &str) -> Result<String, InstallError> {
        let value = self
            .next()
            .ok_or_else(|| InstallError::usage(format!("option `{option}` requires a value")))?;
        value
            .to_str()
            .map(ToOwned::to_owned)
            .ok_or_else(|| InstallError::usage(format!("option `{option}` requires valid UTF-8")))
    }
}

#[cfg(target_os = "linux")]
fn install(args: &InstallArgs) -> Result<InstallOutcome, InstallError> {
    let manifest_bytes = read_bounded_file(
        &args.bundle_manifest,
        MAX_BUNDLE_MANIFEST_BYTES,
        "bundle manifest",
    )?;
    let manifest =
        decode_bundle_manifest(&manifest_bytes, BundleLimits::default()).map_err(|error| {
            InstallError::validation(format!("invalid canonical bundle manifest: {error}"))
        })?;
    let manifest_subject = bundle_subject_digest(&manifest).map_err(|error| {
        InstallError::internal(format!(
            "could not derive canonical bundle subject: {error}"
        ))
    })?;
    let expected_subject = parse_digest_argument(&args.subject, "--subject")?;

    if manifest_subject != expected_subject {
        return Err(InstallError::validation(format!(
            "bundle subject mismatch: manifest is sha256:{} but --subject is sha256:{}",
            manifest_subject.as_str(),
            expected_subject.as_str()
        )));
    }

    let store_root = args.store.clone().map_or_else(default_store_root, Ok)?;
    let loader = CliCandidateLoader {
        path: args.candidate.clone(),
        format: args.candidate_format,
    };
    let store = CliInstallStore { root: store_root };

    match &args.publisher {
        None => install_digest_only(
            &loader,
            &store,
            &manifest,
            &manifest_subject,
            &expected_subject,
        ),
        Some(publisher) => install_authenticated(
            &loader,
            &store,
            &manifest,
            &manifest_subject,
            &expected_subject,
            publisher,
        ),
    }
}

#[cfg(target_os = "linux")]
fn install_digest_only(
    loader: &CliCandidateLoader,
    store: &CliInstallStore,
    manifest: &BundleManifest,
    manifest_subject: &Sha256Digest,
    expected_subject: &Sha256Digest,
) -> Result<InstallOutcome, InstallError> {
    let receipt = install_candidate(loader, store, manifest, manifest_subject, expected_subject)
        .map_err(map_install_error)?;
    Ok(outcome(
        &receipt,
        INSTALLATION_RECORD_FORMAT,
        PublisherOutcome::NotProvided,
    ))
}

#[cfg(target_os = "linux")]
fn install_authenticated(
    loader: &CliCandidateLoader,
    store: &CliInstallStore,
    manifest: &BundleManifest,
    manifest_subject: &Sha256Digest,
    expected_subject: &Sha256Digest,
    publisher: &PublisherInputs,
) -> Result<InstallOutcome, InstallError> {
    let public_key = read_exact_file(
        &publisher.public_key,
        PUBLIC_KEY_BYTES,
        "Ed25519 publisher public key",
    )?;
    let signature = read_exact_file(
        &publisher.signature,
        SIGNATURE_BYTES,
        "Ed25519 publisher signature",
    )?;
    let trusted_key_sha256 =
        parse_digest_argument(&publisher.trusted_key_sha256, "--trusted-key-sha256")?;
    let policy = single_ed25519_policy(&trusted_key_sha256)?;
    let verifier = CliEd25519Verifier {
        public_key,
        signature,
    };

    let receipt = install_authenticated_candidate(
        &verifier,
        &policy,
        loader,
        store,
        manifest,
        manifest_subject,
        expected_subject,
    )
    .map_err(map_authenticated_install_error)?;

    Ok(outcome(
        &receipt,
        AUTHENTICATED_INSTALLATION_RECORD_FORMAT,
        PublisherOutcome::VerifiedAndAuthorized {
            key_sha256: trusted_key_sha256.as_str().to_owned(),
        },
    ))
}

#[cfg(target_os = "linux")]
fn outcome(
    receipt: &InstalledBundle,
    installation_format: &'static str,
    publisher: PublisherOutcome,
) -> InstallOutcome {
    InstallOutcome {
        installation_format,
        subject: receipt.subject().clone(),
        root: receipt.root().to_path_buf(),
        record: receipt.record().to_path_buf(),
        reused: receipt.reused(),
        publisher,
    }
}

#[cfg(target_os = "linux")]
fn parse_digest_argument(value: &str, option: &str) -> Result<Sha256Digest, InstallError> {
    let digest = value.strip_prefix("sha256:").unwrap_or(value);
    Sha256Digest::parse(digest).map_err(|_| {
        InstallError::usage(format!(
            "{option} must be sha256:<64 lowercase hex> or the raw lowercase digest"
        ))
    })
}

#[cfg(target_os = "linux")]
fn single_ed25519_policy(key_sha256: &Sha256Digest) -> Result<TrustPolicy, InstallError> {
    let mechanism = VerificationMechanism::parse(VERIFICATION_MECHANISM).map_err(|error| {
        InstallError::internal(format!("invalid built-in verifier identifier: {error}"))
    })?;
    let identity = PublisherIdentity::new(vec![(
        IDENTITY_KEY_SHA256.to_owned(),
        key_sha256.as_str().to_owned(),
    )])
    .map_err(|error| InstallError::internal(format!("invalid built-in trust identity: {error}")))?;
    TrustPolicy::new(vec![TrustRule::new(mechanism, identity)])
        .map_err(|error| InstallError::internal(format!("invalid explicit trust policy: {error}")))
}

#[cfg(target_os = "linux")]
fn read_bounded_file(
    path: &Path,
    maximum_bytes: usize,
    label: &str,
) -> Result<Vec<u8>, InstallError> {
    let file = File::open(path)
        .map_err(|_| InstallError::input(format!("{label} is unavailable: {}", path.display())))?;
    let metadata = file.metadata().map_err(|_| {
        InstallError::input(format!(
            "{label} metadata is unavailable: {}",
            path.display()
        ))
    })?;
    if !metadata.is_file() {
        return Err(InstallError::input(format!(
            "{label} must be a regular file: {}",
            path.display()
        )));
    }

    let maximum_bytes = u64::try_from(maximum_bytes)
        .map_err(|_| InstallError::internal("file-size bound does not fit u64"))?;
    if metadata.len() > maximum_bytes {
        return Err(InstallError::input(format!(
            "{label} exceeds {maximum_bytes} bytes"
        )));
    }

    let mut bytes = Vec::new();
    file.take(maximum_bytes + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| {
            InstallError::input(format!("{label} could not be read: {}", path.display()))
        })?;
    if u64::try_from(bytes.len()).map_or(true, |length| length > maximum_bytes) {
        return Err(InstallError::input(format!(
            "{label} exceeds {maximum_bytes} bytes"
        )));
    }
    Ok(bytes)
}

#[cfg(target_os = "linux")]
fn read_exact_file(
    path: &Path,
    expected_bytes: usize,
    label: &str,
) -> Result<Vec<u8>, InstallError> {
    let bytes = read_bounded_file(path, expected_bytes, label)?;
    if bytes.len() != expected_bytes {
        return Err(InstallError::validation(format!(
            "{label} must contain exactly {expected_bytes} bytes"
        )));
    }
    Ok(bytes)
}

#[cfg(target_os = "linux")]
fn default_store_root() -> Result<PathBuf, InstallError> {
    data_home_from(std::env::var_os("XDG_DATA_HOME"), std::env::var_os("HOME"))
        .map(|data_home| data_home.join("invokrum").join("store"))
}

#[cfg(any(target_os = "linux", test))]
fn data_home_from(
    xdg_data_home: Option<OsString>,
    home: Option<OsString>,
) -> Result<PathBuf, InstallError> {
    if let Some(value) = xdg_data_home.filter(|value| !value.is_empty()) {
        let path = PathBuf::from(value);
        if path.is_absolute() {
            return Ok(path);
        }
    }

    if let Some(value) = home.filter(|value| !value.is_empty()) {
        let home = PathBuf::from(value);
        if home.is_absolute() {
            return Ok(home.join(".local").join("share"));
        }
    }

    Err(InstallError::input(
        "cannot resolve default store: set an absolute XDG_DATA_HOME or HOME, or pass --store",
    ))
}

#[cfg(target_os = "linux")]
fn prepare_store_root(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(_) => return Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => {
            return Err(format!(
                "installation store could not be inspected: {}",
                path.display()
            ));
        }
    }

    let parent = path.parent().ok_or_else(|| {
        format!(
            "installation store has no usable parent: {}",
            path.display()
        )
    })?;
    fs::create_dir_all(parent).map_err(|_| {
        format!(
            "installation store parent could not be created: {}",
            parent.display()
        )
    })?;

    let mut builder = fs::DirBuilder::new();
    builder.mode(PRIVATE_DIRECTORY_MODE);
    builder.create(path).map_err(|_| {
        format!(
            "installation store could not be created: {}",
            path.display()
        )
    })?;
    fs::set_permissions(path, fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE)).map_err(
        |_| {
            format!(
                "installation store permissions could not be secured: {}",
                path.display()
            )
        },
    )?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn render_human(outcome: &InstallOutcome) -> String {
    let mut text = format!(
        "subject: sha256:{}\nroot: {}\nrecord: {}\nreused: {}\n",
        outcome.subject.as_str(),
        outcome.root.display(),
        outcome.record.display(),
        outcome.reused,
    );
    match &outcome.publisher {
        PublisherOutcome::NotProvided => {
            text.push_str("publisher-authentication: not-provided\n");
        }
        PublisherOutcome::VerifiedAndAuthorized { key_sha256 } => {
            text.push_str("publisher-authentication: verified-and-authorized\n");
            text.push_str(&format!("publisher-mechanism: {VERIFICATION_MECHANISM}\n"));
            text.push_str(&format!("publisher-key-sha256: {key_sha256}\n"));
        }
    }
    text
}

#[cfg(target_os = "linux")]
fn render_json(outcome: &InstallOutcome) -> Result<Vec<u8>, InstallError> {
    let root = outcome.root.to_str().ok_or_else(|| {
        InstallError::output("installed root is not valid UTF-8; JSON output is unavailable")
    })?;
    let record = outcome.record.to_str().ok_or_else(|| {
        InstallError::output(
            "installation record path is not valid UTF-8; JSON output is unavailable",
        )
    })?;
    let publisher_authentication = match &outcome.publisher {
        PublisherOutcome::NotProvided => {
            Value::String(PUBLISHER_AUTHENTICATION_NOT_PROVIDED.to_owned())
        }
        PublisherOutcome::VerifiedAndAuthorized { key_sha256 } => json!({
            "status": PUBLISHER_AUTHENTICATION_VERIFIED,
            "mechanism": VERIFICATION_MECHANISM,
            "subject": outcome.subject.as_str(),
            "identity": {
                IDENTITY_KEY_SHA256: key_sha256,
            },
        }),
    };
    let mut bytes = serde_json::to_vec(&json!({
        "format": CLI_JSON_FORMAT,
        "command": "install",
        "installation_format": outcome.installation_format,
        "subject": format!("sha256:{}", outcome.subject.as_str()),
        "root": root,
        "record": record,
        "reused": outcome.reused,
        "publisher_authentication": publisher_authentication,
    }))
    .map_err(|_| InstallError::internal("installation result could not be serialized"))?;
    bytes.push(b'\n');
    Ok(bytes)
}

#[cfg(target_os = "linux")]
struct CliEd25519Verifier {
    public_key: Vec<u8>,
    signature: Vec<u8>,
}

#[cfg(target_os = "linux")]
impl PublisherVerifier for CliEd25519Verifier {
    type Error = VerificationError;

    fn verify(&self, subject: &Sha256Digest) -> Result<PublisherAssertion, Self::Error> {
        Ed25519SubjectVerifier::new().verify(subject, &self.public_key, &self.signature)
    }
}

#[cfg(target_os = "linux")]
struct CliCandidateLoader {
    path: PathBuf,
    format: CandidateFormat,
}

#[cfg(target_os = "linux")]
#[derive(Debug)]
enum CliCandidateLoadError {
    Directory(LocalCandidateError),
    ArchiveInput(String),
    Archive(ArchiveCandidateError),
}

#[cfg(target_os = "linux")]
impl fmt::Display for CliCandidateLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Directory(error) => write!(formatter, "{error}"),
            Self::ArchiveInput(error) => formatter.write_str(error),
            Self::Archive(error) => write!(formatter, "{error}"),
        }
    }
}

#[cfg(target_os = "linux")]
impl CandidateLoader for CliCandidateLoader {
    type Error = CliCandidateLoadError;

    fn load(&self, manifest: &BundleManifest) -> Result<Vec<CandidateFile>, Self::Error> {
        match self.format {
            CandidateFormat::Directory => {
                let loader = LinuxCandidateLoader::open(&self.path)
                    .map_err(CliCandidateLoadError::Directory)?;
                loader
                    .load(manifest)
                    .map_err(CliCandidateLoadError::Directory)
            }
            CandidateFormat::Ustar => {
                let bytes = read_bounded_file(&self.path, MAX_CLI_ARCHIVE_BYTES, "ustar candidate")
                    .map_err(|error| CliCandidateLoadError::ArchiveInput(error.message))?;
                let source =
                    UstarCandidateSource::new(bytes).map_err(CliCandidateLoadError::Archive)?;
                source
                    .load(manifest)
                    .map_err(CliCandidateLoadError::Archive)
            }
        }
    }
}

#[cfg(target_os = "linux")]
struct CliInstallStore {
    root: PathBuf,
}

#[cfg(target_os = "linux")]
#[derive(Debug)]
enum CliInstallStoreError {
    Prepare(String),
    Store(LinuxInstallStoreError),
}

#[cfg(target_os = "linux")]
impl fmt::Display for CliInstallStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Prepare(error) => formatter.write_str(error),
            Self::Store(error) => write!(formatter, "{error}"),
        }
    }
}

#[cfg(target_os = "linux")]
impl VerifiedBundleStore for CliInstallStore {
    type Receipt = InstalledBundle;
    type Error = CliInstallStoreError;

    fn install(&self, bundle: &VerifiedBundle) -> Result<Self::Receipt, Self::Error> {
        prepare_store_root(&self.root).map_err(CliInstallStoreError::Prepare)?;
        let store = LinuxInstallStore::open(&self.root).map_err(CliInstallStoreError::Store)?;
        VerifiedBundleStore::install(&store, bundle).map_err(CliInstallStoreError::Store)
    }
}

#[cfg(target_os = "linux")]
impl AuthenticatedVerifiedBundleStore for CliInstallStore {
    type Receipt = InstalledBundle;
    type Error = CliInstallStoreError;

    fn install_authenticated(
        &self,
        bundle: &VerifiedBundle,
        publisher: &invokrum_install::AuthorizedPublisher,
    ) -> Result<Self::Receipt, Self::Error> {
        prepare_store_root(&self.root).map_err(CliInstallStoreError::Prepare)?;
        let store = LinuxInstallStore::open(&self.root).map_err(CliInstallStoreError::Store)?;
        AuthenticatedVerifiedBundleStore::install_authenticated(&store, bundle, publisher)
            .map_err(CliInstallStoreError::Store)
    }
}

#[cfg(target_os = "linux")]
fn map_install_error(
    error: InstallWorkflowError<CliCandidateLoadError, CliInstallStoreError>,
) -> InstallError {
    match error {
        InstallWorkflowError::Verification(error) => {
            InstallError::validation(format!("candidate verification failed: {error}"))
        }
        InstallWorkflowError::Load(error) => {
            InstallError::input(format!("candidate loading failed: {error}"))
        }
        InstallWorkflowError::Store(error) => {
            InstallError::output(format!("bundle installation failed: {error}"))
        }
    }
}

#[cfg(target_os = "linux")]
fn map_authenticated_install_error(
    error: AuthenticatedInstallWorkflowError<
        VerificationError,
        CliCandidateLoadError,
        CliInstallStoreError,
    >,
) -> InstallError {
    match error {
        AuthenticatedInstallWorkflowError::CandidateVerification(error) => {
            InstallError::validation(format!("candidate verification failed: {error}"))
        }
        AuthenticatedInstallWorkflowError::PublisherVerification(error) => {
            InstallError::validation(format!("publisher verification failed: {error}"))
        }
        AuthenticatedInstallWorkflowError::PublisherAuthorization(error) => {
            InstallError::validation(format!("publisher authorization failed: {error}"))
        }
        AuthenticatedInstallWorkflowError::Load(error) => {
            InstallError::input(format!("candidate loading failed: {error}"))
        }
        AuthenticatedInstallWorkflowError::Store(error) => {
            InstallError::output(format!("bundle installation failed: {error}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn absolute_test_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!("invokrum-install-delivery-{label}"))
    }

    #[test]
    fn xdg_data_home_wins_when_absolute() {
        let xdg = absolute_test_path("xdg-data");
        let home = absolute_test_path("home");
        assert_eq!(
            data_home_from(
                Some(xdg.clone().into_os_string()),
                Some(home.into_os_string()),
            )
            .expect("absolute XDG data home should be accepted"),
            xdg
        );
    }

    #[test]
    fn relative_xdg_data_home_falls_back_to_home_default() {
        let home = absolute_test_path("home-default");
        assert_eq!(
            data_home_from(
                Some(OsString::from("relative/data")),
                Some(home.clone().into_os_string()),
            )
            .expect("relative XDG path should be ignored"),
            home.join(".local").join("share")
        );
    }

    #[test]
    fn missing_absolute_base_requires_explicit_store() {
        assert!(data_home_from(None, None).is_err());
        assert!(data_home_from(None, Some(OsString::from("relative-home"))).is_err());
    }

    #[test]
    fn parser_requires_complete_authentication_tuple() {
        let arguments = vec![
            OsString::from("candidate"),
            OsString::from("--bundle-manifest"),
            OsString::from("bundle.json"),
            OsString::from("--subject"),
            OsString::from("0".repeat(64)),
            OsString::from("--publisher-public-key"),
            OsString::from("key.bin"),
        ];
        let error = parse(&arguments).expect_err("partial authentication must fail");
        assert!(error.message().contains("requires --publisher-public-key"));
    }
}
