//! Generic, deterministic prompt-overlay composition primitives.
//!
//! This crate owns mechanism: parsing-neutral domain types, validation,
//! deterministic resolution, rendering inputs, and attestable manifests.
//! It must not encode Anthesis-specific policy, approval, or runtime behavior.

#![forbid(unsafe_code)]

pub mod composition;
pub mod model;

pub use composition::{
    Composition, CompositionError, CompositionLimits, OverlaySource, ResolvedEntry,
    ResolvedManifest, ResolvedSegment, SourceFailure, SourceFailureKind, compose,
};
pub use model::{
    Cardinality, DomainError, Identifier, Overlay, OverlayClass, OverlayPack, PackRelativePath,
    Profile, Sensitivity, Variable,
};

#[cfg(test)]
mod tests {
    use super::{Cardinality, DomainError, Identifier, PackRelativePath};

    #[test]
    fn identifiers_reject_empty_and_structural_characters() {
        assert!(matches!(
            Identifier::parse(""),
            Err(DomainError::InvalidIdentifier(_))
        ));
        assert!(matches!(
            Identifier::parse("mode/read-only"),
            Err(DomainError::InvalidIdentifier(_))
        ));
        assert_eq!(
            Identifier::parse("read-only")
                .expect("identifier should be valid")
                .as_str(),
            "read-only"
        );
    }

    #[test]
    fn pack_paths_use_a_portable_forward_slash_grammar() {
        for invalid in [
            "/etc/passwd",
            "C:/secret.txt",
            "overlays\\core.md",
            "overlays/../secret.md",
            "./core.md",
            "overlays//core.md",
            "overlays/",
        ] {
            assert!(
                PackRelativePath::parse(invalid).is_err(),
                "path should be rejected: {invalid}"
            );
        }
        assert_eq!(
            PackRelativePath::parse("overlays/core.md")
                .expect("path should be valid")
                .as_str(),
            "overlays/core.md"
        );
    }

    #[test]
    fn cardinality_rejects_inverted_bounds() {
        assert!(matches!(
            Cardinality::new(2, Some(1)),
            Err(DomainError::InvalidCardinality { .. })
        ));
        let exactly_one = Cardinality::new(1, Some(1)).expect("cardinality should be valid");
        assert!(exactly_one.accepts(1));
        assert!(!exactly_one.accepts(0));
        assert!(!exactly_one.accepts(2));
    }
}
