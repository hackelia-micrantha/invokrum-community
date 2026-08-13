use invokrum_schema::{
    DeclarationKind, DeclarationLimits, SchemaError, SchemaLimits, parse_json_with_limits,
    parse_yaml_with_limits,
};

const MINIMAL_JSON: &str = r#"{"schema":"invokrum.dev/v1","id":"example","classes":[]}"#;
const MINIMAL_YAML: &str = "schema: invokrum.dev/v1\nid: example\nclasses: []\n";

const DECLARATION_JSON: &str = r#"{
  "schema":"invokrum.dev/v1",
  "id":"example",
  "classes":[
    {"id":"mode","order":10,"minimum":0,"maximum":2}
  ],
  "overlays":[
    {
      "id":"first",
      "class":"mode",
      "source":"first.md",
      "incompatible_with":["second"]
    },
    {
      "id":"second",
      "class":"mode",
      "source":"second.md"
    }
  ],
  "profiles":[
    {
      "id":"default",
      "selections":{"mode":["first"]}
    }
  ],
  "variables":[
    {"name":"token","sensitivity":"secret"}
  ]
}"#;

const DECLARATION_YAML: &str = r"
schema: invokrum.dev/v1
id: example
classes:
  - id: mode
    order: 10
    minimum: 0
    maximum: 2
overlays:
  - id: first
    class: mode
    source: first.md
    incompatible_with:
      - second
  - id: second
    class: mode
    source: second.md
profiles:
  - id: default
    selections:
      mode:
        - first
variables:
  - name: token
    sensitivity: secret
";

fn limits(
    document_bytes: usize,
    nesting_depth: usize,
    declarations: DeclarationLimits,
) -> SchemaLimits {
    SchemaLimits::new(document_bytes, nesting_depth, declarations)
}

fn exact_declarations() -> DeclarationLimits {
    DeclarationLimits::new(1, 2, 1, 1, 2, 1)
}

#[test]
fn document_bytes_are_checked_before_decode_for_both_formats() {
    let maximum_bytes = 8;
    let configured = limits(maximum_bytes, 32, DeclarationLimits::default());

    assert_eq!(
        parse_json_with_limits("this is not valid JSON", configured),
        Err(SchemaError::DocumentTooLarge {
            maximum_bytes,
            actual_bytes: 22,
        })
    );
    assert_eq!(
        parse_yaml_with_limits("this: is: not: valid: YAML", configured),
        Err(SchemaError::DocumentTooLarge {
            maximum_bytes,
            actual_bytes: 26,
        })
    );
}

#[test]
fn documents_at_the_exact_byte_limit_remain_valid() {
    parse_json_with_limits(
        MINIMAL_JSON,
        limits(MINIMAL_JSON.len(), 32, DeclarationLimits::default()),
    )
    .expect("JSON at the exact byte limit should be accepted");
    parse_yaml_with_limits(
        MINIMAL_YAML,
        limits(MINIMAL_YAML.len(), 32, DeclarationLimits::default()),
    )
    .expect("YAML at the exact byte limit should be accepted");
}

#[test]
fn documents_one_byte_over_the_limit_are_rejected() {
    let json_maximum = MINIMAL_JSON.len() - 1;
    let yaml_maximum = MINIMAL_YAML.len() - 1;

    assert_eq!(
        parse_json_with_limits(
            MINIMAL_JSON,
            limits(json_maximum, 32, DeclarationLimits::default()),
        ),
        Err(SchemaError::DocumentTooLarge {
            maximum_bytes: json_maximum,
            actual_bytes: MINIMAL_JSON.len(),
        })
    );
    assert_eq!(
        parse_yaml_with_limits(
            MINIMAL_YAML,
            limits(yaml_maximum, 32, DeclarationLimits::default()),
        ),
        Err(SchemaError::DocumentTooLarge {
            maximum_bytes: yaml_maximum,
            actual_bytes: MINIMAL_YAML.len(),
        })
    );
}

#[test]
fn structural_depth_is_bounded_before_schema_negotiation() {
    let configured = limits(1_024, 2, DeclarationLimits::default());
    let deep_json = r#"{"schema":"invokrum.dev/v2","future":[[[]]]}"#;
    let deep_yaml = "schema: invokrum.dev/v2\nfuture:\n  - - - []\n";

    assert_eq!(
        parse_json_with_limits(deep_json, configured),
        Err(SchemaError::NestingTooDeep { maximum_depth: 2 })
    );
    assert_eq!(
        parse_yaml_with_limits(deep_yaml, configured),
        Err(SchemaError::NestingTooDeep { maximum_depth: 2 })
    );
}

#[test]
fn documents_at_the_exact_structural_depth_remain_valid() {
    let configured = limits(1_024, 2, DeclarationLimits::default());

    parse_json_with_limits(MINIMAL_JSON, configured)
        .expect("JSON at exact container depth should be accepted");
    parse_yaml_with_limits(MINIMAL_YAML, configured)
        .expect("YAML at exact container depth should be accepted");
}

#[test]
fn declarations_at_every_exact_limit_remain_valid() {
    let configured = limits(16_384, 16, exact_declarations());

    parse_json_with_limits(DECLARATION_JSON, configured)
        .expect("JSON at exact declaration limits should be accepted");
    parse_yaml_with_limits(DECLARATION_YAML, configured)
        .expect("YAML at exact declaration limits should be accepted");
}

#[test]
fn declaration_limits_fail_before_domain_validation() {
    let declarations = DeclarationLimits::new(0, 2, 1, 1, 2, 1);
    let configured = limits(16_384, 16, declarations);
    let invalid_json = DECLARATION_JSON.replace("\"id\":\"mode\"", "\"id\":\"invalid id\"");
    let invalid_yaml = DECLARATION_YAML.replace("id: mode", "id: invalid id");
    let expected = Err(SchemaError::TooManyDeclarations {
        kind: DeclarationKind::Class,
        maximum: 0,
        actual: 1,
    });

    assert_eq!(parse_json_with_limits(&invalid_json, configured), expected);
    assert_eq!(parse_yaml_with_limits(&invalid_yaml, configured), expected);
}

#[test]
fn each_declaration_category_fails_one_step_over_the_limit() {
    let cases = [
        (
            DeclarationKind::Class,
            DeclarationLimits::new(0, 2, 1, 1, 2, 1),
            0,
            1,
        ),
        (
            DeclarationKind::Overlay,
            DeclarationLimits::new(1, 1, 1, 1, 2, 1),
            1,
            2,
        ),
        (
            DeclarationKind::Profile,
            DeclarationLimits::new(1, 2, 0, 1, 2, 1),
            0,
            1,
        ),
        (
            DeclarationKind::Variable,
            DeclarationLimits::new(1, 2, 1, 0, 2, 1),
            0,
            1,
        ),
        (
            DeclarationKind::Selection,
            DeclarationLimits::new(1, 2, 1, 1, 1, 1),
            1,
            2,
        ),
        (
            DeclarationKind::Incompatibility,
            DeclarationLimits::new(1, 2, 1, 1, 2, 0),
            0,
            1,
        ),
    ];

    for (kind, declarations, maximum, actual) in cases {
        let configured = limits(16_384, 16, declarations);
        let expected = Err(SchemaError::TooManyDeclarations {
            kind,
            maximum,
            actual,
        });

        assert_eq!(
            parse_json_with_limits(DECLARATION_JSON, configured),
            expected,
            "JSON should enforce the {kind} declaration limit"
        );
        assert_eq!(
            parse_yaml_with_limits(DECLARATION_YAML, configured),
            expected,
            "YAML should enforce the {kind} declaration limit"
        );
    }
}
