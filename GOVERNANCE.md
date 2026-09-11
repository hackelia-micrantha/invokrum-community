# Project governance

Invokrum Community is currently maintained by the repository owners under a lightweight maintainer model appropriate for an early-stage distribution/community project.

## Authority split

`hackelia-micrantha/invokrum` is the canonical authority for current engine implementation, product builds, and canonical product release identity.

`hackelia-micrantha/invokrum-community` is the public authority for intentionally public distribution surfaces, including package definitions, release metadata, schemas, examples, manual pages, verification documentation, compatibility/conformance material, and community contribution policy.

The public repository must not create a second independently built executable under the same canonical product release identity. Public product binaries are promoted byte-for-byte from an authorized canonical release and bound by immutable hashes.

## Decision authority

Community maintainers are responsible for:

- accepting or rejecting changes to public distribution/community surfaces;
- maintaining public compatibility and security contracts;
- reviewing package/release metadata and public promotion evidence;
- resolving disputes about the public/private repository boundary;
- coordinating vulnerability response;
- updating this governance model as the contributor base grows.

Canonical implementation and build decisions remain with the canonical repository maintainers.

## Decision process

Routine public documentation, packaging, schema, fixture, and community-tooling decisions are made through pull-request review.

An architecture decision record or equivalent explicit design review is expected when a public change materially affects:

- mechanism-versus-policy boundaries;
- public schemas or persistent formats;
- canonicalization and deterministic behavior;
- trust boundaries or network behavior;
- plugin execution;
- compatibility policy;
- public package/release identity;
- public library or adapter contracts;
- the source-exposure/distribution boundary.

Decisions should optimize for correctness, auditability, and maintainability rather than consensus for its own sake. Material dissent and rejected alternatives should be recorded when they improve future understanding.

## Compatibility and security changes

Changes that weaken validation, alter path handling, expose sensitive values, add implicit network access, modify an attestation boundary, or broaden public implementation exposure require explicit maintainer review.

Public schema, lockfile, manifest, JSON-output, exit-code, API, release-metadata, and package changes must identify their compatibility impact.

Private implementation source is attacker-cost/IP protection, not a security boundary. Distributed executables are assumed reverse engineerable, and no secret, signing key, privileged credential, or authorization decision may depend on binary opacity.

## Releases

Canonical product releases are built and attested by `hackelia-micrantha/invokrum`. Community maintainers approve public distribution only after the canonical release exists and the reviewed public metadata/package definition identifies the same immutable canonical release and artifact digests.

The public release process promotes exact canonical bytes; it does not rebuild the same product version independently. Public package validation, byte-comparison, anonymous download checks, and clean-consumer qualification are part of distribution acceptance.

Repository-only documentation or community metadata releases may be managed independently when they do not claim a canonical Invokrum product version or replace product artifact identity.

## Becoming a maintainer

There is no fixed maintainer nomination process while the project is small. Sustained, technically sound contributions; constructive review; security awareness; and demonstrated stewardship may lead to expanded repository responsibility.

## Conflicts of interest

Maintainers should disclose material conflicts that could affect a decision and, where practical, defer review or enforcement to another maintainer.
