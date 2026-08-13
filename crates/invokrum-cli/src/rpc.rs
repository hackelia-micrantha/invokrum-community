use std::io::Read;
use std::path::{Path, PathBuf};
use std::str;

use invokrum_core::{CompositionLimits, Identifier, OverlayPack, OverlaySource, PackRelativePath};
use invokrum_fs::LocalPackSource;
use invokrum_host::{
    HOST_CONTRACT_VERSION, HostError, ResolvedBundle, resolve_bundle, verify_bundle,
};
use invokrum_integrity::{DriftKind, MAX_LOCKFILE_BYTES, decode_lockfile};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{EXIT_INPUT, EXIT_INTERNAL, EXIT_SUCCESS, EXIT_VALIDATION, Execution};

const MAX_REQUEST_BYTES: usize = 1_048_576;
const MAX_PACK_BYTES: usize = 1_048_576;
const MAX_JSON_DEPTH: usize = 32;
const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const INTERNAL_RESPONSE: &[u8] = br#"{"error":{"code":"internal","message":"failed to encode response"},"format":"invokrum.host/v1","ok":false,"request_id":null}"#;

#[derive(Debug, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
enum Request {
    Capabilities {
        protocol: String,
        request_id: String,
    },
    Resolve {
        protocol: String,
        request_id: String,
        pack: PathBuf,
        profile: String,
    },
    Verify {
        protocol: String,
        request_id: String,
        pack: PathBuf,
        profile: String,
        expected_lock_base64: String,
    },
}

impl Request {
    fn protocol(&self) -> &str {
        match self {
            Self::Capabilities { protocol, .. }
            | Self::Resolve { protocol, .. }
            | Self::Verify { protocol, .. } => protocol,
        }
    }

    fn request_id(&self) -> &str {
        match self {
            Self::Capabilities { request_id, .. }
            | Self::Resolve { request_id, .. }
            | Self::Verify { request_id, .. } => request_id,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum RpcErrorKind {
    Request,
    Input,
    Validation,
    Composition,
    Integrity,
}

impl RpcErrorKind {
    const fn code(self) -> &'static str {
        match self {
            Self::Request => "request",
            Self::Input => "input",
            Self::Validation => "validation",
            Self::Composition => "composition",
            Self::Integrity => "integrity",
        }
    }

    const fn exit_code(self) -> i32 {
        match self {
            Self::Request | Self::Input => EXIT_INPUT,
            Self::Validation | Self::Composition => EXIT_VALIDATION,
            Self::Integrity => EXIT_INTERNAL,
        }
    }
}

#[derive(Debug)]
struct RpcError {
    kind: RpcErrorKind,
    message: String,
}

impl RpcError {
    fn new(kind: RpcErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

pub(crate) fn execute(stdin: &mut dyn Read) -> Execution {
    let request_bytes = match read_bounded(stdin, MAX_REQUEST_BYTES) {
        Ok(bytes) => bytes,
        Err(error) => return error_execution(None, &error),
    };
    if let Err(message) = validate_json_depth(&request_bytes, MAX_JSON_DEPTH) {
        return error_execution(None, &RpcError::new(RpcErrorKind::Request, message));
    }
    let request = match serde_json::from_slice::<Request>(&request_bytes) {
        Ok(request) => request,
        Err(error) => {
            return error_execution(
                None,
                &RpcError::new(RpcErrorKind::Request, bounded_parser_message(&error)),
            );
        }
    };
    let request_id = request.request_id().to_owned();
    if request.protocol() != HOST_CONTRACT_VERSION {
        return error_execution(
            Some(&request_id),
            &RpcError::new(
                RpcErrorKind::Request,
                format!(
                    "unsupported protocol `{}`; expected `{HOST_CONTRACT_VERSION}`",
                    request.protocol()
                ),
            ),
        );
    }
    if request_id.is_empty() || request_id.len() > 128 {
        return error_execution(
            Some(&request_id),
            &RpcError::new(
                RpcErrorKind::Request,
                "request_id must contain between 1 and 128 bytes",
            ),
        );
    }

    match execute_request(request) {
        Ok((operation, result)) => success_execution(&request_id, operation, &result),
        Err(error) => error_execution(Some(&request_id), &error),
    }
}

fn execute_request(request: Request) -> Result<(&'static str, Value), RpcError> {
    match request {
        Request::Capabilities { .. } => Ok(("capabilities", capabilities())),
        Request::Resolve { pack, profile, .. } => {
            let (pack_value, source) = load_pack(&pack)?;
            let profile = parse_profile(&profile)?;
            let bundle =
                resolve_bundle(&pack_value, &profile, &source, CompositionLimits::default())
                    .map_err(map_host_error)?;
            Ok(("resolve", bundle_json(&bundle)))
        }
        Request::Verify {
            pack,
            profile,
            expected_lock_base64,
            ..
        } => {
            let lock_bytes = decode_base64(&expected_lock_base64)
                .map_err(|message| RpcError::new(RpcErrorKind::Request, message))?;
            if lock_bytes.len() > MAX_LOCKFILE_BYTES {
                return Err(RpcError::new(
                    RpcErrorKind::Integrity,
                    "decoded expected lock exceeds the configured byte limit",
                ));
            }
            let expected = decode_lockfile(&lock_bytes)
                .map_err(|error| RpcError::new(RpcErrorKind::Integrity, error.to_string()))?;
            let (pack_value, source) = load_pack(&pack)?;
            let profile = parse_profile(&profile)?;
            let verified = verify_bundle(
                &expected,
                &pack_value,
                &profile,
                &source,
                CompositionLimits::default(),
            )
            .map_err(map_host_error)?;
            Ok((
                "verify",
                json!({
                    "bundle": bundle_json(verified.current()),
                    "drifts": verified.report().drifts().iter().map(|drift| drift_json(*drift)).collect::<Vec<_>>(),
                    "verified": verified.is_verified(),
                }),
            ))
        }
    }
}

fn capabilities() -> Value {
    let limits = CompositionLimits::default();
    json!({
        "capabilities": ["capabilities", "resolve", "verify"],
        "default_limits": {
            "maximum_json_depth": MAX_JSON_DEPTH,
            "maximum_output_bytes": limits.maximum_output_bytes(),
            "maximum_overlay_bytes": limits.maximum_overlay_bytes(),
            "maximum_overlays": limits.maximum_overlays(),
            "maximum_pack_bytes": MAX_PACK_BYTES,
            "maximum_request_bytes": MAX_REQUEST_BYTES,
            "maximum_request_id_bytes": 128,
        },
        "network_access": false,
        "persistent_writes": false,
        "runtime_invocation": false,
    })
}

fn bundle_json(bundle: &ResolvedBundle) -> Value {
    let manifest = bundle.manifest();
    json!({
        "context_base64": encode_base64(bundle.context()),
        "lock_base64": encode_base64(bundle.lock_bytes()),
        "manifest": {
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
        },
        "output_digest": bundle.lockfile().manifest.output.digest.as_str(),
    })
}

fn load_pack(path: &Path) -> Result<(OverlayPack, LocalPackSource), RpcError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| RpcError::new(RpcErrorKind::Input, "pack path must name a UTF-8 file"))?;
    let relative = PackRelativePath::parse(file_name.to_owned()).map_err(|_| {
        RpcError::new(
            RpcErrorKind::Input,
            "pack file name violates the portable path policy",
        )
    })?;
    let source = LocalPackSource::open(parent)
        .map_err(|error| RpcError::new(RpcErrorKind::Input, error.to_string()))?;
    let bytes = source.load(&relative, MAX_PACK_BYTES).map_err(|error| {
        RpcError::new(
            RpcErrorKind::Input,
            format!(
                "pack input `{}` was rejected: {}",
                error.path.as_str(),
                error.kind
            ),
        )
    })?;
    let text = str::from_utf8(&bytes)
        .map_err(|_| RpcError::new(RpcErrorKind::Input, "pack document must be valid UTF-8"))?;
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
    .map_err(|error| RpcError::new(RpcErrorKind::Validation, error.to_string()))?;
    Ok((pack, source))
}

fn parse_profile(value: &str) -> Result<Identifier, RpcError> {
    Identifier::parse(value.to_owned())
        .map_err(|_| RpcError::new(RpcErrorKind::Validation, "profile identifier is invalid"))
}

fn map_host_error(error: HostError) -> RpcError {
    match error {
        HostError::Composition(error) => {
            RpcError::new(RpcErrorKind::Composition, error.to_string())
        }
        HostError::Integrity(error) => RpcError::new(RpcErrorKind::Integrity, error.to_string()),
    }
}

fn success_execution(request_id: &str, operation: &str, result: &Value) -> Execution {
    Execution {
        code: EXIT_SUCCESS,
        stdout: encode_response(&json!({
            "format": HOST_CONTRACT_VERSION,
            "ok": true,
            "operation": operation,
            "request_id": request_id,
            "result": result,
        })),
    }
}

fn error_execution(request_id: Option<&str>, error: &RpcError) -> Execution {
    Execution {
        code: error.kind.exit_code(),
        stdout: encode_response(&json!({
            "error": {
                "code": error.kind.code(),
                "message": error.message,
            },
            "format": HOST_CONTRACT_VERSION,
            "ok": false,
            "request_id": request_id,
        })),
    }
}

fn encode_response(value: &Value) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(value).unwrap_or_else(|_| INTERNAL_RESPONSE.to_vec());
    bytes.push(b'\n');
    bytes
}

fn read_bounded(reader: &mut dyn Read, maximum_bytes: usize) -> Result<Vec<u8>, RpcError> {
    let mut bytes = Vec::new();
    reader
        .take((maximum_bytes as u64) + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| RpcError::new(RpcErrorKind::Input, "failed to read request from stdin"))?;
    if bytes.len() > maximum_bytes {
        return Err(RpcError::new(
            RpcErrorKind::Request,
            "request exceeds the configured byte limit",
        ));
    }
    if bytes.is_empty() {
        return Err(RpcError::new(RpcErrorKind::Request, "request is empty"));
    }
    Ok(bytes)
}

fn validate_json_depth(bytes: &[u8], maximum_depth: usize) -> Result<(), &'static str> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for byte in bytes {
        if in_string {
            if escaped {
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match *byte {
            b'"' => in_string = true,
            b'{' | b'[' => {
                depth = depth.saturating_add(1);
                if depth > maximum_depth {
                    return Err("request exceeds the configured JSON depth limit");
                }
            }
            b'}' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    Ok(())
}

fn bounded_parser_message(error: &serde_json::Error) -> String {
    error.to_string().chars().take(256).collect()
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

fn encode_base64(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = *chunk.get(1).unwrap_or(&0);
        let third = *chunk.get(2).unwrap_or(&0);
        encoded.push(char::from(BASE64_ALPHABET[usize::from(first >> 2)]));
        encoded.push(char::from(
            BASE64_ALPHABET[usize::from(((first & 0x03) << 4) | (second >> 4))],
        ));
        if chunk.len() > 1 {
            encoded.push(char::from(
                BASE64_ALPHABET[usize::from(((second & 0x0f) << 2) | (third >> 6))],
            ));
        } else {
            encoded.push('=');
        }
        if chunk.len() > 2 {
            encoded.push(char::from(BASE64_ALPHABET[usize::from(third & 0x3f)]));
        } else {
            encoded.push('=');
        }
    }
    encoded
}

fn decode_base64(value: &str) -> Result<Vec<u8>, &'static str> {
    let bytes = value.as_bytes();
    if bytes.len() % 4 != 0 {
        return Err("expected_lock_base64 length is not a multiple of four");
    }
    let mut decoded = Vec::with_capacity((bytes.len() / 4) * 3);
    for (index, chunk) in bytes.chunks_exact(4).enumerate() {
        let last = index + 1 == bytes.len() / 4;
        let padding = match (chunk[2], chunk[3]) {
            (b'=', b'=') => 2,
            (_, b'=') => 1,
            (b'=', _) => return Err("expected_lock_base64 has invalid padding"),
            _ => 0,
        };
        if padding > 0 && !last {
            return Err("expected_lock_base64 padding must appear only at the end");
        }
        let first = decode_base64_character(chunk[0])?;
        let second = decode_base64_character(chunk[1])?;
        let third = if padding == 2 {
            0
        } else {
            decode_base64_character(chunk[2])?
        };
        let fourth = if padding > 0 {
            0
        } else {
            decode_base64_character(chunk[3])?
        };
        if padding == 2 && second & 0x0f != 0 {
            return Err("expected_lock_base64 has noncanonical trailing bits");
        }
        if padding == 1 && third & 0x03 != 0 {
            return Err("expected_lock_base64 has noncanonical trailing bits");
        }
        decoded.push((first << 2) | (second >> 4));
        if padding < 2 {
            decoded.push((second << 4) | (third >> 2));
        }
        if padding == 0 {
            decoded.push((third << 6) | fourth);
        }
    }
    Ok(decoded)
}

fn decode_base64_character(value: u8) -> Result<u8, &'static str> {
    match value {
        b'A'..=b'Z' => Ok(value - b'A'),
        b'a'..=b'z' => Ok(value - b'a' + 26),
        b'0'..=b'9' => Ok(value - b'0' + 52),
        b'+' => Ok(62),
        b'/' => Ok(63),
        _ => Err("expected_lock_base64 contains an invalid character"),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::{INTERNAL_RESPONSE, decode_base64, encode_base64};

    #[test]
    fn base64_round_trips_boundary_lengths() {
        for value in [b"".as_slice(), b"a", b"ab", b"abc", b"abcd"] {
            let encoded = encode_base64(value);
            assert_eq!(decode_base64(&encoded), Ok(value.to_vec()));
        }
    }

    #[test]
    fn base64_rejects_noncanonical_or_misplaced_padding() {
        for value in ["A===", "AA=A", "AB==", "AAB="] {
            assert!(decode_base64(value).is_err(), "value should fail: {value}");
        }
    }

    #[test]
    fn emergency_response_is_valid_machine_json() {
        let response: Value =
            serde_json::from_slice(INTERNAL_RESPONSE).expect("fallback must be valid JSON");
        assert_eq!(response["format"], "invokrum.host/v1");
        assert_eq!(response["ok"], false);
        assert_eq!(response["error"]["code"], "internal");
    }
}
