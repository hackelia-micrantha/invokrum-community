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

### 2026-08-14 — v0.2.0 release metadata and compatibility versioning

- Canonical source repository: `hackelia-micrantha/invokrum`
- Canonical source commit: `69239c9b955d0e65ba2d3e81008595a083864ae7`
- Canonical source review: PR #91
- Community base before promotion: `d916a402d5342586818bc4e8c41c28f11bfc6b4f`
- Historical release identity preserved: canonical `v0.1.0` remains bound to source commit `23a3ecc8de1f77333e12e8f21a986cfb0dff108f`; the later public baseline is versioned as `v0.2.0` rather than relabeled.

Promoted or source-derived paths:

- `Cargo.toml` — workspace version promoted to `0.2.0`; the community-owned `repository` URL remains `hackelia-micrantha/invokrum-community`.
- `Cargo.lock` — exact reviewed canonical `v0.2.0` lock bytes; only Invokrum workspace package versions differ from the previous community lock.
- `crates/invokrum-schema/Cargo.toml`
- `crates/invokrum-fs/Cargo.toml`
- `crates/invokrum-cli/Cargo.toml`
- `crates/invokrum-host/Cargo.toml`
- `crates/invokrum-integrity/Cargo.toml`
- `crates/invokrum-acquisition/Cargo.toml`
- `crates/invokrum-distribution-json/Cargo.toml`
- `crates/invokrum-acquisition-linux/Cargo.toml`
- `crates/invokrum-acquisition-archive/Cargo.toml`
- `crates/invokrum-install/Cargo.toml`
- `crates/invokrum-verifier-ed25519/Cargo.toml`
- `crates/invokrum-install-linux/Cargo.toml`
- `crates/invokrum-install-delivery/Cargo.toml`
- `CHANGELOG.md`
- `RELEASE_NOTES_v0.2.0.md`
- `docs/README.md`
- `docs/releases/v0.2.0.md`
- `docs/release.md` — community-owned release verification examples updated from historical `v0.1.0` to the reviewed `v0.2.0` release while retaining community repository attestation identity.

Canonical-only `community-export-policy.toml`, post-cutover `fuzz/**` material, canonical Git refs/history, and unrelated private-default paths are not part of this promotion. The community repository remains independently buildable and releaseable from its own history and repository-owned release workflows.
