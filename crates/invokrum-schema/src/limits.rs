use std::fmt;

/// Default maximum serialized JSON or YAML document size: 1 MiB.
pub const DEFAULT_MAX_DOCUMENT_BYTES: usize = 1_048_576;
/// Default maximum nested mapping/sequence depth.
pub const DEFAULT_MAX_NESTING_DEPTH: usize = 32;
/// Default maximum number of class declarations.
pub const DEFAULT_MAX_CLASSES: usize = 64;
/// Default maximum number of overlay declarations.
pub const DEFAULT_MAX_OVERLAYS: usize = 256;
/// Default maximum number of profile declarations.
pub const DEFAULT_MAX_PROFILES: usize = 128;
/// Default maximum number of variable declarations.
pub const DEFAULT_MAX_VARIABLES: usize = 256;
/// Default maximum number of profile selection declarations.
pub const DEFAULT_MAX_SELECTIONS: usize = 4_096;
/// Default maximum number of incompatibility declarations.
pub const DEFAULT_MAX_INCOMPATIBILITIES: usize = 4_096;

/// A declaration category governed by [`DeclarationLimits`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeclarationKind {
    Class,
    Overlay,
    Profile,
    Variable,
    Selection,
    Incompatibility,
}

impl fmt::Display for DeclarationKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Class => "class",
            Self::Overlay => "overlay",
            Self::Profile => "profile",
            Self::Variable => "variable",
            Self::Selection => "selection",
            Self::Incompatibility => "incompatibility",
        })
    }
}

/// Immutable limits for declarations decoded from one pack document.
///
/// Selection declarations count both each class key in a profile's `selections`
/// map and each selected overlay identifier. This bounds empty selection maps as
/// well as large selected-overlay lists.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeclarationLimits {
    classes: usize,
    overlays: usize,
    profiles: usize,
    variables: usize,
    selections: usize,
    incompatibilities: usize,
}

impl DeclarationLimits {
    #[must_use]
    pub const fn new(
        classes: usize,
        overlays: usize,
        profiles: usize,
        variables: usize,
        selections: usize,
        incompatibilities: usize,
    ) -> Self {
        Self {
            classes,
            overlays,
            profiles,
            variables,
            selections,
            incompatibilities,
        }
    }

    #[must_use]
    pub const fn maximum(self, kind: DeclarationKind) -> usize {
        match kind {
            DeclarationKind::Class => self.classes,
            DeclarationKind::Overlay => self.overlays,
            DeclarationKind::Profile => self.profiles,
            DeclarationKind::Variable => self.variables,
            DeclarationKind::Selection => self.selections,
            DeclarationKind::Incompatibility => self.incompatibilities,
        }
    }
}

impl Default for DeclarationLimits {
    fn default() -> Self {
        Self::new(
            DEFAULT_MAX_CLASSES,
            DEFAULT_MAX_OVERLAYS,
            DEFAULT_MAX_PROFILES,
            DEFAULT_MAX_VARIABLES,
            DEFAULT_MAX_SELECTIONS,
            DEFAULT_MAX_INCOMPATIBILITIES,
        )
    }
}

/// Immutable schema-boundary resource limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchemaLimits {
    document_bytes: usize,
    nesting_depth: usize,
    declarations: DeclarationLimits,
}

impl SchemaLimits {
    #[must_use]
    pub const fn new(
        document_bytes: usize,
        nesting_depth: usize,
        declarations: DeclarationLimits,
    ) -> Self {
        Self {
            document_bytes,
            nesting_depth,
            declarations,
        }
    }

    #[must_use]
    pub const fn document_bytes(self) -> usize {
        self.document_bytes
    }

    #[must_use]
    pub const fn nesting_depth(self) -> usize {
        self.nesting_depth
    }

    #[must_use]
    pub const fn declarations(self) -> DeclarationLimits {
        self.declarations
    }
}

impl Default for SchemaLimits {
    fn default() -> Self {
        Self::new(
            DEFAULT_MAX_DOCUMENT_BYTES,
            DEFAULT_MAX_NESTING_DEPTH,
            DeclarationLimits::default(),
        )
    }
}
