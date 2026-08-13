use invokrum_schema::{
    SCHEMA_FAMILY, SchemaError, YamlFeature, parse_json, parse_yaml, to_normalized_json,
};
use serde_json::Value;

#[test]
fn yaml_and_json_produce_the_same_normalized_model() {
    let yaml = include_str!("../../../tests/fixtures/schema/minimal-pack.yaml");
    let json = include_str!("../../../tests/fixtures/schema/minimal-pack.json");

    let yaml_pack = parse_yaml(yaml).expect("YAML fixture should be valid");
    let json_pack = parse_json(json).expect("JSON fixture should be valid");

    assert_eq!(
        to_normalized_json(&yaml_pack).expect("YAML pack should serialize"),
        to_normalized_json(&json_pack).expect("JSON pack should serialize")
    );
}

#[test]
fn schema_rejects_malformed_json_and_yaml() {
    assert!(matches!(
        parse_json(r#"{"schema":"invokrum.dev/v1""#),
        Err(SchemaError::Decode { format: "json", .. })
    ));
    assert!(matches!(
        parse_yaml("schema: [invokrum.dev/v1"),
        Err(SchemaError::Decode { format: "yaml", .. })
    ));
}

#[test]
fn duplicate_named_fields_have_a_stable_error_category() {
    let duplicate_json = r#"{
        "schema":"invokrum.dev/v1",
        "id":"first",
        "id":"second",
        "classes":[]
    }"#;
    let duplicate_yaml = "schema: invokrum.dev/v1\nid: first\nid: second\nclasses: []\n";

    assert_eq!(
        parse_json(duplicate_json),
        Err(SchemaError::DuplicateMappingKey { format: "json" })
    );
    assert_eq!(
        parse_yaml(duplicate_yaml),
        Err(SchemaError::DuplicateMappingKey { format: "yaml" })
    );
}

#[test]
fn duplicate_profile_selection_keys_are_rejected_at_any_mapping_depth() {
    let duplicate_json = r#"{
        "schema":"invokrum.dev/v1",
        "id":"example",
        "classes":[{"id":"mode","order":10,"minimum":0}],
        "profiles":[{
            "id":"review",
            "selections":{
                "mode":[],
                "mode":[]
            }
        }]
    }"#;
    let duplicate_yaml = r"
schema: invokrum.dev/v1
id: example
classes:
  - id: mode
    order: 10
    minimum: 0
profiles:
  - id: review
    selections:
      mode: []
      mode: []
";

    assert_eq!(
        parse_json(duplicate_json),
        Err(SchemaError::DuplicateMappingKey { format: "json" })
    );
    assert_eq!(
        parse_yaml(duplicate_yaml),
        Err(SchemaError::DuplicateMappingKey { format: "yaml" })
    );
}

#[test]
fn ambiguous_future_documents_fail_before_version_negotiation() {
    let duplicate_future_field = r#"{
        "schema":"invokrum.dev/v2",
        "id":"example",
        "classes":[],
        "future":{"mode":"first","mode":"second"}
    }"#;

    assert_eq!(
        parse_json(duplicate_future_field),
        Err(SchemaError::DuplicateMappingKey { format: "json" })
    );
}

#[test]
fn schema_rejects_unknown_v1_fields() {
    let unknown_field = r#"{
        "schema":"invokrum.dev/v1",
        "id":"example",
        "classes":[],
        "unexpected":true
    }"#;

    assert!(matches!(
        parse_json(unknown_field),
        Err(SchemaError::Decode { format: "json", .. })
    ));
}

#[test]
fn unambiguous_unsupported_schema_precedes_strict_v1_field_decoding() {
    let unsupported = r#"{
        "schema":"invokrum.dev/v2",
        "id":"example",
        "classes":[],
        "future_only":true
    }"#;

    assert_eq!(
        parse_json(unsupported),
        Err(SchemaError::UnsupportedSchema("invokrum.dev/v2".to_owned()))
    );
}

#[test]
fn json_key_named_like_yaml_merge_is_not_treated_as_yaml_syntax() {
    let unsupported = r#"{
        "schema":"invokrum.dev/v2",
        "id":"example",
        "classes":[],
        "<<":{}
    }"#;

    assert_eq!(
        parse_json(unsupported),
        Err(SchemaError::UnsupportedSchema("invokrum.dev/v2".to_owned()))
    );
}

#[test]
fn yaml_subset_rejects_parser_expansion_and_complex_features() {
    let cases = [
        (
            "%YAML 1.2\nschema: invokrum.dev/v1\nid: example\nclasses: []\n",
            YamlFeature::Directive,
        ),
        (
            "schema: invokrum.dev/v1\nid: example\nclasses: &shared []\n",
            YamlFeature::Anchor,
        ),
        (
            "schema: invokrum.dev/v1\nid: example\nclasses: *shared\n",
            YamlFeature::Alias,
        ),
        (
            "schema: invokrum.dev/v1\nid: example\n<<    : {}\nclasses: []\n",
            YamlFeature::MergeKey,
        ),
        (
            "schema: invokrum.dev/v1\nid: !custom example\nclasses: []\n",
            YamlFeature::Tag,
        ),
        (
            "schema: invokrum.dev/v1\nid: |\n  example\nclasses: []\n",
            YamlFeature::BlockScalar,
        ),
        (
            "? schema\n: invokrum.dev/v1\nid: example\nclasses: []\n",
            YamlFeature::ExplicitMappingKey,
        ),
        (
            "schema: invokrum.dev/v1\nid: example\nclasses: []\n...\n",
            YamlFeature::DocumentEndMarker,
        ),
    ];

    for (input, feature) in cases {
        assert_eq!(
            parse_yaml(input),
            Err(SchemaError::UnsupportedYamlFeature(feature)),
            "feature should be rejected: {feature}"
        );
    }
}

#[test]
fn yaml_rejects_non_string_mapping_keys() {
    let input = r"
schema: invokrum.dev/v1
id: example
classes:
  - id: mode
    order: 10
    minimum: 0
profiles:
  - id: review
    selections:
      1: []
";

    assert!(matches!(
        parse_yaml(input),
        Err(SchemaError::Decode { format: "yaml", .. })
    ));
}

#[test]
fn yaml_rejects_multiple_documents_but_accepts_one_start_marker() {
    let one_document = r"
---
schema: invokrum.dev/v1
id: example
classes: []
";
    let multiple_documents = r"
---
schema: invokrum.dev/v1
id: first
classes: []
---
schema: invokrum.dev/v1
id: second
classes: []
";

    parse_yaml(one_document).expect("one explicit YAML document should be accepted");
    assert_eq!(
        parse_yaml(multiple_documents),
        Err(SchemaError::MultipleYamlDocuments)
    );
}

#[test]
fn unsupported_schema_names_are_bounded_in_errors() {
    let schema = "x".repeat(1_024);
    let document = format!("{{\"schema\":\"{schema}\",\"id\":\"example\",\"classes\":[]}}");

    let Err(SchemaError::UnsupportedSchema(reported)) = parse_json(&document) else {
        panic!("long schema family should be unsupported");
    };
    assert!(reported.chars().count() <= 129);
    assert!(reported.ends_with('…'));
}

#[test]
fn duplicate_incompatibility_entries_are_rejected_instead_of_deduplicated() {
    let duplicate = r#"{
        "schema":"invokrum.dev/v1",
        "id":"example",
        "classes":[
            {"id":"mode","order":10,"minimum":0}
        ],
        "overlays":[
            {
                "id":"read-only",
                "class":"mode",
                "source":"overlays/read-only.md",
                "incompatible_with":["read-only","read-only"]
            }
        ]
    }"#;

    assert!(matches!(
        parse_json(duplicate),
        Err(SchemaError::DuplicateListValue {
            field: "overlays[].incompatible_with",
            ..
        })
    ));
}

#[test]
fn optional_maximum_defaults_to_unbounded() {
    let document = r#"{
        "schema":"invokrum.dev/v1",
        "id":"example",
        "classes":[
            {"id":"quality","order":10,"minimum":0}
        ]
    }"#;

    let pack = parse_json(document).expect("maximum should be optional");
    assert_eq!(pack.classes()[0].cardinality.maximum(), None);
}

#[test]
fn machine_schema_is_valid_json_and_bound_to_the_runtime_family() {
    let schema_text = include_str!("../../../schemas/invokrum-pack-v1.schema.json");
    let schema: Value = serde_json::from_str(schema_text).expect("schema should be valid JSON");

    assert_eq!(
        schema["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
    assert_eq!(schema["properties"]["schema"]["const"], SCHEMA_FAMILY);
    assert_eq!(schema["additionalProperties"], false);

    let required = schema["$defs"]["class"]["required"]
        .as_array()
        .expect("class required fields should be an array");
    assert!(!required.iter().any(|field| field == "maximum"));
}
