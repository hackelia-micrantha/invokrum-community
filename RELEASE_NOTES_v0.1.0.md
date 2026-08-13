# Invokrum v0.1.0

Invokrum v0.1.0 is the first standalone prerelease of the deterministic prompt-overlay composition and attestation engine.

## Highlights

- Strict, bounded `invokrum.dev/v1` YAML and JSON pack schema.
- Deterministic profile composition with explicit ordering, cardinality, and compatibility rules.
- Fail-closed Linux filesystem adapter with path, link, identity, mutation, and resource controls.
- Canonical `invokrum.lock/v1` evidence, SHA-256 digests, drift verification, and structural diffing.
- Operator CLI with `validate`, `compose`, `inspect`, `lock`, `verify`, `diff`, and read-only `rpc` workflows.
- Transport-neutral `invokrum-host` façade and versioned `invokrum.host/v1` subprocess contract.
- Layered unit, integration, E2E, golden, architecture, security, and portability checks.
- Deterministic release archives with checksums, SPDX SBOMs, and GitHub provenance/SBOM attestations.
- A manually dispatched, validated tag-creation workflow supporting workspace, patch, minor, major, and explicit version selection without mutating reviewed source versions.

## Security boundary

Invokrum validates structure, composition, exact input bytes, and evidence consistency. It does not determine whether prompt content is semantically safe, authorize agent actions, authenticate third-party pack publishers, or sandbox host execution.

Secure filesystem-backed source composition and persistent output currently support Linux only and require a stable mount namespace and protected output parent. macOS and Windows binaries support parsing, validation, inspection, lock handling, diffing, RPC framing, and stdout-oriented workflows; filesystem-backed composition fails closed until platform-native adapters exist.

Artifact attestations establish provenance for exact release artifacts produced by the trusted GitHub workflow. They do not authenticate overlay packs.

## Relationship to Anthesis

Invokrum originated from a generic composition mechanism explored in Anthesis, but this release is independent. Anthesis is not required to install, run, verify, or integrate Invokrum v0.1.0. Any future Anthesis fixtures are optional reference-consumer conformance evidence and are not part of the v0.1 release criteria.

## Verification

Verify downloaded archives with their target-specific SHA-256 files and GitHub attestations. See [`docs/release.md`](docs/release.md) for commands, artifact contents, reproducibility boundaries, and the release procedure.
