use std::path::Path;
use std::str;

use invokrum_core::{
    Composition, CompositionError, CompositionLimits, Identifier, OverlayPack, OverlaySource,
    PackRelativePath, compose,
};
use invokrum_fs::LocalPackSource;
use invokrum_integrity::{
    DriftKind, Lockfile, MAX_LOCKFILE_BYTES, Sha256Digester, build_lockfile, decode_lockfile,
    encode_lockfile, verify,
};
use serde_json::{Value, json};

use crate::args::{Command, Destination, OutputFormat, PackSelection, USAGE};
use crate::output;
use crate::{CliError, Execution, escape_human};

const MAX_PACK_BYTES: usize = 1_048_576;
const CLI_JSON_FORMAT: &str = "invokrum.cli/v1";

pub(crate) fn execute(command: Command) -> Result<Execution, CliError> {
    match command {
        Command::Version => Ok(Execution::success(
            format!("invokrum {}\n", env!("CARGO_PKG_VERSION")).into_bytes(),
        )),
        Command::Help => Ok(Execution::success(USAGE.as_bytes().to_vec())),
        Command::Validate {
            pack,
            profile,
            format,
        } => validate(&pack, profile.as_deref(), format),
        Command::Compose {
            selection,
            destination,
        } => compose_command(&selection, &destination),
        Command::Inspect { selection, format } => inspect(&selection, format),
        Command::Lock {
            selection,
            destination,
        } => lock(&selection, &destination),
        Command::Verify {
            lock,
            selection,
            format,
        } => verify_command(&lock, &selection, format),
        Command::Diff {
            baseline,
            candidate,
            format,
        } => diff(&baseline, &candidate, format),
        Command::Rpc => Err(CliError::internal(
            "rpc must be routed through the explicit stdin boundary",
        )),
    }
}

fn validate(
    pack_path: &Path,
    profile: Option<&str>,
    format: OutputFormat,
) -> Result<Execution, CliError> {
    let loaded = load_pack(pack_path)?;
    let profile = profile.map(parse_profile).transpose()?;
    if let Some(profile) = &profile {
        require_profile(&loaded.pack, profile)?;
    }

    let stdout = match format {
        OutputFormat::Human => {
            let profile = profile
                .as_ref()
                .map_or_else(|| "all".to_owned(), ToString::to_string);
            format!("valid pack={} profile={}\n", loaded.pack.id, profile).into_bytes()
        }
        OutputFormat::Json => json_line(&json!({
            "command": "validate",
            "ok": true,
            "pack": loaded.pack.id.to_string(),
            "profile": profile.map(|value| value.to_string()),
            "schema": loaded.pack.schema_family.as_str(),
        }))?,
    };
    Ok(Execution::success(stdout))
}

fn compose_command(
    selection: &PackSelection,
    destination: &Destination,
) -> Result<Execution, CliError> {
    let current = compose_selection(selection)?;
    deliver(current.composition.normalized_context(), destination)
}

fn inspect(selection: &PackSelection, format: OutputFormat) -> Result<Execution, CliError> {
    let current = compose_selection(selection)?;
    let stdout = match format {
        OutputFormat::Human => inspect_human(&current.composition),
        OutputFormat::Json => inspect_json(&current.composition)?,
    };
    Ok(Execution::success(stdout))
}

fn lock(selection: &PackSelection, destination: &Destination) -> Result<Execution, CliError> {
    let current = compose_selection(selection)?;
    let lock = build_lockfile(&current.pack, &current.composition, &Sha256Digester)
        .map_err(CliError::integrity)?;
    let bytes = encode_lockfile(&lock).map_err(CliError::integrity)?;
    deliver(&bytes, destination)
}

fn verify_command(
    lock_path: &Path,
    selection: &PackSelection,
    format: OutputFormat,
) -> Result<Execution, CliError> {
    let expected = load_lock(lock_path)?;
    let current = compose_selection(selection)?;
    let report =
        verify(&expected, &current.pack, &current.composition).map_err(CliError::integrity)?;
    let code = if report.is_verified() {
        crate::EXIT_SUCCESS
    } else {
        crate::EXIT_DRIFT
    };
    let stdout = match format {
        OutputFormat::Human => {
            if report.is_verified() {
                b"verified\n".to_vec()
            } else {
                let mut text = String::from("drift detected\n");
                for drift in report.drifts() {
                    text.push_str("- ");
                    text.push_str(&drift_human(*drift));
                    text.push('\n');
                }
                text.into_bytes()
            }
        }
        OutputFormat::Json => json_line(&json!({
            "command": "verify",
            "drifts": report.drifts().iter().map(|drift| drift_json(*drift)).collect::<Vec<_>>(),
            "verified": report.is_verified(),
        }))?,
    };
    Ok(Execution { code, stdout })
}

fn diff(
    baseline_path: &Path,
    candidate_path: &Path,
    format: OutputFormat,
) -> Result<Execution, CliError> {
    let baseline = load_lock(baseline_path)?;
    let candidate = load_lock(candidate_path)?;
    let differences = compare_locks(&baseline, &candidate);
    let code = if differences.is_empty() {
        crate::EXIT_SUCCESS
    } else {
        crate::EXIT_DRIFT
    };
    let stdout = match format {
        OutputFormat::Human => {
            if differences.is_empty() {
                b"identical\n".to_vec()
            } else {
                let mut text = String::from("lockfiles differ\n");
                for difference in &differences {
                    text.push_str("- ");
                    text.push_str(&difference.human());
                    text.push('\n');
                }
                text.into_bytes()
            }
        }
        OutputFormat::Json => json_line(&json!({
            "changes": differences.iter().map(Difference::json).collect::<Vec<_>>(),
            "command": "diff",
            "different": !differences.is_empty(),
        }))?,
    };
    Ok(Execution { code, stdout })
}

fn deliver(bytes: &[u8], destination: &Destination) -> Result<Execution, CliError> {
    if let Some(path) = &destination.output {
        output::write_atomic(path, bytes, destination.force).map_err(|error| {
            CliError::output(format!("output `{}` failed: {error}", path.display()))
        })?;
        Ok(Execution::success(Vec::new()))
    } else {
        Ok(Execution::success(bytes.to_vec()))
    }
}

struct LoadedPack {
    pack: OverlayPack,
    source: LocalPackSource,
}

struct CurrentComposition {
    pack: OverlayPack,
    composition: Composition,
}

fn compose_selection(selection: &PackSelection) -> Result<CurrentComposition, CliError> {
    let loaded = load_pack(&selection.pack)?;
    let profile = parse_profile(&selection.profile)?;
    let composition = compose(
        &loaded.pack,
        &profile,
        &loaded.source,
        CompositionLimits::default(),
    )
    .map_err(composition_error)?;
    Ok(CurrentComposition {
        pack: loaded.pack,
        composition,
    })
}

fn load_pack(path: &Path) -> Result<LoadedPack, CliError> {
    let (source, bytes) = read_local(path, MAX_PACK_BYTES)?;
    let text =
        str::from_utf8(&bytes).map_err(|_| CliError::input("pack document must be valid UTF-8"))?;
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let pack = match extension.as_str() {
        "json" => invokrum_schema::parse_json(text),
        "yaml" | "yml" => invokrum_schema::parse_yaml(text),
        _ if text.trim_start().starts_with('{') => invokrum_schema::parse_json(text),
        _ => invokrum_schema::parse_yaml(text),
    }
    .map_err(CliError::validation)?;

    Ok(LoadedPack { pack, source })
}

fn load_lock(path: &Path) -> Result<Lockfile, CliError> {
    let (_, bytes) = read_local(path, MAX_LOCKFILE_BYTES)?;
    decode_lockfile(&bytes).map_err(CliError::integrity)
}

fn read_local(path: &Path, maximum_bytes: usize) -> Result<(LocalPackSource, Vec<u8>), CliError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| CliError::input("input path must name a UTF-8 file"))?;
    let relative = PackRelativePath::parse(file_name.to_owned())
        .map_err(|_| CliError::input("input file name violates the portable path policy"))?;
    let source = LocalPackSource::open(parent).map_err(CliError::input)?;
    let bytes = source.load(&relative, maximum_bytes).map_err(|error| {
        CliError::input(format!(
            "input `{}` was rejected: {}",
            error.path.as_str(),
            error.kind
        ))
    })?;
    Ok((source, bytes))
}

fn composition_error(error: CompositionError) -> CliError {
    match error {
        CompositionError::Source(failure) => CliError::composition(format!(
            "overlay source `{}` was rejected: {}",
            failure.path.as_str(),
            failure.kind
        )),
        other => CliError::composition(other),
    }
}

fn parse_profile(value: &str) -> Result<Identifier, CliError> {
    Identifier::parse(value.to_owned())
        .map_err(|_| CliError::validation("profile identifier is invalid"))
}

fn require_profile(pack: &OverlayPack, profile: &Identifier) -> Result<(), CliError> {
    if pack
        .profiles()
        .iter()
        .any(|candidate| &candidate.id == profile)
    {
        Ok(())
    } else {
        Err(CliError::validation(format!("unknown profile `{profile}`")))
    }
}

fn inspect_human(composition: &Composition) -> Vec<u8> {
    let manifest = composition.manifest();
    let mut text = format!(
        "pack: {}\nprofile: {}\nsource-bytes: {}\noutput-bytes: {}\n",
        manifest.pack, manifest.profile, manifest.source_bytes, manifest.output_bytes
    );
    for entry in &manifest.entries {
        text.push_str(&format!(
            "- class={} overlay={} source={} bytes={}\n",
            entry.class,
            entry.overlay,
            escape_human(entry.source.as_str()),
            entry.byte_length
        ));
    }
    text.into_bytes()
}

fn inspect_json(composition: &Composition) -> Result<Vec<u8>, CliError> {
    let manifest = composition.manifest();
    json_line(&json!({
        "command": "inspect",
        "entries": manifest.entries.iter().map(|entry| json!({
            "byte_length": entry.byte_length,
            "class": entry.class.to_string(),
            "overlay": entry.overlay.to_string(),
            "source": entry.source.as_str(),
        })).collect::<Vec<_>>(),
        "output_bytes": manifest.output_bytes,
        "pack": manifest.pack.to_string(),
        "profile": manifest.profile.to_string(),
        "schema": manifest.schema_family.as_str(),
        "source_bytes": manifest.source_bytes,
    }))
}

fn json_line(value: &Value) -> Result<Vec<u8>, CliError> {
    let mut envelope = value
        .as_object()
        .cloned()
        .ok_or_else(|| CliError::internal("JSON output must be an object"))?;
    if envelope
        .insert(
            "format".to_owned(),
            Value::String(CLI_JSON_FORMAT.to_owned()),
        )
        .is_some()
    {
        return Err(CliError::internal(
            "JSON output attempted to replace the format discriminator",
        ));
    }
    let mut bytes = serde_json::to_vec(&Value::Object(envelope))
        .map_err(|_| CliError::internal("failed to encode JSON output"))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn drift_human(drift: DriftKind) -> String {
    match drift {
        DriftKind::PackMetadata => "pack metadata".to_owned(),
        DriftKind::ProfileSelection => "profile selection".to_owned(),
        DriftKind::OverlaySet => "overlay set".to_owned(),
        DriftKind::OverlayContent { index } => format!("overlay content at index {index}"),
        DriftKind::RenderedOutput => "rendered output".to_owned(),
    }
}

fn drift_json(drift: DriftKind) -> Value {
    match drift {
        DriftKind::PackMetadata => json!({ "kind": "pack_metadata" }),
        DriftKind::ProfileSelection => json!({ "kind": "profile_selection" }),
        DriftKind::OverlaySet => json!({ "kind": "overlay_set" }),
        DriftKind::OverlayContent { index } => {
            json!({ "index": index, "kind": "overlay_content" })
        }
        DriftKind::RenderedOutput => json!({ "kind": "rendered_output" }),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Difference {
    PackMetadata,
    ProfileSelection,
    OverlaySet,
    OverlayContent { index: usize },
    RenderedOutput,
}

impl Difference {
    fn human(self) -> String {
        match self {
            Self::PackMetadata => "pack metadata".to_owned(),
            Self::ProfileSelection => "profile selection".to_owned(),
            Self::OverlaySet => "overlay set".to_owned(),
            Self::OverlayContent { index } => format!("overlay content at index {index}"),
            Self::RenderedOutput => "rendered output".to_owned(),
        }
    }

    fn json(&self) -> Value {
        match *self {
            Self::PackMetadata => json!({ "kind": "pack_metadata" }),
            Self::ProfileSelection => json!({ "kind": "profile_selection" }),
            Self::OverlaySet => json!({ "kind": "overlay_set" }),
            Self::OverlayContent { index } => {
                json!({ "index": index, "kind": "overlay_content" })
            }
            Self::RenderedOutput => json!({ "kind": "rendered_output" }),
        }
    }
}

fn compare_locks(baseline: &Lockfile, candidate: &Lockfile) -> Vec<Difference> {
    let mut differences = Vec::new();
    if baseline.manifest.pack != candidate.manifest.pack {
        differences.push(Difference::PackMetadata);
    }
    if baseline.manifest.profile != candidate.manifest.profile {
        differences.push(Difference::ProfileSelection);
    }
    if same_overlay_set(baseline, candidate) {
        for (index, (left, right)) in baseline
            .manifest
            .overlays
            .iter()
            .zip(&candidate.manifest.overlays)
            .enumerate()
        {
            if left.byte_length != right.byte_length || left.digest != right.digest {
                differences.push(Difference::OverlayContent { index });
            }
        }
    } else {
        differences.push(Difference::OverlaySet);
    }
    if baseline.manifest.output != candidate.manifest.output {
        differences.push(Difference::RenderedOutput);
    }
    differences
}

fn same_overlay_set(baseline: &Lockfile, candidate: &Lockfile) -> bool {
    baseline.manifest.overlays.len() == candidate.manifest.overlays.len()
        && baseline
            .manifest
            .overlays
            .iter()
            .zip(&candidate.manifest.overlays)
            .all(|(left, right)| {
                left.class == right.class && left.id == right.id && left.source == right.source
            })
}
