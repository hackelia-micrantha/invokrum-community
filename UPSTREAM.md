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

## Promotion history

### 2026-08-13 — Distribution-contract coverage for install delivery

- Canonical source repository: `hackelia-micrantha/invokrum`
- Canonical source base: `3cf2f369798f3aa0079f0fcc36ab44e138a4f4a6`
- Canonical source commit: `0858ede1ef02d69c40a0a4e6cbd887dcb136106b`
- Canonical source review: PR #84
- Promoted source paths:
  - `.github/workflows/distribution.yml`
  - `tests/test_distribution_workflow_contract.py`
- Target-only provenance path: `UPSTREAM.md`

Before publication, this promotion was dry-run against community `main` at `6ad70bdba482eb72660c899b3e219733c9a61adc`. The dry run selected these exact paths, classified the workflow as an explicit mixed-governance export, rejected unknown-path broadening, produced only the expected source-file diff, and re-read community `main` unchanged afterward. No private Git history is copied by this promotion.
