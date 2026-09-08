# Invokrum documentation

Invokrum v0.1.0 established deterministic local composition, strict pack parsing, canonical lock verification, a read-only host/RPC contract, an independent subprocess reference host, and gated cross-platform release artifacts.

The current v0.2 line additionally implements canonical `invokrum.pack-bundle/v1` identity, provider-neutral host trust-policy contracts, bounded Linux directory and POSIX ustar candidate acquisition, exact expected-subject/content verification, strict `ed25519-subject-v1` publisher verification, host-owned authorization, Linux quarantine/content-addressed installation, versioned authenticated installation evidence, and the explicit local install API/CLI surface. Remote/network acquisition, registry discovery, freshness/revocation/transparency, rollback-selection policy, and non-Linux secure acquisition/install adapters remain planned.

Documentation distinguishes between **accepted design**, **planned interfaces**, and **implemented behavior** so examples do not imply unsupported functionality. Publisher authentication is implemented for the supported local directory/archive path; it remains an acquisition/install claim and does not become part of ordinary deterministic composition.

## Start here

- [Purpose and scope](purpose.md)
- [Use cases](use-cases.md)
- [Architecture](architecture/README.md)
- [Repository topology](repository-topology.md)
- [Deterministic composition and filesystem contract](composition-and-filesystem.md)
- [Integrity, canonical manifests, and lockfiles](integrity-and-lockfiles.md)
- [Pack bundle format v1](bundle-format-v1.md)
- [Host adapters and subprocess integration](host-adapters.md)
- [Threat model and trust boundaries](security/threat-model.md)
- [Publisher trust and signed pack installation](security/publisher-trust.md)
- [Offline bundle candidate verification](security/offline-candidate-verification.md)
- [Linux local candidate loading and installation](security/linux-local-installation.md)
- [Fuzzing strategy](security/fuzzing.md)
- [V1 schema contract](schema-v1.md)
- [Configuration model](configuration.md)
- [Usage model](usage.md)
- [Governed code-review example](../examples/governed-code-review/README.md)
- [Independent reference host](../examples/reference-host/README.md)
- [Anthesis reference-consumer conformance](../examples/anthesis-conformance/README.md)
- [Visual identity and reusable assets](branding.md)
- [Release and artifact verification](release.md)
- [v0.2.1 release notes](releases/v0.2.1.md)
- [v0.2.0 release notes](releases/v0.2.0.md)
- [Development guide](development.md)
- [Roadmap](roadmap.md)

## Project policies

- [Contributing](../CONTRIBUTING.md)
- [Security policy](../SECURITY.md)
- [Support](../SUPPORT.md)
- [Code of conduct](../CODE_OF_CONDUCT.md)
- [License](../LICENSE)

## Status vocabulary

Documentation uses these terms deliberately:

- **Accepted** — recorded in an accepted architecture decision or repository policy.
- **Planned** — intended for a future milestone but not yet available.
- **Implemented** — present in the repository and covered by executable validation.
- **Experimental** — implemented but not yet compatibility-stable.

Security controls additionally use **Partial**, **Delegated**, and **Out of scope** as defined by the [threat model](security/threat-model.md).

When documentation and implementation diverge, implementation and executable contracts are authoritative; the discrepancy should be reported as documentation drift.
