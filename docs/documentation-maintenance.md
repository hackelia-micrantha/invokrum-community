# Documentation maintenance contract

**Adoption:** Micrantha shared `documentation-sync` default, unchanged.

Apply the shared [documentation-sync skill](https://github.com/hackelia-micrantha/.github/blob/main/skills/documentation-sync/SKILL.md) and the [public-claims consistency review](https://github.com/hackelia-micrantha/.github/blob/main/docs/prompts/documentation/public-consistency-review.md). This file records Invokrum Community-specific authority, verification, and transition facts; it does not copy or replace the shared skill.

## Authority and public boundary

- The private canonical `hackelia-micrantha/invokrum` repository owns implementation behavior, product build, release identity, and the authoritative qualification of those claims.
- This public repository owns intentionally public distribution metadata, published release assets, CLI/schema/manual and example contracts, compatibility and verification documentation, and community contribution material.
- The historical, buildable public source and source-oriented Nix flake are **transitional**, not an independent authority to build a second product under a canonical release identity. Historical publication cannot be made private retroactively.
- The planned binary-only distribution transition is tracked by [public distribution issue #12](https://github.com/hackelia-micrantha/invokrum-community/issues/12) and the [public binary distribution contract](public-binary-distribution.md). Do not claim this transition complete based only on a proposed artifact, metadata, or passing source CI.

Public documentation must not include new private implementation source, internal-only security details, credentials, private release inputs, or operational logs. Publicly describing a repository's role does not authorize copying content from that repository.

## Public surfaces and claim checks

Review the root [README](../README.md), [documentation index](README.md), [distribution contract](public-binary-distribution.md), [Nix instructions](nix.md), [threat model](security/threat-model.md), public schemas, examples, release/verification guidance, and any actually published `invokrum(1)` manual artifact together when a change affects their claims. The proposed manual page is **not present** in this public repository's current tree; do not describe it as a currently validated public file.

For every supported capability, distinguish **implemented in canonical source**, **validated for a supported platform**, **published as a public release**, and **planned**. In particular:

- Linux-specific installation and filesystem behavior must not be implied by macOS or Windows build availability.
- Content digests prove selected bytes, not a publisher identity; publisher authentication is separate from host authorization and prompt semantic safety.
- Public v0.2.x source-oriented packages and the proposed v0.3 binary cutover have different distribution paths. An untested binary flake, planned target, or unpromoted release is not a downloadable/supported public product.
- Public release and install claims require an exact tag, publicly retrievable artifacts, integrity/provenance evidence, and a clean credential-free consumer test. Preserve release-source identity and never treat publishing as a side effect of a documentation PR.

If the canonical implementation changes without a public distribution update, qualify the public wording; do not silently strengthen it or copy private files into the public repository.

## Validation entry points

Use the narrowest applicable validation on the exact candidate revision. The existing source-build path retains its checks until the binary cutover is proven:

```bash
# Repository-local Markdown paths (no network fetch of external links)
python3 scripts/check_markdown_links.py

# Public documentation/policy validation exercised by CI
python3 -m unittest discover -s tests -p 'test_*.py'

# Currently published public source-oriented Nix surface
nix flake check --show-trace
nix build --no-link .#packages.x86_64-linux.invokrum
nix run .#invokrum -- --version
```

The [CI workflow](../.github/workflows/ci.yml) includes Markdown link and contract checks; the [Nix workflow](../.github/workflows/nix.yml) checks the public flake. Record their exact-head results separately; source CI does **not** establish artifact publication, private-canonical equivalence, or the future binary flake's security properties.

For a promoted binary release, additionally follow the [distribution contract](public-binary-distribution.md): compare canonical and public checksums, manifests, metadata, SBOM/provenance, immutable tags and final anonymous downloads; verify the pinned flake from a clean public consumer without private credentials. These are gates to execute when applicable, not checks claimed complete by this document.

## Maintenance triggers

Review public claims after canonical CLI/schema/security boundary changes, a supported-platform change, a release/tag or installation-route change, the transition in #12, or a meaningful community packaging update. Prefer event-driven checks and small project-owned fixes to parallel documentation forks.

A passing docs PR does not authorize canonical release, tag creation, publication, private-source export, or changing distribution authority. Ambiguities affecting public release, security, licensing, or support claims remain explicit human decisions under the shared skill. No further local specialization is required for the current contract.
