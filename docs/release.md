# Release and artifact verification

Invokrum releases are gated prereleases built from version tags on the default branch. The release workflow reuses the complete CI workflow before creating any draft release.

## Release inputs

A release tag must:

- use `vMAJOR.MINOR.PATCH` or a SemVer prerelease suffix;
- match `[workspace.package].version` in `Cargo.toml` exactly;
- point to the current `main` commit;
- pass formatting, Clippy, architecture, security, schema, integration, E2E, golden, and portable-build gates.

The workflow never executes pull-request code with release-write or attestation permissions. Tag builds run only from repository-owned refs on GitHub-hosted runners.

Every `gh release` command passes `--repo "$GITHUB_REPOSITORY"` explicitly so release operations remain valid in jobs that intentionally do not check out the repository.

## Create a release tag

The manually dispatched **Create Release Tag** workflow creates the immutable annotated tag and then explicitly dispatches the separate **Release** workflow at that tag. It does not change source versions, release notes, or other repository files.

The explicit dispatch is required because GitHub suppresses ordinary workflow chaining for refs created with the workflow's `GITHUB_TOKEN`. The Release workflow also retains its `push.tags` trigger for tags created outside GitHub Actions.

The workflow supports these version-selection modes:

- `workspace`: use `[workspace.package].version` directly;
- `patch`: increment the patch component of the highest existing stable release tag;
- `minor`: increment minor and reset patch;
- `major`: increment major and reset minor and patch;
- `explicit`: use the supplied SemVer value.

Regardless of mode, the selected version must equal the version already reviewed in `Cargo.toml`. Increment modes are therefore a validation and convenience mechanism, not an unreviewed version-bump mechanism. If the calculated version differs, prepare the version and release notes through a pull request first.

The workflow also:

- requires the exact confirmation text `CREATE TAG`;
- requires the target to resolve to current `origin/main`;
- parses existing stable tags using semantic-version ordering rather than lexical ordering;
- fails when an increment is requested before any stable release tag exists;
- rejects an existing local or remote tag;
- reuses `scripts/release.py validate-tag`;
- creates an annotated tag object and immutable tag reference through the GitHub API;
- never force-moves or replaces an existing tag;
- dispatches the Release workflow using the immutable tag ref.

To create the current workspace release, open **Actions → Create Release Tag → Run workflow**, select `workspace`, leave the target as `main`, and enter `CREATE TAG`. The workflow creates the tag and dispatches **Release** at that exact ref.

## Produced targets

Each tagged prerelease builds one CLI archive for:

- `x86_64-unknown-linux-gnu` on Ubuntu 24.04;
- `x86_64-apple-darwin` on an Intel macOS runner;
- `x86_64-pc-windows-msvc` on Windows Server 2025.

The local source and persistent-output adapters remain Linux-only. The macOS and Windows artifacts provide parsing, validation, inspection, lock, diff, and stdout workflows; filesystem-backed composition fails closed until native adapters exist.

## Artifact contents

Each archive contains:

- the `invokrum` or `invokrum.exe` binary;
- `README.md`;
- `LICENSE`;
- `SBOM.spdx.json`.

Each release also publishes:

- a standalone target-specific SPDX 2.3 JSON SBOM;
- a SHA-256 checksum file for the archive;
- a GitHub artifact provenance attestation;
- a GitHub SPDX SBOM attestation.

The release remains a draft if any target build, packaging step, attestation, or upload fails. It is published as a prerelease only after every target succeeds.

## Determinism boundary

`scripts/release.py` produces deterministic archive metadata, file ordering, permissions, timestamps, gzip headers, ZIP entries, SPDX ordering, and checksum formatting from fixed inputs. Unit tests build identical tar and ZIP packages twice and require byte equality.

The Rust toolchain and dependency graph are pinned, and the source timestamp comes from the tagged commit. GitHub-hosted operating-system images and native linkers are managed externally, so Invokrum does not yet claim independently reproducible native binary bytes across different runner-image revisions. The archive guarantee is:

> identical binary bytes, source files, SBOM, target, version, and source timestamp produce identical archive and checksum bytes.

A stronger cross-builder reproducible-binary claim requires hermetic toolchains or independently repeated builders and is not currently made.

## Verify a download

Verify the published checksum from the directory containing both files:

```bash
sha256sum --check invokrum-v0.2.0-x86_64-unknown-linux-gnu.tar.gz.sha256
```

On macOS:

```bash
shasum -a 256 --check invokrum-v0.2.0-x86_64-apple-darwin.tar.gz.sha256
```

Verify GitHub provenance and SBOM attestations against this repository:

```bash
gh attestation verify \
  invokrum-v0.2.0-x86_64-unknown-linux-gnu.tar.gz \
  --repo hackelia-micrantha/invokrum-community
```

Artifact attestations use short-lived keyless signing credentials issued during the trusted workflow. There is no long-lived project signing key to distribute or rotate. Attestations establish workflow provenance for exact artifact digests; they do not authenticate overlay packs or make prompt content semantically safe.

## Release procedure

1. Update the workspace version and release notes in a reviewed pull request.
2. Ensure the default branch is green.
3. Dispatch **Create Release Tag** using `workspace` or an increment/explicit mode that resolves to the reviewed workspace version.
4. The tag workflow creates the immutable tag and explicitly dispatches **Release** at that tag.
5. The Release workflow reruns all required delivery gates.
6. Review the draft release, checksums, SBOMs, and attestation links.
7. The workflow publishes the prerelease after all matrix jobs complete.
8. Verify at least one asset using both its checksum and `gh attestation verify`.

## Recover an existing draft release

Rerunning a failed tagged workflow uses the workflow snapshot stored at that immutable tag. A later workflow fix on `main` is therefore not applied to that rerun.

When the build and upload jobs succeeded but publication failed, use **Publish Existing Draft Release** from `main`. Supply the immutable tag and type `PUBLISH RELEASE`. The recovery workflow verifies that the tag exists, the matching release is still a draft, and the complete expected archive, checksum, and SPDX SBOM asset set is present for Linux, macOS, and Windows before publishing it as a prerelease. It never moves the tag or rebuilds artifacts.

A failed run may leave a draft release. Correct the cause and rerun from the same immutable tag only when the source commit remains appropriate. Do not move a published version tag; create a new version for changed source or artifacts.

## Dependency and scanner exceptions

Security gates fail closed by default.

An exception must be introduced through a reviewed pull request and include:

- the advisory, license, source, or secret-scanner finding identifier;
- a linked public issue or private security advisory;
- affected package or exact scanner fingerprint;
- exploitability or false-positive analysis;
- compensating controls;
- an owner;
- a review or expiry condition.

Rust advisory ignores belong in `deny.toml` with an adjacent issue reference. License exceptions must be package-specific rather than broad new allow-list entries when possible. Secret-scan exceptions must use an exact fingerprint or narrow path/rule combination; broad directory exclusions are not acceptable. Workflow action revisions remain full immutable commit SHAs.

The CODEOWNERS file assigns the repository maintainer to workflows, dependency policy, security documentation, and release tooling. Security reports follow [`SECURITY.md`](../SECURITY.md).
