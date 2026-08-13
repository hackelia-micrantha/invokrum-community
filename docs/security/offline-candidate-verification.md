# Offline bundle candidate verification

**Status:** Implemented for bounded Linux local-directory and uncompressed POSIX ustar candidates, exact expected-subject/content verification, Linux content-addressed installation, and the authenticated Ed25519 + host-policy workflow. Remote/network transport, registries, freshness/revocation, and rollback selection remain separate and unimplemented.

`invokrum-acquisition` verifies that one bounded set of exact candidate bytes matches a validated `invokrum.pack-bundle/v1` manifest and the immutable bundle subject expected by the caller. `invokrum-acquisition-linux` supplies those exact owned bytes from an already-local Linux directory under a fail-closed filesystem policy. `invokrum-acquisition-archive` supplies the same `CandidateFile` representation from the supported bounded POSIX ustar profile without extracting archive content to the host filesystem.

## Boundary

```text
Linux directory / bounded POSIX ustar / future remote adapter
                         │ exact in-memory bytes
                         ▼
                  CandidateFile[]
                         │
                         ▼
                invokrum-acquisition
                  subject equality
                  path-set equality
                  byte-length equality
                  SHA-256 equality
                         │
                         ▼
                   VerifiedBundle
                  exact verified bytes
                         │
              +----------+-----------+
              |                      |
              | digest-only          | authenticated
              |                      v
              |             ed25519-subject-v1
              |                      |
              |             PublisherAssertion
              |                      |
              |                TrustPolicy
              |                      |
              |              AuthorizedPublisher
              |                      |
              +----------+-----------+
                         |
                         v
            invokrum-install / LinuxInstallStore
              private quarantine -> staged reverify
                       -> sha256/<subject>
```

The in-memory verification use case performs no filesystem, archive, network, process, environment, clock, credential, trust-store, serialization, or signature-provider access. Filesystem, archive parsing, concrete cryptography, authorization, and installation remain isolated in outer adapters/use cases.

## Inputs

The exact-content verification layer receives:

- a validated `BundleManifest`;
- `manifest_subject`, derived by the canonical distribution serialization boundary;
- `expected_subject`, obtained from an explicit trusted digest channel or the authenticated workflow's exact subject;
- candidate files represented as validated `BundlePath` plus owned exact bytes.

The acquisition crate intentionally does not depend on `invokrum-distribution-json`. Canonical parsing and subject derivation stay at that outer adapter boundary. The use case first requires `manifest_subject == expected_subject`; subject mismatch fails before candidate content is processed.

An expected digest alone authenticates exact bytes only relative to the trusted channel that supplied that digest. It does not identify a publisher. Publisher identity is established separately by a concrete verifier. Publisher authorization remains a separate host-policy decision after concrete verification and cannot be inferred from exact-byte verification or signature validity alone.

## Linux local-directory source

`invokrum-acquisition-linux::LinuxLocalCandidateSource` is the Linux candidate-filesystem policy. It:

- validates and revalidates one non-symlink root identity;
- bounds traversal depth and visible-entry collection;
- enumerates opened directories through `/proc/self/fd/<fd>`;
- opens declared files with `O_NOFOLLOW` and validates their opened descriptor targets;
- rejects symlinks, hard-linked files, special files, device crossings, root escapes, undeclared entries, unsupported names, and case-folding logical collisions;
- bounds reads to the v1 per-file limit;
- compares stable metadata before/after reads;
- returns only owned `CandidateFile` buffers.

The default `load` operation permits no undeclared filesystem content. A separate `load_with_allowed_file` operation exists only so the installer can verify one explicit versioned metadata file in an already-installed tree; that file remains part of exact-tree inspection and never becomes bundle content.

The host must provide a stable mount namespace and protect the candidate-root parent against privileged replacement when that threat is in scope.

## Bounded POSIX ustar source

`invokrum-acquisition-archive` accepts the documented uncompressed POSIX ustar profile and returns owned `CandidateFile` buffers without writing archive entries to disk. It rejects unsupported or unsafe archive structures including traversal/absolute names, links and special entries, collisions, undeclared or missing files, malformed/trailing forms, unsupported metadata, and resource-limit violations.

Directory and archive candidate sources converge on the same exact verification use case. Archive parsing therefore does not gain authority to reinterpret the canonical manifest or publisher policy.

## Verification order

The in-memory use case fails closed in this order:

1. immutable bundle-subject mismatch;
2. candidate file-count and byte-resource limits;
3. duplicate candidate paths;
4. deterministic path-set comparison for undeclared or missing files;
5. declared byte-length mismatch;
6. SHA-256 content-digest mismatch.

Candidates are sorted by validated path before the path-set merge walk. The manifest is already normalized by `BundleManifest` construction.

## Exact-byte preservation

Successful verification returns `VerifiedBundle`, containing:

- the immutable bundle subject;
- the manifest entry point;
- a deterministic ordered `VerifiedFile` list;
- the exact owned byte buffers that were hashed;
- the corresponding declared SHA-256 digest identity.

The installer consumes these verified bytes directly. It never receives the original candidate path. Reopening the candidate source and then claiming the earlier verification would reintroduce a time-of-check/time-of-use gap.

## Resource bounds

Candidate verification enforces the same hard v1 maxima used by the bundle contract:

- at most 512 files;
- at most 1 MiB per candidate file;
- at most 32 MiB aggregate candidate bytes.

These are hard acquisition-use-case bounds, not caller-relaxable configuration. Directory/archive adapters add their own bounded parsing/traversal controls before these in-memory bounds are evaluated.

## Stable failure categories

`CandidateVerificationError` distinguishes subject, count/size, duplicate/missing/undeclared path, length, and digest mismatches. Source adapters separately describe filesystem/archive-policy failures. Provider-specific Ed25519 failures do not become acquisition-domain error text, and authorization remains a separate host-policy decision.

Validated paths may be included in diagnostics; candidate contents and private signing material are never included.

## Security claims

The exact-content layer can establish:

> These exact owned bytes match this validated bundle manifest whose supplied canonical subject equals the explicitly expected immutable subject.

The supported authenticated workflow can additionally establish:

> A strict `ed25519-subject-v1` verifier validated this exact subject under this normalized key identity, explicit host `TrustPolicy` authorized that assertion, and the Linux installation evidence records the same authorized subject and identity.

Neither claim establishes:

- whether the publisher is still trusted or unrevoked now;
- whether a future remote locator was authentic or fresh;
- whether the bundle is the newest acceptable version;
- whether prompt content is semantically safe;
- whether a runtime is authorized to execute the resulting prompt.

## Current local/archive boundary

The implemented path composes the layers as:

1. `invokrum-acquisition-linux` or `invokrum-acquisition-archive` returns bounded owned candidate bytes;
2. `invokrum-acquisition` checks expected subject, path set, lengths, and SHA-256 digests;
3. digest-only installation may proceed without publisher authentication and records `invokrum.installation/v1` with `publisher_authentication: not-provided`;
4. authenticated installation uses `invokrum-verifier-ed25519` to produce `PublisherAssertion`, applies host-owned `TrustPolicy` to produce `AuthorizedPublisher`, and requires exact subject agreement before installation;
5. `invokrum-install-linux` materializes verified bytes in private quarantine, reverifies them, writes exact v1/v2 installation evidence, and atomically promotes to `sha256/<subject>`;
6. the final installed root enters the existing offline schema/filesystem/composition path.

Remote transport, registry discovery, freshness/revocation/transparency, and rollback selection remain outside the implemented path. The next delivery step is exposing these proven local/archive install workflows through the explicit public API/CLI contract in #64.
