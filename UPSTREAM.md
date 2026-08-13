# Upstream provenance

`hackelia-micrantha/invokrum-community` is the public distribution of Invokrum. Canonical development is maintained separately.

## Initial cutover

The exact upstream cutover commit is intentionally not selected during split preparation because `invokrum` may continue public development before the visibility change.

Before the canonical repository becomes private, this file MUST be updated with:

```text
cutover_source_repository: hackelia-micrantha/invokrum
cutover_source_commit: <40-hex commit SHA>
cutover_timestamp_utc: <RFC3339 timestamp>
license_at_cutover: Apache-2.0
```

The baseline-import pull request must also list every repository-relocation transformation made after copying the selected public snapshot. Typical allowed transformations include repository URLs, public README text, issue/security links, and CI/release destinations.

All other differences from the selected public upstream snapshot must be intentional, reviewed, and documented.

## Later promotions

Each canonical-to-community promotion should record the canonical source commit or commit range in its pull request description or machine-readable provenance metadata.

Future private Git history is not mirrored here. The public repository receives only reviewed public content and the minimum provenance needed to attribute that content to the canonical source state.