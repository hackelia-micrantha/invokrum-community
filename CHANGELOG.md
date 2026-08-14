# Changelog

All notable user-visible changes to Invokrum will be documented in this file.

The format is based on Keep a Changelog principles, and versioned releases will follow Semantic Versioning once a stable public contract exists.

## 0.2.0 — prerelease

### Added

- Canonical `invokrum.pack-bundle/v1` bundle identity and provider-neutral distribution contracts.
- Bounded local Linux directory acquisition and bounded uncompressed POSIX ustar acquisition without filesystem extraction.
- Exact immutable-subject candidate verification before installation.
- Strict `ed25519-subject-v1` publisher-signature verification with normalized key fingerprints.
- Explicit host-owned publisher authorization kept separate from cryptographic verification.
- Linux content-addressed installation through private quarantine with deterministic unsigned `invokrum.installation/v1` and authenticated `invokrum.installation/v2` evidence.
- Operator-facing local directory/archive install workflows for digest-only and authenticated Ed25519 claims, including Linux XDG store-root resolution.
- Independent subprocess reference-host coverage for capability negotiation, exact-byte persistence, verification, and deterministic drift blocking.
- Private-canonical/public-community repository topology with allowlisted, provenance-recorded promotion instead of Git-history mirroring.

### Security

- Bundle identity, publisher signature, host authorization, installation evidence, and composition lock evidence use distinct claim domains and cannot substitute for one another.
- Candidate metadata cannot choose or relax host trust policy.
- Authenticated installation requires verifier success, host-policy authorization, and exact subject agreement before persistent store mutation.
- Local directory/archive inputs are bounded and fail closed on traversal, links, special entries, collisions, undeclared files, malformed archives, and resource-limit violations.
- Existing content-addressed roots reject provenance upgrade, downgrade, signer substitution, and mechanism substitution in place.
- Installation staging is private and reverified before atomic content-addressed promotion.

### Distribution

- `hackelia-micrantha/invokrum-community` is the public source, contribution, and release surface.
- `v0.2.0` is the first version intended to be published from the community repository.
- The historical canonical `v0.1.0` release remains bound to its original source commit; the later community baseline is not relabeled as `v0.1.0`.

### Limitations

- Secure acquisition and installation adapters currently support Linux only.
- Remote transport, registry discovery, freshness, expiry/revocation, transparency, and rollback/freeze selection semantics are not implemented.
- Publisher authentication establishes provenance for an immutable subject; it does not establish prompt semantic safety or runtime authorization.
- Production publisher private-key custody/signing workflows remain external follow-up work.

## 0.1.0 — prerelease

### Added

- Typed overlay-pack, class, overlay, profile, variable, and compatibility domain model.
- Strict bounded `invokrum.dev/v1` YAML and JSON schema adapter with duplicate-key and unsupported-YAML rejection.
- Deterministic composition with fail-closed cardinality, compatibility, path, source-byte, and output limits.
- Linux filesystem adapter with canonical-root pinning, symlink and hard-link rejection, descriptor containment, identity checks, and mutation detection.
- Canonical `invokrum.lock/v1` evidence, SHA-256 digests, deterministic drift verification, and structural lock diffing.
- Operator CLI for validate, compose, inspect, lock, verify, diff, and read-only JSON RPC workflows.
- Transport-neutral host façade and versioned `invokrum.host/v1` request/response contract.
- Governed code-review example with exact context, manifest, and lock golden artifacts.
- Layered unit, integration, E2E, golden, architecture, security, and portability test gates.
- Deterministic cross-platform release packaging with checksums, SPDX SBOMs, and GitHub attestations.
- Manually dispatched validated release-tag workflow with workspace, patch, minor, major, and explicit selection modes.
- Initial project purpose, architecture, use-case, configuration, usage, development, security, support, contribution, and roadmap documentation.
- ADR-0001 defining the mechanism-versus-policy boundary.

### Security

- Untrusted schema documents are bounded by bytes, nesting depth, and declaration counts before domain aggregate construction.
- Human diagnostics visibly encode attacker-controlled control characters.
- Persistent Linux output uses private permissions, explicit replacement, same-directory staging, link rejection, identity checks, atomic commit, and failure cleanup.
- CI uses immutable action revisions, dependency/license/source policy, full-history secret scanning, least-privilege release permissions, and provenance/SBOM attestations.

### Limitations

- Secure filesystem-backed composition and persistent output support Linux only.
- Structural validation and exact-byte integrity do not establish prompt semantic safety, authorization, runtime isolation, or third-party pack-publisher identity.
- Native binary reproducibility across different GitHub runner-image revisions is not claimed.
- Anthesis is not required for this release; future Anthesis conformance fixtures are optional reference-consumer validation.

## Versioning notes

Before `1.0.0`, minor releases may include breaking changes to experimental interfaces. Breaking changes must still be explicit in this changelog and in release notes.

Compatibility-sensitive surfaces include:

- overlay-pack schemas;
- canonicalization and rendering rules;
- resolved manifest and lockfile formats;
- machine-readable CLI output;
- exit codes;
- public Rust APIs;
- adapter request and response envelopes.

Documentation-only corrections that do not alter a public contract may be grouped under the next release.
