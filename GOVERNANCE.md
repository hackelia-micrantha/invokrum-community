# Project governance

Invokrum is currently maintained by the repository owners under a lightweight maintainer model appropriate for an early-stage project.

## Decision authority

Maintainers are responsible for:

- accepting or rejecting changes;
- defining release scope;
- maintaining compatibility and security policy;
- resolving disputes about project boundaries;
- coordinating vulnerability response;
- updating this governance model as the contributor base grows.

## Decision process

Routine implementation and documentation decisions are made through pull-request review.

An architecture decision record is expected when a change materially affects:

- mechanism-versus-policy boundaries;
- public schemas or persistent formats;
- canonicalization and deterministic behavior;
- trust boundaries or network behavior;
- plugin execution;
- compatibility policy;
- public library or adapter contracts.

Decisions should optimize for correctness, auditability, and maintainability rather than consensus for its own sake. Material dissent and rejected alternatives should be recorded when they improve future understanding.

## Compatibility and security changes

Changes that weaken validation, alter path handling, expose sensitive values, add implicit network access, or modify an attestation boundary require explicit maintainer review.

Public schema, lockfile, manifest, JSON-output, exit-code, and API changes must identify their compatibility impact.

## Releases

Maintainers approve releases after required validation passes and release artifacts are reproducible under the documented process. Release automation and support policy are tracked separately from core behavior.

## Becoming a maintainer

There is no fixed maintainer nomination process while the project is small. Sustained, technically sound contributions; constructive review; security awareness; and demonstrated stewardship may lead to expanded repository responsibility.

## Conflicts of interest

Maintainers should disclose material conflicts that could affect a decision and, where practical, defer review or enforcement to another maintainer.