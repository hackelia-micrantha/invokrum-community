use std::collections::{BTreeMap, BTreeSet};

use invokrum_core::{
    Cardinality, CompositionError, CompositionLimits, Identifier, Overlay, OverlayClass,
    OverlayPack, OverlaySource, PackRelativePath, Profile, SourceFailure, SourceFailureKind,
    compose,
};

fn id(value: &str) -> Identifier {
    Identifier::parse(value).expect("test identifier should be valid")
}

fn path(value: &str) -> PackRelativePath {
    PackRelativePath::parse(value).expect("test path should be valid")
}

fn class(name: &str, order: u32) -> OverlayClass {
    OverlayClass {
        id: id(name),
        order,
        cardinality: Cardinality::new(0, None).expect("valid cardinality"),
    }
}

fn overlay(name: &str, class_name: &str, incompatible_with: &[&str]) -> Overlay {
    Overlay {
        id: id(name),
        class: id(class_name),
        source: path(&format!("overlays/{name}.md")),
        incompatible_with: incompatible_with.iter().map(|value| id(value)).collect(),
    }
}

fn pack(incompatible: bool) -> OverlayPack {
    let profile = Profile {
        id: id("review"),
        selections: BTreeMap::from([
            (id("core"), vec![id("core-default")]),
            (id("mode"), vec![id("read-only"), id("guarded")]),
        ]),
    };
    OverlayPack::new(
        id("example"),
        "test/v1",
        vec![class("mode", 20), class("core", 10)],
        vec![
            overlay("guarded", "mode", &[]),
            overlay(
                "core-default",
                "core",
                if incompatible { &["guarded"] } else { &[] },
            ),
            overlay("read-only", "mode", &[]),
        ],
        vec![profile],
        Vec::new(),
    )
    .expect("pack should be valid")
}

#[derive(Default)]
struct MemorySource {
    files: BTreeMap<PackRelativePath, Vec<u8>>,
}

impl MemorySource {
    fn with(mut self, source: &str, bytes: &[u8]) -> Self {
        self.files.insert(path(source), bytes.to_vec());
        self
    }
}

impl OverlaySource for MemorySource {
    fn load(
        &self,
        source: &PackRelativePath,
        maximum_bytes: usize,
    ) -> Result<Vec<u8>, SourceFailure> {
        let bytes = self
            .files
            .get(source)
            .ok_or_else(|| SourceFailure::new(source.clone(), SourceFailureKind::NotFound))?;
        if bytes.len() > maximum_bytes {
            return Err(SourceFailure::new(
                source.clone(),
                SourceFailureKind::TooLarge,
            ));
        }
        Ok(bytes.clone())
    }
}

fn source() -> MemorySource {
    MemorySource::default()
        .with("overlays/core-default.md", b"core")
        .with("overlays/read-only.md", b"read-only")
        .with("overlays/guarded.md", b"guarded")
}

#[test]
fn composition_is_byte_identical_and_structurally_ordered() {
    let pack = pack(false);
    let first = compose(
        &pack,
        &id("review"),
        &source(),
        CompositionLimits::default(),
    )
    .expect("composition should succeed");
    let second = compose(
        &pack,
        &id("review"),
        &source(),
        CompositionLimits::default(),
    )
    .expect("composition should succeed");

    assert_eq!(first, second);
    assert_eq!(first.normalized_context(), b"core\n\nread-only\n\nguarded");
    let order: Vec<_> = first
        .manifest()
        .entries
        .iter()
        .map(|entry| (entry.class.as_str(), entry.overlay.as_str()))
        .collect();
    assert_eq!(
        order,
        vec![
            ("core", "core-default"),
            ("mode", "read-only"),
            ("mode", "guarded")
        ]
    );
    assert_eq!(first.manifest().source_bytes, 20);
    assert_eq!(first.manifest().output_bytes, 24);
}

#[test]
fn empty_segments_remain_structurally_distinct_in_normalized_output() {
    let source = MemorySource::default()
        .with("overlays/core-default.md", b"")
        .with("overlays/read-only.md", b"read-only")
        .with("overlays/guarded.md", b"guarded");
    let result = compose(
        &pack(false),
        &id("review"),
        &source,
        CompositionLimits::default(),
    )
    .expect("composition should succeed");

    assert_eq!(result.normalized_context(), b"\n\nread-only\n\nguarded");
    assert_eq!(result.segments().len(), 3);
}

#[test]
fn incompatibility_failure_is_deterministic_before_source_reads() {
    let result = compose(
        &pack(true),
        &id("review"),
        &MemorySource::default(),
        CompositionLimits::default(),
    );

    assert_eq!(
        result,
        Err(CompositionError::IncompatibleOverlays {
            overlay: id("core-default"),
            other: id("guarded"),
        })
    );
}

#[test]
fn composition_enforces_overlay_and_output_limits() {
    let pack = pack(false);
    let source = source();

    assert!(matches!(
        compose(
            &pack,
            &id("review"),
            &source,
            CompositionLimits::new(2, 64, 128),
        ),
        Err(CompositionError::TooManyOverlays {
            count: 3,
            maximum: 2
        })
    ));

    assert!(matches!(
        compose(
            &pack,
            &id("review"),
            &source,
            CompositionLimits::new(3, 64, 10),
        ),
        Err(CompositionError::OutputTooLarge { maximum: 10, .. })
    ));
}

#[test]
fn source_failure_categories_are_preserved() {
    let pack = pack(false);
    let result = compose(
        &pack,
        &id("review"),
        &MemorySource::default(),
        CompositionLimits::default(),
    );

    assert_eq!(
        result,
        Err(CompositionError::Source(SourceFailure::new(
            path("overlays/core-default.md"),
            SourceFailureKind::NotFound,
        )))
    );
}

#[test]
fn source_diagnostics_do_not_echo_attacker_controlled_paths() {
    let error = CompositionError::Source(SourceFailure::new(
        path("overlays/control\nsequence.md"),
        SourceFailureKind::NotFound,
    ));
    assert_eq!(error.to_string(), "overlay source was rejected: not found");
}

#[test]
fn profile_order_is_preserved_within_each_class() {
    let result = compose(
        &pack(false),
        &id("review"),
        &source(),
        CompositionLimits::default(),
    )
    .expect("composition should succeed");

    let selected: BTreeSet<_> = result
        .segments()
        .iter()
        .map(|segment| segment.overlay.as_str())
        .collect();
    assert_eq!(selected.len(), 3);
    assert_eq!(result.segments()[1].overlay, id("read-only"));
    assert_eq!(result.segments()[2].overlay, id("guarded"));
}
