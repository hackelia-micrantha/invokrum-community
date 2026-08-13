# Contributing to Invokrum Community

Invokrum Community accepts focused issues, documentation improvements, adversarial fixtures, compatibility tests, and implementation contributions to the public distribution.

## Before contributing

1. Search existing issues and pull requests.
2. Open an issue before changing public schemas, persistent formats, security boundaries, compatibility guarantees, or externally observable APIs.
3. Keep product- or organization-specific policy outside the generic engine unless it is explicitly a public example or fixture.
4. Do not include secrets, private deployment configuration, private-repository excerpts, or internal-only security material.

## Repository topology

This repository is the public community distribution. The canonical development repository is maintained separately and may contain work that is not public.

Shared implementation contributions may follow this path:

```text
community PR
    -> public review
    -> canonical re-ingestion when applicable
    -> later reviewed promotion to community
```

This keeps the canonical implementation authoritative without treating private Git history as publishable material.

Community-only metadata or documentation can be maintained directly here when it does not affect shared implementation behavior.

## Pull requests

A pull request should state:

- the problem and intended user-visible outcome;
- compatibility impact;
- security or trust-boundary impact;
- tests and validation performed;
- documentation impact;
- whether the change is community-only or expected to be shared with canonical development.

For externally observable behavior, include tests and stable fixtures where relevant. Public JSON, schemas, exit codes, canonicalization, lockfiles, signatures, installation evidence, or adapter contracts require explicit compatibility review.

## Security reports

Do not open public issues for suspected vulnerabilities. Follow [SECURITY.md](SECURITY.md).

## License

Unless explicitly stated otherwise, contributions submitted for inclusion in this repository are licensed under Apache-2.0.