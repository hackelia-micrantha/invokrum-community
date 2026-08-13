# Upstream provenance

This repository is the public community distribution of Invokrum. Shared implementation originates in the separately maintained canonical repository and is promoted here through reviewed, allowlisted snapshots rather than private-history mirroring.

## Initial public baseline

- Source repository: `hackelia-micrantha/invokrum`
- Source commit: `0e6083d0608d22673b7172ce3330328c64be05bc`
- Cutover timestamp (UTC): `2026-08-13T06:30:29Z`
- License at source commit: Apache-2.0

The source commit was publicly accessible before the canonical repository visibility change.

## Accounted baseline differences

The initial community tree intentionally differs from the selected source snapshot only in repository-boundary material:

- community-specific `README.md`, `CONTRIBUTING.md`, and `SECURITY.md` are retained; README/SECURITY cutover status is finalized and both link the imported public threat model required by the public security contract;
- this `UPSTREAM.md` records source provenance and the cutover timestamp;
- `Cargo.toml`, `scripts/release.py`, `docs/release.md`, and `docs/releases/v0.1.0.md` point public source/release metadata at `hackelia-micrantha/invokrum-community`;
- canonical-only `community-export-policy.toml` is omitted;
- the public topology document remains unchanged because references to the canonical repository there describe provenance and promotion direction rather than a public download/source endpoint.

All imported implementation, schema, example, fixture, ordinary documentation, and final workflow bytes are otherwise sourced from the commit above. Future promotions must record their canonical source commit or range without copying private Git history.
