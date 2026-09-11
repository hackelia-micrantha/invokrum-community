# Contributing to Invokrum Community

Invokrum Community accepts focused issues and pull requests for intentionally public surfaces: documentation, schemas, examples, compatibility/conformance fixtures, packaging, release verification, public metadata, and community tooling.

## Before contributing

1. Search existing issues and pull requests.
2. Open an issue before changing public schemas, persistent formats, security boundaries, compatibility guarantees, package/release metadata, or externally observable APIs.
3. Keep product- or organization-specific policy outside the generic public contracts unless it is explicitly a public example or fixture.
4. Do not include secrets, private deployment configuration, private-repository excerpts, private implementation details, or internal-only security material.

## Repository topology

This repository is the public community distribution and contract surface. The canonical `hackelia-micrantha/invokrum` repository separately owns current engine implementation, product builds, and canonical release identity.

The repository history contains implementation source that was already public before the source-exposure cutover. During the transition that source remains buildable only until the replacement public binary path is proven. Its presence does not make this repository a second product build authority.

The forward contribution path is:

```text
public contract/package/docs contribution
    -> community review
    -> merge in invokrum-community when it belongs to the public surface

engine implementation issue or behavior proposal
    -> public issue/discussion when safe to disclose
    -> canonical implementation review by maintainers
    -> later public contract/artifact promotion where applicable
```

Do not copy private canonical implementation into a public pull request. If a behavior change requires engine work, describe the externally observable requirement and compatibility/security impact without publishing private source or internal-only defensive material.

## Community UI

If this repository introduces or materially redesigns user-facing UI, it follows the shared Phyllotaxis community directive by default: **1990s in visual character, not in capability.** Prefer plain, direct, content-first interfaces with obvious browser-native affordances and minimal decorative chrome, while retaining modern accessibility, semantics, responsive behavior, and security.

See the organization-wide [community UI design directive](https://github.com/hackelia-micrantha/.github/blob/main/docs/standards/ui-design.md). Repository-specific deviations should be justified by a concrete product, usability, or accessibility requirement.

## Pull requests

A pull request should state:

- the problem and intended user-visible outcome;
- compatibility impact;
- security or trust-boundary impact;
- tests and validation performed;
- documentation impact;
- whether the change is distribution/community-only or requires a corresponding canonical implementation change.

For externally observable behavior, include tests and stable fixtures where relevant. Public JSON, schemas, exit codes, canonicalization, lockfiles, signatures, installation evidence, release metadata, package definitions, or adapter contracts require explicit compatibility review.

A public package/release change must not rebuild an independently modified binary under an existing canonical product version. Product artifacts are promoted from the canonical release authority and identified by immutable digests.

## Security reports

Do not open public issues for suspected vulnerabilities. Follow [SECURITY.md](SECURITY.md).

## License

Unless explicitly stated otherwise, contributions submitted for inclusion in this repository are licensed under Apache-2.0. Historical public implementation remains under its existing license; the forward private-canonical boundary does not retroactively change that history.
