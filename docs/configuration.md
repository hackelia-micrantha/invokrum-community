# Configuration model

> [!NOTE]
> The `invokrum.dev/v1` pack schema, deterministic composition, bounded Linux file loading, rendering, canonical lockfiles, verification/diff behavior, and the current offline CLI commands are implemented. Remote acquisition is not part of ordinary composition. The explicit install API/CLI surface is tracked separately in #64.

## Overview

A v1 Invokrum pack consists of:

- a schema family and pack identifier;
- explicitly ordered overlay classes;
- overlay definitions backed by pack-relative local files;
- named profiles selecting overlays by class;
- optional incompatibility declarations;
- optional variables with sensitivity metadata.

## Implemented pack shape

```yaml
schema: invokrum.dev/v1
id: example

classes:
  - id: core
    order: 10
    minimum: 1
    maximum: 1
  - id: security
    order: 30
    minimum: 0
  - id: mode
    order: 90
    minimum: 1
    maximum: 1

overlays:
  - id: core-invariant
    class: core
    source: overlays/core/invariant.md
  - id: security-default
    class: security
    source: overlays/security/default.md
  - id: review
    class: mode
    source: overlays/modes/review.md
    incompatible_with:
      - implementation

profiles:
  - id: secure-review
    selections:
      core:
        - core-invariant
      security:
        - security-default
      mode:
        - review

variables:
  - name: repository
    sensitivity: public
  - name: access_token
    sensitivity: secret
```

Unknown fields are rejected. Unsupported schema families fail before strict v1 decoding.

## Ordering and normalization

Class order is explicit through `order`. Filesystem traversal and input map iteration never determine precedence.

The validated aggregate normalizes:

- classes by explicit order;
- overlays, profiles, and variables by identifier;
- map keys and set-like incompatibility declarations deterministically.

Selection order within a class is preserved as explicit input.

## Cardinality

Each class declares a `minimum` and an optional `maximum` using unsigned 32-bit values.

Typical examples:

- exactly one core overlay: `minimum: 1`, `maximum: 1`;
- exactly one mode: `minimum: 1`, `maximum: 1`;
- optional environment: `minimum: 0`, `maximum: 1`;
- additive quality overlays: `minimum: 0`, with `maximum` omitted.

Omitted or `null` maximum values are unbounded above. A maximum lower than the minimum is invalid.

## Compatibility declarations

Each overlay may declare `incompatible_with` as a list of overlay identifiers. Referenced overlays must exist. Duplicate entries are rejected rather than silently normalized.

Selected incompatibilities are evaluated deterministically during composition and fail before source reads when the selected profile is invalid.

## Paths

Overlay sources use portable `/`-separated pack-relative paths. The v1 lexical grammar rejects:

- absolute paths;
- Windows-style prefixes and backslashes;
- `:` characters;
- empty, `.`, or `..` segments;
- trailing separators.

The Linux composition adapter additionally establishes and pins a canonical pack root, rejects links and unsupported filesystem states, verifies opened-file containment/identity, and fails closed on root escape or unstable reads. Runtime HTTP or Git references remain out of scope for ordinary v1 composition.

## Variables

Variables are explicit declarations, not ambient environment reads. The implemented v1 declaration contains:

- `name`;
- `sensitivity`: `public` or `secret`.

Variable values, defaults, interpolation, and rendering constraints are deferred. Sensitive values must not enter diagnostics, manifests, or lockfiles by default.

## Profiles

Profiles contain a `selections` object keyed by class identifier. Each value is an ordered list of overlay identifiers. Profiles cannot redefine class order or engine behavior, and profile inheritance is not part of v1.

## Machine-readable schema

The JSON Schema is [`schemas/invokrum-pack-v1.schema.json`](../schemas/invokrum-pack-v1.schema.json). See [schema-v1.md](schema-v1.md) for validation and normalization details.
