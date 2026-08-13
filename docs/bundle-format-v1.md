# Pack bundle format v1

**Status:** Implemented bundle identity and trust-policy contract used by the supported local directory/archive installation paths. Exact candidate verification, `ed25519-subject-v1` verification, host authorization, and Linux installation evidence are implemented; remote transport, freshness/revocation, rollback selection, and registry discovery remain planned.

`invokrum.pack-bundle/v1` defines the immutable distributed-content subject that digest-pinned and publisher-authenticated installation flows bind before a pack enters ordinary composition. It is deliberately separate from `invokrum.lock/v1`: the bundle manifest identifies distributed files, while the lock identifies a resolved composition and its rendered output.

## Boundary

```text
local directory / bounded ustar / future remote candidate bytes
        │
        ▼
invokrum.pack-bundle/v1
  exact file identities
        │
        +-----------------------+
        │                       │
        ▼                       ▼
 exact candidate verify   ed25519-subject-v1
        │                       │
        ▼                       ▼
  VerifiedBundle        PublisherAssertion
        │                       │
        │                 host TrustPolicy
        │                       │
        │                       ▼
        │                AuthorizedPublisher
        │                       │
        +-----------+-----------+
                    │
                    ▼
        Linux quarantine / CAS install
                    │
                    ▼
           protected local pack root
                    │
                    ▼
       existing offline composition
          + invokrum.lock/v1
```

The distribution domain performs no network, filesystem, clock, credential, environment, or signing-provider access. `invokrum-distribution` owns parsing-neutral bundle and trust values. `invokrum-distribution-json` owns strict JSON decoding, canonical bytes, duplicate-key rejection, and SHA-256 bundle-subject derivation using the dependency-free `invokrum-digest` primitive.

## Canonical JSON document

The exact top-level field order is:

1. `format`
2. `digest_algorithm`
3. `entry_point`
4. `files`

Each file record uses:

1. `path`
2. `byte_length`
3. `digest`

Example:

```json
{"format":"invokrum.pack-bundle/v1","digest_algorithm":"sha256","entry_point":"pack.yaml","files":[{"path":"overlays/core.md","byte_length":12,"digest":"1111111111111111111111111111111111111111111111111111111111111111"},{"path":"pack.yaml","byte_length":42,"digest":"0000000000000000000000000000000000000000000000000000000000000000"}]}
```

The machine-readable schema is [`schemas/invokrum-pack-bundle-v1.schema.json`](../schemas/invokrum-pack-bundle-v1.schema.json).

## Canonicalization

Validated file records are sorted lexicographically by their portable path. Canonical JSON is compact UTF-8 JSON emitted from fixed structs; no insignificant whitespace is present.

The strict decoder requires input bytes to equal canonical re-encoding exactly. Therefore these all fail closed:

- alternate object-key order;
- noncanonical file order;
- leading/trailing or interstitial insignificant whitespace;
- duplicate JSON object keys;
- unknown fields;
- unsupported format or digest identifiers.

This intentionally makes the signed/digest-pinned subject one exact byte representation rather than a family of semantically equivalent JSON documents.

## Bundle subject digest

The v1 immutable subject is:

```text
sha256(canonical invokrum.pack-bundle/v1 bytes)
```

For the repository golden fixture, the canonical subject digest is:

```text
7e908eda57397f79992a6236bcfb6f2bef38fd9f28f51b4ef767d81357753f86
```

This digest identifies the bundle manifest and therefore the exact enumerated file tree. It does **not** identify or authorize a publisher by itself.

## File records and hard limits

Every installable file must appear exactly once. Each file record binds:

- one portable pack-relative path;
- exact byte length;
- lowercase hexadecimal SHA-256 digest.

The entry point must identify one enumerated file. Duplicate paths and missing entry points fail closed.

The v1 format hard maxima are:

| Limit | v1 maximum |
| --- | ---: |
| Manifest input bytes | 1 MiB |
| File records | 512 |
| Bytes per file | 1 MiB |
| Aggregate expanded bytes | 32 MiB |
| Path bytes | 1024 |

The JSON adapter checks file-count and byte limits before constructing the validated aggregate. Hosts may inject **tighter** file-count, per-file, and aggregate limits, but `BundleLimits` clamps attempts to exceed the v1 maxima. A caller therefore cannot make runtime decoding accept a document that violates the published v1 size contract by simply raising host limits.

## Path grammar

V1 bundle paths are **printable ASCII** using `/` separators. The ASCII restriction makes JSON Schema `maxLength` and Rust byte-length limits equivalent and avoids Unicode-normalization ambiguity in signed file identities.

Paths reject:

- non-ASCII characters;
- ASCII control characters;
- absolute paths;
- trailing `/`;
- backslashes;
- `:` platform prefixes;
- empty components;
- `.` and `..` components;
- paths over 1024 bytes/characters.

The implemented candidate adapters add source-specific checks before exact verification:

- Linux directory loading rejects links, hard links, special files, device crossings, root escapes, undeclared entries, unsupported names, case-folding logical collisions, unstable identities, and resource-limit violations;
- bounded uncompressed POSIX ustar ingestion rejects traversal/absolute names, links and special entries, logical collisions, undeclared/missing files, malformed/trailing forms, unsupported metadata, and resource-limit violations without extracting archive entries to the host filesystem.

Both adapters return the same owned `CandidateFile` representation and converge on the same exact manifest/content verification use case.

## Publisher assertion and host policy

`invokrum-distribution` defines provider-neutral trust values:

- normalized verification-mechanism identifier;
- exact SHA-256 signed subject;
- normalized publisher identity attributes;
- host-owned allow rules;
- deterministic authorization failures.

Verification mechanisms and attribute names are normalized lowercase identifiers. Identity values are bounded and reject control characters. Host rules match an explicit mechanism plus required identity attributes.

The first concrete verifier is `ed25519-subject-v1`. It verifies a domain-separated message for the exact bundle subject using raw Ed25519 public-key/signature evidence and normalizes the verified key identity as `key.sha256`.

Authorization requires both:

1. the concretely verified assertion subject equals the expected bundle-subject digest; and
2. an explicit host-owned publisher rule matches the assertion mechanism and normalized identity.

A cryptographically valid assertion from an unrecognized signer is denied. Pack metadata contains no publisher-authorization field and cannot relax host policy.

## Installation evidence

The supported Linux installer consumes `VerifiedBundle` directly and never reopens the original candidate source.

Digest-only/unauthenticated installs use `invokrum.installation/v1` and record exactly:

```text
publisher_authentication: not-provided
```

Authenticated installs require concrete verifier success plus host authorization for the same exact subject and use `invokrum.installation/v2` with structured `verified-and-authorized` publisher evidence.

Existing content-addressed subject roots require exact expected evidence bytes for reuse. An existing v1 root cannot be silently upgraded to v2, a v2 root cannot be silently downgraded to v1, and a v2 root cannot silently change signer/mechanism while preserving the same subject path.

## Claims not provided yet

The current implementation does not:

- fetch remote/network candidates;
- discover registries or mirrors;
- evaluate publisher-key expiry, revocation, transparency, or authenticated freshness;
- implement rollback/freeze version-selection policy;
- verify Sigstore, X.509, PGP, or other signature systems beyond the implemented `ed25519-subject-v1` mechanism;
- provide non-Linux installation adapters;
- expose the completed local/archive install workflow through the final public API/CLI surface (#64).

These capabilities must preserve the exact bundle/trust contract and offline composition boundary defined in [ADR-0002](architecture/ADR-0002-publisher-trust-and-acquisition-boundary.md) and the [publisher trust architecture](security/publisher-trust.md).

## Compatibility

`invokrum.pack-bundle/v1` is independent from:

- pack schema version;
- engine version;
- `invokrum.lock/v1`;
- signing provider or verification mechanism;
- transport container format.

An incompatible canonicalization or subject-identity rule requires a new bundle format identifier rather than silently changing v1 bytes.
