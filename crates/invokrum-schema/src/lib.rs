//! Strict serialization adapters for Invokrum overlay packs.
//!
//! This crate is an infrastructure boundary. It translates YAML and JSON
//! documents into the parsing-neutral domain model owned by `invokrum-core`.

#![forbid(unsafe_code)]

mod limits;
mod strict;

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use invokrum_core::{
    Cardinality, DomainError, Identifier, Overlay, OverlayClass, OverlayPack, PackRelativePath,
    Profile, Sensitivity, Variable,
};
use serde::{Deserialize, Serialize};
use strict::{PreflightError, preflight_json, preflight_yaml};

pub use limits::{
    DEFAULT_MAX_CLASSES, DEFAULT_MAX_DOCUMENT_BYTES, DEFAULT_MAX_INCOMPATIBILITIES,
    DEFAULT_MAX_NESTING_DEPTH, DEFAULT_MAX_OVERLAYS, DEFAULT_MAX_PROFILES, DEFAULT_MAX_SELECTIONS,
    DEFAULT_MAX_VARIABLES, DeclarationKind, DeclarationLimits, SchemaLimits,
};
pub use strict::YamlFeature;

const MAX_ERROR_MESSAGE_CHARS: usize = 512;
const MAX_SCHEMA_NAME_CHARS: usize = 128;

/// Schema family implemented by this adapter.
pub const SCHEMA_FAMILY: &str = "invokrum.dev/v1";

#[derive(Clone, Debug, Deserialize)]
struct SchemaEnvelope {
    schema: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PackDocument {
    schema: String,
    id: String,
    classes: Vec<ClassDocument>,
    #[serde(default)]
    overlays: Vec<OverlayDocument>,
    #[serde(default)]
    profiles: Vec<ProfileDocument>,
    #[serde(default)]
    variables: Vec<VariableDocument>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ClassDocument {
    id: String,
    order: u32,
    minimum: u32,
    #[serde(default)]
    maximum: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct OverlayDocument {
    id: String,
    class: String,
    source: String,
    #[serde(default)]
    incompatible_with: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ProfileDocument {
    id: String,
    selections: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct VariableDocument {
    name: String,
    sensitivity: SensitivityDocument,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum SensitivityDocument {
    Public,
    Secret,
}

/// Parse a strict v1 JSON pack document with the default resource limits.
///
/// # Errors
///
/// Returns [`SchemaError`] when a resource limit is exceeded, JSON decoding
/// fails, mapping keys are repeated, unknown fields are present, the schema
/// family is unsupported, duplicate list values are present, or domain
/// invariants are violated.
pub fn parse_json(input: &str) -> Result<OverlayPack, SchemaError> {
    parse_json_with_limits(input, SchemaLimits::default())
}

/// Parse a strict v1 JSON pack document with explicit immutable limits.
///
/// The byte limit is checked before deserialization. Recursive structural
/// preflight rejects duplicate object keys and excessive container depth before
/// schema-family negotiation. Declaration limits are checked after DTO decoding
/// and before domain aggregate construction.
///
/// # Errors
///
/// Returns [`SchemaError`] for resource-limit, decoding, schema, list, or domain
/// failures.
pub fn parse_json_with_limits(
    input: &str,
    limits: SchemaLimits,
) -> Result<OverlayPack, SchemaError> {
    ensure_document_size(input, limits)?;
    preflight_json(input, limits.nesting_depth())?;

    let envelope: SchemaEnvelope = serde_json::from_str(input)
        .map_err(|error| SchemaError::decode("json", &error.to_string()))?;
    ensure_supported_schema(&envelope.schema)?;

    let document: PackDocument = serde_json::from_str(input)
        .map_err(|error| SchemaError::decode("json", &error.to_string()))?;
    document.ensure_declaration_limits(limits.declarations())?;
    document.try_into()
}

/// Parse a strict v1 YAML pack document with the default resource limits.
///
/// # Errors
///
/// Returns [`SchemaError`] when a resource limit is exceeded, YAML decoding
/// fails, mapping keys are repeated, unsupported YAML features or multiple
/// documents are present, unknown fields are present, the schema family is
/// unsupported, duplicate list values are present, or domain invariants are
/// violated.
pub fn parse_yaml(input: &str) -> Result<OverlayPack, SchemaError> {
    parse_yaml_with_limits(input, SchemaLimits::default())
}

/// Parse a strict v1 YAML pack document with explicit immutable limits.
///
/// The byte limit is checked before scanning or deserialization. Recursive
/// structural preflight rejects duplicate mapping keys, excessive container
/// depth, multiple documents, and YAML features outside the v1 subset before
/// schema-family negotiation. Declaration limits are checked before domain
/// aggregate construction.
///
/// # Errors
///
/// Returns [`SchemaError`] for resource-limit, decoding, YAML-subset, schema,
/// list, or domain failures.
pub fn parse_yaml_with_limits(
    input: &str,
    limits: SchemaLimits,
) -> Result<OverlayPack, SchemaError> {
    ensure_document_size(input, limits)?;
    preflight_yaml(input, limits.nesting_depth())?;

    let envelope: SchemaEnvelope = serde_yaml_ng::from_str(input)
        .map_err(|error| SchemaError::decode("yaml", &error.to_string()))?;
    ensure_supported_schema(&envelope.schema)?;

    let document: PackDocument = serde_yaml_ng::from_str(input)
        .map_err(|error| SchemaError::decode("yaml", &error.to_string()))?;
    document.ensure_declaration_limits(limits.declarations())?;
    document.try_into()
}

/// Serialize a validated pack into stable, pretty-printed JSON.
///
/// The domain aggregate normalizes classes by explicit order and other named
/// collections by identifier. Ordered map and set types stabilize nested data.
///
/// # Errors
///
/// Returns [`SchemaError`] if serialization unexpectedly fails.
pub fn to_normalized_json(pack: &OverlayPack) -> Result<String, SchemaError> {
    serde_json::to_string_pretty(&PackDocument::from(pack)).map_err(|error| {
        SchemaError::Encode(bounded_text(&error.to_string(), MAX_ERROR_MESSAGE_CHARS))
    })
}

fn ensure_document_size(input: &str, limits: SchemaLimits) -> Result<(), SchemaError> {
    let actual_bytes = input.len();
    let maximum_bytes = limits.document_bytes();
    if actual_bytes > maximum_bytes {
        Err(SchemaError::DocumentTooLarge {
            maximum_bytes,
            actual_bytes,
        })
    } else {
        Ok(())
    }
}

fn ensure_supported_schema(schema: &str) -> Result<(), SchemaError> {
    if schema == SCHEMA_FAMILY {
        Ok(())
    } else {
        Err(SchemaError::UnsupportedSchema(bounded_text(
            schema,
            MAX_SCHEMA_NAME_CHARS,
        )))
    }
}

fn bounded_text(value: &str, maximum_chars: usize) -> String {
    let mut bounded: String = value.chars().take(maximum_chars).collect();
    if value.chars().count() > maximum_chars {
        bounded.push('…');
    }
    bounded
}

fn parse_identifier_set(
    values: Vec<String>,
    field: &'static str,
) -> Result<BTreeSet<Identifier>, SchemaError> {
    let mut identifiers = BTreeSet::new();
    for value in values {
        let identifier = Identifier::parse(value)?;
        let duplicate = identifier.clone();
        if !identifiers.insert(identifier) {
            return Err(SchemaError::DuplicateListValue {
                field,
                value: duplicate,
            });
        }
    }
    Ok(identifiers)
}

fn ensure_count(
    kind: DeclarationKind,
    actual: usize,
    limits: DeclarationLimits,
) -> Result<(), SchemaError> {
    let maximum = limits.maximum(kind);
    if actual > maximum {
        Err(SchemaError::TooManyDeclarations {
            kind,
            maximum,
            actual,
        })
    } else {
        Ok(())
    }
}

fn ensure_accumulated_count(
    kind: DeclarationKind,
    counts: impl IntoIterator<Item = usize>,
    limits: DeclarationLimits,
) -> Result<(), SchemaError> {
    let maximum = limits.maximum(kind);
    let mut actual = 0usize;
    for count in counts {
        actual = actual
            .checked_add(count)
            .ok_or(SchemaError::TooManyDeclarations {
                kind,
                maximum,
                actual: usize::MAX,
            })?;
        if actual > maximum {
            return Err(SchemaError::TooManyDeclarations {
                kind,
                maximum,
                actual,
            });
        }
    }
    Ok(())
}

impl PackDocument {
    fn ensure_declaration_limits(&self, limits: DeclarationLimits) -> Result<(), SchemaError> {
        ensure_count(DeclarationKind::Class, self.classes.len(), limits)?;
        ensure_count(DeclarationKind::Overlay, self.overlays.len(), limits)?;
        ensure_count(DeclarationKind::Profile, self.profiles.len(), limits)?;
        ensure_count(DeclarationKind::Variable, self.variables.len(), limits)?;

        ensure_accumulated_count(
            DeclarationKind::Selection,
            self.profiles.iter().flat_map(|profile| {
                profile
                    .selections
                    .values()
                    .flat_map(|overlays| [1usize, overlays.len()])
            }),
            limits,
        )?;
        ensure_accumulated_count(
            DeclarationKind::Incompatibility,
            self.overlays
                .iter()
                .map(|overlay| overlay.incompatible_with.len()),
            limits,
        )
    }
}

impl TryFrom<PackDocument> for OverlayPack {
    type Error = SchemaError;

    fn try_from(document: PackDocument) -> Result<Self, Self::Error> {
        ensure_supported_schema(&document.schema)?;

        let classes = document
            .classes
            .into_iter()
            .map(|class| {
                Ok(OverlayClass {
                    id: Identifier::parse(class.id)?,
                    order: class.order,
                    cardinality: Cardinality::new(class.minimum, class.maximum)?,
                })
            })
            .collect::<Result<Vec<_>, DomainError>>()?;

        let overlays = document
            .overlays
            .into_iter()
            .map(|overlay| {
                Ok(Overlay {
                    id: Identifier::parse(overlay.id)?,
                    class: Identifier::parse(overlay.class)?,
                    source: PackRelativePath::parse(overlay.source)?,
                    incompatible_with: parse_identifier_set(
                        overlay.incompatible_with,
                        "overlays[].incompatible_with",
                    )?,
                })
            })
            .collect::<Result<Vec<_>, SchemaError>>()?;

        let profiles = document
            .profiles
            .into_iter()
            .map(|profile| {
                let selections = profile
                    .selections
                    .into_iter()
                    .map(|(class, overlays)| {
                        Ok((
                            Identifier::parse(class)?,
                            overlays
                                .into_iter()
                                .map(Identifier::parse)
                                .collect::<Result<Vec<_>, _>>()?,
                        ))
                    })
                    .collect::<Result<BTreeMap<_, _>, DomainError>>()?;
                Ok(Profile {
                    id: Identifier::parse(profile.id)?,
                    selections,
                })
            })
            .collect::<Result<Vec<_>, DomainError>>()?;

        let variables = document
            .variables
            .into_iter()
            .map(|variable| {
                Ok(Variable {
                    name: Identifier::parse(variable.name)?,
                    sensitivity: match variable.sensitivity {
                        SensitivityDocument::Public => Sensitivity::Public,
                        SensitivityDocument::Secret => Sensitivity::Secret,
                    },
                })
            })
            .collect::<Result<Vec<_>, DomainError>>()?;

        OverlayPack::new(
            Identifier::parse(document.id)?,
            document.schema,
            classes,
            overlays,
            profiles,
            variables,
        )
        .map_err(Into::into)
    }
}

impl From<&OverlayPack> for PackDocument {
    fn from(pack: &OverlayPack) -> Self {
        Self {
            schema: pack.schema_family.clone(),
            id: pack.id.to_string(),
            classes: pack
                .classes()
                .iter()
                .map(|class| ClassDocument {
                    id: class.id.to_string(),
                    order: class.order,
                    minimum: class.cardinality.minimum(),
                    maximum: class.cardinality.maximum(),
                })
                .collect(),
            overlays: pack
                .overlays()
                .iter()
                .map(|overlay| OverlayDocument {
                    id: overlay.id.to_string(),
                    class: overlay.class.to_string(),
                    source: overlay.source.as_str().to_owned(),
                    incompatible_with: overlay
                        .incompatible_with
                        .iter()
                        .map(ToString::to_string)
                        .collect(),
                })
                .collect(),
            profiles: pack
                .profiles()
                .iter()
                .map(|profile| ProfileDocument {
                    id: profile.id.to_string(),
                    selections: profile
                        .selections
                        .iter()
                        .map(|(class, overlays)| {
                            (
                                class.to_string(),
                                overlays.iter().map(ToString::to_string).collect(),
                            )
                        })
                        .collect(),
                })
                .collect(),
            variables: pack
                .variables()
                .iter()
                .map(|variable| VariableDocument {
                    name: variable.name.to_string(),
                    sensitivity: match variable.sensitivity {
                        Sensitivity::Public => SensitivityDocument::Public,
                        Sensitivity::Secret => SensitivityDocument::Secret,
                    },
                })
                .collect(),
        }
    }
}

/// A schema-boundary failure while decoding, validating, or encoding a pack.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SchemaError {
    DocumentTooLarge {
        maximum_bytes: usize,
        actual_bytes: usize,
    },
    NestingTooDeep {
        maximum_depth: usize,
    },
    TooManyDeclarations {
        kind: DeclarationKind,
        maximum: usize,
        actual: usize,
    },
    Decode {
        format: &'static str,
        message: String,
    },
    DuplicateMappingKey {
        format: &'static str,
    },
    UnsupportedYamlFeature(YamlFeature),
    MultipleYamlDocuments,
    Encode(String),
    UnsupportedSchema(String),
    DuplicateListValue {
        field: &'static str,
        value: Identifier,
    },
    Domain(DomainError),
}

impl SchemaError {
    fn decode(format: &'static str, message: &str) -> Self {
        Self::Decode {
            format,
            message: bounded_text(message, MAX_ERROR_MESSAGE_CHARS),
        }
    }
}

impl From<PreflightError> for SchemaError {
    fn from(error: PreflightError) -> Self {
        match error {
            PreflightError::Decode { format, message } => Self::Decode { format, message },
            PreflightError::DuplicateMappingKey { format } => Self::DuplicateMappingKey { format },
            PreflightError::UnsupportedYamlFeature(feature) => {
                Self::UnsupportedYamlFeature(feature)
            }
            PreflightError::MultipleYamlDocuments => Self::MultipleYamlDocuments,
            PreflightError::NestingTooDeep { maximum_depth } => {
                Self::NestingTooDeep { maximum_depth }
            }
        }
    }
}

impl From<DomainError> for SchemaError {
    fn from(error: DomainError) -> Self {
        Self::Domain(error)
    }
}

impl fmt::Display for SchemaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DocumentTooLarge {
                maximum_bytes,
                actual_bytes,
            } => write!(
                formatter,
                "schema document is {actual_bytes} bytes; maximum is {maximum_bytes} bytes"
            ),
            Self::NestingTooDeep { maximum_depth } => write!(
                formatter,
                "schema document exceeds maximum container depth {maximum_depth}"
            ),
            Self::TooManyDeclarations {
                kind,
                maximum,
                actual,
            } => write!(
                formatter,
                "schema document has {actual} {kind} declarations; maximum is {maximum}"
            ),
            Self::Decode { format, message } => write!(formatter, "invalid {format}: {message}"),
            Self::DuplicateMappingKey { format } => {
                write!(formatter, "duplicate mapping key in {format} input")
            }
            Self::UnsupportedYamlFeature(feature) => {
                write!(formatter, "unsupported YAML feature: {feature}")
            }
            Self::MultipleYamlDocuments => {
                formatter.write_str("multiple YAML documents are not supported")
            }
            Self::Encode(message) => {
                write!(formatter, "failed to encode normalized JSON: {message}")
            }
            Self::UnsupportedSchema(schema) => {
                write!(formatter, "unsupported schema family: {schema}")
            }
            Self::DuplicateListValue { field, value } => {
                write!(formatter, "duplicate value `{value}` in {field}")
            }
            Self::Domain(error) => write!(formatter, "invalid pack: {error}"),
        }
    }
}

impl Error for SchemaError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Domain(error) => Some(error),
            _ => None,
        }
    }
}
