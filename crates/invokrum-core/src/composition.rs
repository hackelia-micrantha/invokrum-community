use std::fmt;

use crate::{Identifier, Overlay, OverlayPack, PackRelativePath, Profile};

/// Explicit resource limits applied during composition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompositionLimits {
    maximum_overlays: usize,
    maximum_overlay_bytes: usize,
    maximum_output_bytes: usize,
}

impl CompositionLimits {
    /// Creates composition limits.
    #[must_use]
    pub const fn new(
        maximum_overlays: usize,
        maximum_overlay_bytes: usize,
        maximum_output_bytes: usize,
    ) -> Self {
        Self {
            maximum_overlays,
            maximum_overlay_bytes,
            maximum_output_bytes,
        }
    }

    /// Returns the maximum number of selected overlays.
    #[must_use]
    pub const fn maximum_overlays(self) -> usize {
        self.maximum_overlays
    }

    /// Returns the maximum bytes accepted from one overlay source.
    #[must_use]
    pub const fn maximum_overlay_bytes(self) -> usize {
        self.maximum_overlay_bytes
    }

    /// Returns the maximum normalized output size.
    #[must_use]
    pub const fn maximum_output_bytes(self) -> usize {
        self.maximum_output_bytes
    }
}

impl Default for CompositionLimits {
    fn default() -> Self {
        Self::new(256, 1_048_576, 8_388_608)
    }
}

/// Port used by the composition use case to obtain stable source bytes.
///
/// Implementations must return bytes from one stable read, perform no network
/// access, and fail closed when the requested path cannot be proven acceptable.
pub trait OverlaySource {
    /// Loads one validated pack-relative source.
    ///
    /// # Errors
    ///
    /// Returns a stable [`SourceFailure`] category when the source cannot be
    /// loaded under the adapter's containment and file-type policy.
    fn load(
        &self,
        source: &PackRelativePath,
        maximum_bytes: usize,
    ) -> Result<Vec<u8>, SourceFailure>;
}

/// Stable category for an outward source-adapter rejection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceFailureKind {
    NotFound,
    NotRegularFile,
    SymbolicLink,
    HardLink,
    DeviceBoundary,
    RootEscape,
    ChangedDuringRead,
    TooLarge,
    PermissionDenied,
    UnsupportedPlatform,
    Io,
}

impl fmt::Display for SourceFailureKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NotFound => "not found",
            Self::NotRegularFile => "not a regular file",
            Self::SymbolicLink => "symbolic link rejected",
            Self::HardLink => "hard link rejected",
            Self::DeviceBoundary => "filesystem device boundary rejected",
            Self::RootEscape => "pack-root escape rejected",
            Self::ChangedDuringRead => "source changed during read",
            Self::TooLarge => "source exceeds configured size limit",
            Self::PermissionDenied => "permission denied",
            Self::UnsupportedPlatform => "unsupported platform",
            Self::Io => "I/O failure",
        })
    }
}

/// A source-adapter failure tied to a validated pack-relative path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceFailure {
    pub path: PackRelativePath,
    pub kind: SourceFailureKind,
}

impl SourceFailure {
    /// Creates a stable source failure without embedding host error text.
    #[must_use]
    pub const fn new(path: PackRelativePath, kind: SourceFailureKind) -> Self {
        Self { path, kind }
    }
}

/// One normalized manifest entry in composition order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedEntry {
    pub class: Identifier,
    pub overlay: Identifier,
    pub source: PackRelativePath,
    pub byte_length: usize,
}

/// Deterministic structural description of a resolved profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedManifest {
    pub pack: Identifier,
    pub schema_family: String,
    pub profile: Identifier,
    pub entries: Vec<ResolvedEntry>,
    pub source_bytes: usize,
    pub output_bytes: usize,
}

/// Exact source bytes paired with their validated structural authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedSegment {
    pub class: Identifier,
    pub overlay: Identifier,
    pub source: PackRelativePath,
    pub bytes: Vec<u8>,
}

/// A deterministic composition result.
///
/// Structural ordering is represented independently from overlay prose, so a
/// later segment cannot redefine the class authority or ordering established by
/// the validated pack and profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Composition {
    manifest: ResolvedManifest,
    segments: Vec<ResolvedSegment>,
    normalized_context: Vec<u8>,
}

impl Composition {
    /// Returns the normalized manifest.
    #[must_use]
    pub const fn manifest(&self) -> &ResolvedManifest {
        &self.manifest
    }

    /// Returns exact source segments in deterministic composition order.
    #[must_use]
    pub fn segments(&self) -> &[ResolvedSegment] {
        &self.segments
    }

    /// Returns normalized context bytes.
    ///
    /// Adjacent source byte sequences are separated by exactly two line-feed
    /// bytes. Exact original bytes remain available through [`Self::segments`].
    #[must_use]
    pub fn normalized_context(&self) -> &[u8] {
        &self.normalized_context
    }
}

/// A deterministic composition failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompositionError {
    UnknownProfile(Identifier),
    MissingOverlay(Identifier),
    TooManyOverlays {
        count: usize,
        maximum: usize,
    },
    IncompatibleOverlays {
        overlay: Identifier,
        other: Identifier,
    },
    OverlayTooLarge {
        overlay: Identifier,
        size: usize,
        maximum: usize,
    },
    OutputTooLarge {
        size: usize,
        maximum: usize,
    },
    Source(SourceFailure),
}

impl fmt::Display for CompositionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownProfile(profile) => write!(formatter, "unknown profile `{profile}`"),
            Self::MissingOverlay(overlay) => {
                write!(
                    formatter,
                    "validated profile references missing overlay `{overlay}`"
                )
            }
            Self::TooManyOverlays { count, maximum } => {
                write!(formatter, "selected {count} overlays; maximum is {maximum}")
            }
            Self::IncompatibleOverlays { overlay, other } => {
                write!(
                    formatter,
                    "overlay `{overlay}` is incompatible with `{other}`"
                )
            }
            Self::OverlayTooLarge {
                overlay,
                size,
                maximum,
            } => write!(
                formatter,
                "overlay `{overlay}` contains {size} bytes; maximum is {maximum}"
            ),
            Self::OutputTooLarge { size, maximum } => {
                write!(
                    formatter,
                    "normalized output requires {size} bytes; maximum is {maximum}"
                )
            }
            Self::Source(failure) => {
                write!(formatter, "overlay source was rejected: {}", failure.kind)
            }
        }
    }
}

impl std::error::Error for CompositionError {}

/// Resolves one profile and loads its overlays in deterministic class order.
///
/// Overlay order within a class is the explicit order declared by the profile.
/// Compatibility checks and source reads use that same stable order. The use
/// case performs no network, filesystem, process, environment, clock, or random
/// access; all source bytes arrive through [`OverlaySource`].
///
/// # Errors
///
/// Returns [`CompositionError`] when the profile is unknown, selected overlays
/// conflict, a configured limit is exceeded, the validated aggregate is
/// inconsistent, or the source adapter rejects an input.
pub fn compose(
    pack: &OverlayPack,
    profile_id: &Identifier,
    source: &impl OverlaySource,
    limits: CompositionLimits,
) -> Result<Composition, CompositionError> {
    let profile = pack
        .profiles()
        .iter()
        .find(|profile| &profile.id == profile_id)
        .ok_or_else(|| CompositionError::UnknownProfile(profile_id.clone()))?;
    let selected = resolve_selected(pack, profile)?;

    if selected.len() > limits.maximum_overlays {
        return Err(CompositionError::TooManyOverlays {
            count: selected.len(),
            maximum: limits.maximum_overlays,
        });
    }

    reject_incompatibilities(&selected)?;
    assemble(pack, profile, selected, source, limits)
}

fn resolve_selected<'a>(
    pack: &'a OverlayPack,
    profile: &Profile,
) -> Result<Vec<&'a Overlay>, CompositionError> {
    let mut selected = Vec::new();
    for class in pack.classes() {
        if let Some(overlay_ids) = profile.selections.get(&class.id) {
            for overlay_id in overlay_ids {
                let overlay = pack
                    .overlays()
                    .binary_search_by(|candidate| candidate.id.cmp(overlay_id))
                    .ok()
                    .map(|index| &pack.overlays()[index])
                    .ok_or_else(|| CompositionError::MissingOverlay(overlay_id.clone()))?;
                selected.push(overlay);
            }
        }
    }
    Ok(selected)
}

fn reject_incompatibilities(selected: &[&Overlay]) -> Result<(), CompositionError> {
    for overlay in selected {
        for other in selected {
            if overlay.incompatible_with.contains(&other.id) {
                return Err(CompositionError::IncompatibleOverlays {
                    overlay: overlay.id.clone(),
                    other: other.id.clone(),
                });
            }
        }
    }
    Ok(())
}

fn load_segment(
    overlay: &Overlay,
    source: &impl OverlaySource,
    maximum_bytes: usize,
) -> Result<ResolvedSegment, CompositionError> {
    let bytes = source
        .load(&overlay.source, maximum_bytes)
        .map_err(CompositionError::Source)?;
    if bytes.len() > maximum_bytes {
        return Err(CompositionError::OverlayTooLarge {
            overlay: overlay.id.clone(),
            size: bytes.len(),
            maximum: maximum_bytes,
        });
    }
    Ok(ResolvedSegment {
        class: overlay.class.clone(),
        overlay: overlay.id.clone(),
        source: overlay.source.clone(),
        bytes,
    })
}

fn assemble(
    pack: &OverlayPack,
    profile: &Profile,
    selected: Vec<&Overlay>,
    source: &impl OverlaySource,
    limits: CompositionLimits,
) -> Result<Composition, CompositionError> {
    let mut entries = Vec::with_capacity(selected.len());
    let mut segments = Vec::with_capacity(selected.len());
    let mut normalized_context = Vec::new();
    let mut source_bytes = 0_usize;

    for overlay in selected {
        let segment = load_segment(overlay, source, limits.maximum_overlay_bytes)?;
        source_bytes = checked_output_size(
            source_bytes,
            segment.bytes.len(),
            limits.maximum_output_bytes,
        )?;
        let separator_bytes = usize::from(!entries.is_empty()) * 2;
        let framed_bytes = checked_output_size(
            separator_bytes,
            segment.bytes.len(),
            limits.maximum_output_bytes,
        )?;
        let required = checked_output_size(
            normalized_context.len(),
            framed_bytes,
            limits.maximum_output_bytes,
        )?;

        if separator_bytes != 0 {
            normalized_context.extend_from_slice(b"\n\n");
        }
        normalized_context.extend_from_slice(&segment.bytes);
        entries.push(ResolvedEntry {
            class: segment.class.clone(),
            overlay: segment.overlay.clone(),
            source: segment.source.clone(),
            byte_length: segment.bytes.len(),
        });
        segments.push(segment);

        debug_assert_eq!(normalized_context.len(), required);
    }

    Ok(Composition {
        manifest: ResolvedManifest {
            pack: pack.id.clone(),
            schema_family: pack.schema_family.clone(),
            profile: profile.id.clone(),
            entries,
            source_bytes,
            output_bytes: normalized_context.len(),
        },
        segments,
        normalized_context,
    })
}

fn checked_output_size(
    current: usize,
    additional: usize,
    maximum: usize,
) -> Result<usize, CompositionError> {
    let size = current
        .checked_add(additional)
        .ok_or(CompositionError::OutputTooLarge {
            size: usize::MAX,
            maximum,
        })?;
    if size > maximum {
        return Err(CompositionError::OutputTooLarge { size, maximum });
    }
    Ok(size)
}
