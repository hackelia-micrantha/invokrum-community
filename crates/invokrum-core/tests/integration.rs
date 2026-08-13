use std::collections::{BTreeMap, BTreeSet};

use invokrum_core::{
    Cardinality, DomainError, Identifier, Overlay, OverlayClass, OverlayPack, PackRelativePath,
    Profile,
};

fn id(value: &str) -> Identifier {
    Identifier::parse(value).expect("test identifier should be valid")
}

fn path(value: &str) -> PackRelativePath {
    PackRelativePath::parse(value).expect("test path should be valid")
}

fn class(name: &str, order: u32, minimum: u32, maximum: Option<u32>) -> OverlayClass {
    OverlayClass {
        id: id(name),
        order,
        cardinality: Cardinality::new(minimum, maximum).expect("valid cardinality"),
    }
}

fn overlay(name: &str, class_name: &str) -> Overlay {
    Overlay {
        id: id(name),
        class: id(class_name),
        source: path(&format!("overlays/{name}.md")),
        incompatible_with: BTreeSet::new(),
    }
}

#[test]
fn pack_construction_normalizes_declared_class_order() {
    let classes = vec![
        class("quality", 30, 0, None),
        class("core", 10, 1, Some(1)),
        class("mode", 20, 1, Some(1)),
    ];
    let overlays = vec![
        overlay("read-only", "mode"),
        overlay("core-default", "core"),
    ];
    let profile = Profile {
        id: id("review"),
        selections: BTreeMap::from([
            (id("core"), vec![id("core-default")]),
            (id("mode"), vec![id("read-only")]),
        ]),
    };

    let pack = OverlayPack::new(
        id("example"),
        "test/v1",
        classes,
        overlays,
        vec![profile],
        Vec::new(),
    )
    .expect("pack should be valid");

    let ordered: Vec<_> = pack
        .classes()
        .iter()
        .map(|class| class.id.as_str())
        .collect();
    assert_eq!(ordered, vec!["core", "mode", "quality"]);

    let overlays: Vec<_> = pack
        .overlays()
        .iter()
        .map(|overlay| overlay.id.as_str())
        .collect();
    assert_eq!(overlays, vec!["core-default", "read-only"]);
}

#[test]
fn pack_rejects_duplicate_declarations() {
    let duplicate = overlay("same", "core");
    let result = OverlayPack::new(
        id("example"),
        "test/v1",
        vec![class("core", 10, 0, None)],
        vec![duplicate.clone(), duplicate],
        Vec::new(),
        Vec::new(),
    );

    assert!(matches!(
        result,
        Err(DomainError::DuplicateIdentifier {
            kind: "overlay",
            ..
        })
    ));
}

#[test]
fn pack_rejects_profile_selection_from_the_wrong_class() {
    let classes = vec![class("core", 10, 0, None), class("mode", 20, 0, None)];
    let overlays = vec![overlay("read-only", "mode")];
    let profile = Profile {
        id: id("invalid"),
        selections: BTreeMap::from([(id("core"), vec![id("read-only")])]),
    };

    let result = OverlayPack::new(
        id("example"),
        "test/v1",
        classes,
        overlays,
        vec![profile],
        Vec::new(),
    );

    assert!(matches!(
        result,
        Err(DomainError::OverlayClassMismatch { .. })
    ));
}

#[test]
fn pack_rejects_profile_that_omits_a_required_class() {
    let profile = Profile {
        id: id("invalid"),
        selections: BTreeMap::new(),
    };

    let result = OverlayPack::new(
        id("example"),
        "test/v1",
        vec![class("core", 10, 1, Some(1))],
        Vec::new(),
        vec![profile],
        Vec::new(),
    );

    assert!(matches!(
        result,
        Err(DomainError::CardinalityViolation { count: 0, .. })
    ));
}

#[test]
fn pack_rejects_profile_above_maximum_cardinality() {
    let profile = Profile {
        id: id("invalid"),
        selections: BTreeMap::from([(id("mode"), vec![id("read-only"), id("write-enabled")])]),
    };

    let result = OverlayPack::new(
        id("example"),
        "test/v1",
        vec![class("mode", 10, 0, Some(1))],
        vec![
            overlay("read-only", "mode"),
            overlay("write-enabled", "mode"),
        ],
        vec![profile],
        Vec::new(),
    );

    assert!(matches!(
        result,
        Err(DomainError::CardinalityViolation { count: 2, .. })
    ));
}

#[test]
fn pack_rejects_duplicate_profile_selection() {
    let profile = Profile {
        id: id("invalid"),
        selections: BTreeMap::from([(id("mode"), vec![id("read-only"), id("read-only")])]),
    };

    let result = OverlayPack::new(
        id("example"),
        "test/v1",
        vec![class("mode", 10, 0, None)],
        vec![overlay("read-only", "mode")],
        vec![profile],
        Vec::new(),
    );

    assert!(matches!(
        result,
        Err(DomainError::DuplicateProfileSelection(identifier))
            if identifier == id("read-only")
    ));
}

#[test]
fn pack_rejects_dangling_profile_selection() {
    let profile = Profile {
        id: id("invalid"),
        selections: BTreeMap::from([(id("mode"), vec![id("missing")])]),
    };

    let result = OverlayPack::new(
        id("example"),
        "test/v1",
        vec![class("mode", 10, 0, None)],
        Vec::new(),
        vec![profile],
        Vec::new(),
    );

    assert!(matches!(
        result,
        Err(DomainError::UnknownOverlayReference(identifier))
            if identifier == id("missing")
    ));
}
