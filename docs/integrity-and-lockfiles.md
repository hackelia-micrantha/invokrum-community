# Integrity, canonical manifests, and lockfiles

## Ownership

`invokrum-integrity` is an outward adapter. It consumes validated `invokrum-core` values and exact composition bytes, then produces deterministic evidence. It does not read files, acquire packs, invoke runtimes, or persist output. Filesystem persistence and CLI wiring remain delivery concerns.

The adapter owns:

- canonical evidence encoding;
- SHA-256 calculation;
- versioned lockfile decoding and validation;
- drift classification;
- internal lock consistency checks.

`invokrum-core` remains independent of Serde, JSON, and hashing implementations.

## Version identifiers

V1 uses:

- lockfile format: `invokrum.lock/v1`;
- canonical encoding: `invokrum.canonical-json/v1`;
- digest algorithm: `sha256`.

Unknown identifiers fail closed before drift comparison.

## Canonical JSON rules

Canonical bytes are compact UTF-8 JSON with no insignificant whitespace or trailing newline. Field order is the declaration order of the versioned format structs. Domain collections are already normalized before encoding:

- classes by explicit numeric order;
- overlays, profiles, and variables by identifier;
- profile selection maps by class identifier;
- incompatibility sets by overlay identifier;
- composed overlays by class order and explicit profile order.

Numbers are JSON integers. Strings are escaped by the JSON encoder without platform path conversion. Pack-relative paths retain their validated forward-slash representation.

The v1 decoder requires the supplied bytes to equal the canonical re-encoding exactly. Whitespace changes, reordered fields, duplicate keys, alternate string escapes, and trailing bytes fail rather than being silently normalized. Changing any canonical rule requires a new canonicalization identifier.

## Resource and identity limits

V1 fails before unbounded decoding when:

- lockfile input exceeds 1 MiB;
- a manifest contains more than 256 selected overlays.

Pack, profile, overlay, and class identities must satisfy the core identifier grammar. Every overlay source must satisfy the portable pack-relative path grammar. This prevents decoded evidence from introducing path traversal or control-character identities that could later cross a diagnostic or output boundary.

These limits apply to lock evidence. Schema-document and composition limits remain separately defined by their owning adapters.

## Digest domains

All digests are lowercase 64-character SHA-256 hexadecimal strings.

### Pack metadata digest

The pack digest covers canonical validated pack metadata:

- schema family and pack identifier;
- classes, ordering, and cardinality;
- overlay identifiers, classes, source paths, and incompatibilities;
- all profile declarations and selections;
- variable names and sensitivity declarations.

It does not cover overlay source bytes; each selected source has a separate digest.

### Selected-profile digest

The profile digest covers the selected profile identifier and its ordered class selections. This remains separately explainable even though profile declarations also contribute to the pack digest.

### Overlay content digests

Each selected overlay records class, overlay identifier, source path, exact byte length, and a digest of the exact bytes returned by the source adapter. Integrity code never reopens source paths.

### Engine-input digest

The engine-input digest covers canonical pack and profile digests plus the ordered selected-overlay records. It excludes rendered output so input and output mismatches remain distinguishable.

### Output digest

The output digest covers the exact normalized context bytes returned by composition.

### Manifest digest

The manifest digest covers the canonical manifest object. It detects accidental corruption and internally inconsistent edits before repository drift analysis.

The manifest digest is not a signature or message-authentication code. Anyone able to replace a lockfile can recompute unkeyed digests. Publisher identity, signatures, trusted distribution, and authorization remain host responsibilities.

## Lockfile structure

```json
{
  "format": "invokrum.lock/v1",
  "canonicalization": "invokrum.canonical-json/v1",
  "digest_algorithm": "sha256",
  "manifest": {
    "engine_inputs_digest": "<sha256>",
    "pack": {
      "id": "example",
      "schema": "invokrum.dev/v1",
      "digest": "<sha256>"
    },
    "profile": {
      "id": "default",
      "digest": "<sha256>"
    },
    "overlays": [
      {
        "class": "core",
        "id": "core-default",
        "source": "overlays/core.md",
        "byte_length": 4,
        "digest": "<sha256>"
      }
    ],
    "output": {
      "byte_length": 4,
      "digest": "<sha256>"
    }
  },
  "manifest_digest": "<sha256>"
}
```

Unknown fields and duplicate known fields are rejected by the v1 decoder.

## Verification order

Verification is deterministic and fail-closed:

1. reject an oversized byte stream;
2. decode strict JSON;
3. validate format, canonicalization, and digest algorithm identifiers;
4. enforce overlay-count and identity/path limits;
5. validate every digest representation;
6. recompute the stored engine-input digest;
7. recompute the stored manifest digest;
8. require exact canonical input bytes;
9. generate current lock material from the supplied pack and composition;
10. report ordered drift categories.

Drift categories are:

1. pack metadata;
2. selected profile;
3. selected overlay set or identity;
4. per-overlay content by deterministic index;
5. rendered output.

A malformed, unsupported, noncanonical, or internally inconsistent lock is an integrity error, not repository drift.

## Sensitive data policy

The v1 operation accepts no variable-value input and the lockfile has no variable-value field. Secret values therefore cannot be persisted by this API. Pack metadata is represented by a digest rather than embedding variable declarations, so secret variable names are not exposed in lock bytes either.

Future interpolation must define an explicit redacted identity model before values can influence manifests or lockfiles. Secret values must remain excluded by default.

## Cross-platform guarantee

Lock generation depends only on validated domain strings, ordered collections, exact source bytes, exact normalized output bytes, integer lengths, canonical JSON, and SHA-256. It does not hash filesystem metadata, native path encodings, timestamps, ownership, permissions, locale, or line-ending transformations.

Identical validated inputs and source bytes therefore produce identical v1 lock bytes on every platform that implements the same composition and canonicalization contracts.
