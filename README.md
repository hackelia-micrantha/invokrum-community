# Invokrum Community

Public distribution and community contract surface for **Invokrum**, a deterministic prompt-overlay composition and attestation engine for governed AI contexts.

## Repository role

The private canonical `hackelia-micrantha/invokrum` repository owns current implementation, product build, and canonical release identity. This public repository owns the community-facing distribution surface: public release artifacts, package metadata, CLI/schema contracts, examples, manual pages, verification documentation, and contribution material that is intentionally public.

The repository began as an independently buildable public implementation snapshot. That history remains public historical disclosure and is not erased by the current migration.

The target release flow is:

```text
private canonical source
  -> reviewed canonical tag/build
  -> immutable archives + checksums + SBOM/provenance
  -> byte-for-byte public promotion
  -> invokrum-community release + binary Nix flake
```

See [Public binary distribution](docs/public-binary-distribution.md) for the distribution contract and migration ordering.

## Current transition state

The current `main` branch still contains the previously public Invokrum implementation and a source-building Nix flake. That path remains temporarily available so the replacement is established before anything is removed.

The planned first product release under the private-canonical/public-binary topology is `v0.3.0`. The binary cutover will not occur until canonical artifacts exist and a clean public consumer can build/run the hash-pinned public flake without private-repository credentials.

After that proof:

- the community flake will consume immutable public release binaries rather than compile the private implementation;
- current buildable implementation source and source-build CI can be removed from the active tree;
- public contracts, schemas, examples, `invokrum(1)`, release metadata, checksums/provenance guidance, and contribution material remain public;
- prior Git history remains acknowledged rather than represented as secret again.

## Release metadata

Promoted binary releases use `release.json` with schema `invokrum.public-distribution/v1`, validated by [`schemas/invokrum-public-distribution-v1.schema.json`](schemas/invokrum-public-distribution-v1.schema.json).

The metadata binds the public package to one exact canonical repository/tag/commit and one explicit set of platform archive SHA-256 digests. Placeholder digests are not accepted.

## Design expectations

Public security claims and residual risks remain defined by the supported public contracts and threat-model documentation. In particular:

- public package evaluation/build must not require private repository state or credentials;
- a public distribution release must reuse the canonical artifact bytes rather than rebuild the same product version independently;
- release/package identity must remain tied to immutable hashes and canonical provenance;
- private implementation is attacker-cost/IP protection, not a security boundary;
- distributed binaries are assumed reverse engineerable;
- no secret, signing key, privileged credential, or authorization decision may depend on binary opacity;
- released schemas, CLI contracts, machine-readable formats, examples, and verification guidance remain inspectable publicly;
- security-sensitive reports use the private reporting path described in `SECURITY.md`.

## License

Invokrum Community is distributed under the Apache License 2.0. See [LICENSE](LICENSE).

The implementation already present in this repository was publicly available under Apache-2.0 before the source-exposure cutover. The forward binary-distribution boundary does not retroactively alter that license or historical disclosure.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Contributions to public contracts, packaging, examples, documentation, and other intentionally public surfaces are welcome. Implementation changes that belong to the private canonical engine require separate canonical review rather than silently establishing this repository as a second product build authority.
