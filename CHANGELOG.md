# Changelog

All notable user-visible changes to Invokrum will be documented in this file.

The format is based on Keep a Changelog principles, and versioned releases will follow Semantic Versioning once a stable public contract exists.

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
