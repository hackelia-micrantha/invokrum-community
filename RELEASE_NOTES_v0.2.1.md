# Invokrum v0.2.1

Invokrum `v0.2.1` adds a credential-free public Nix distribution surface to the community release boundary.

## Highlights

- Public `packages`, `apps`, `checks`, `devShells`, and formatter outputs.
- The packaged CLI is built only from public community source.
- Deterministic smoke coverage exercises validate, compose, lock, and verify with credentials and `SSH_AUTH_SOCK` absent.
- Downstream hosts can use a reviewed release tag while retaining exact commit and NAR identity in their Nix lockfile.

## Trust boundary

The release makes distribution distinct from authority:

1. the community release identifies reviewed public source and artifacts;
2. a downstream lockfile binds that release to exact immutable source identity;
3. downstream adoption is a separate reviewed decision;
4. installation, policy, promotion, and runtime activation remain separate authorities.

No private canonical-repository credential, SSH agent, provider credential, signer key, remote pack, or host mutation capability is required for normal Nix evaluation/build.

## Compatibility

This patch release does not intentionally change Invokrum schema, lock, bundle, installation-evidence, CLI machine-output, host RPC, or Rust API contracts. Existing `v0.2.0` behavior remains supported; `v0.2.1` adds the public Nix packaging and validation surface.

## Release artifacts

The existing community release pipeline publishes prerelease archives, SHA-256 checksums, SPDX SBOMs, and GitHub attestations for supported release targets. The Nix flake is part of the tagged public source and is consumed through the immutable tag/lock identity rather than through private canonical source.
