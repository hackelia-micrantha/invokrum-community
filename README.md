# Invokrum Community

Public community distribution of **Invokrum**, a deterministic prompt-overlay composition and attestation engine for governed AI contexts.

## Repository role

This repository is the public distribution and contribution surface for Invokrum.

The canonical development repository is maintained separately. Public implementation changes are promoted here deliberately through reviewed, allowlisted changes rather than by mirroring private repository history.

The initial code baseline will be imported from the final public `hackelia-micrantha/invokrum` commit immediately before that canonical repository changes visibility. The import will record the exact upstream commit and account for repository-relocation-only differences.

## Current status

The initial public implementation baseline is imported from canonical source snapshot `0e6083d0608d22673b7172ce3330328c64be05bc`. This repository now contains the supported public Invokrum source, schemas, examples, compatibility fixtures, documentation, CI, and release tooling needed to build and evaluate the community distribution independently.

## Design expectations

The public security claims and residual risks are defined by the [threat model](docs/security/threat-model.md) and must remain backed by controls and tests available in this repository.

The community distribution must remain independently usable:

- public builds and tests cannot depend on private repository state;
- released schemas, CLI contracts, machine-readable formats, and compatibility fixtures remain testable here;
- package and release metadata must point to publicly accessible sources;
- security-sensitive reports use the private reporting path described in `SECURITY.md`;
- provenance for promoted changes records the canonical source commit or range without exposing private Git history.

## License

Invokrum Community is distributed under the Apache License 2.0. See [LICENSE](LICENSE).

The initial implementation baseline was already publicly released under Apache-2.0 before the repository split; the cutover process preserves that license and attribution continuity.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Shared implementation contributions may be re-ingested into the canonical development repository before a later public promotion so that the public and canonical implementations do not silently diverge.