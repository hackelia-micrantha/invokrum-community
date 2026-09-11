# Public binary distribution

Invokrum Community is transitioning from an independently buildable public-source snapshot to the public distribution surface for binaries produced by the private canonical Invokrum build/release authority.

## Target topology

```text
hackelia-micrantha/invokrum (private canonical source/build authority)
  -> immutable reviewed tag
  -> canonical platform archives + checksums + SPDX SBOMs + provenance
  -> byte-for-byte promotion
  -> hackelia-micrantha/invokrum-community (public distribution authority)
       -> release.json
       -> binary-oriented Nix flake
       -> public contracts, schemas, examples, man page, verification docs
```

The public repository must not rebuild a second executable and assign it the same product release identity. Canonical artifact bytes are promoted unchanged.

## Release metadata

A promoted release is described by `release.json` using `invokrum.public-distribution/v1` and the schema at `schemas/invokrum-public-distribution-v1.schema.json`.

The metadata binds one public distribution revision to:

- the canonical repository identity;
- the immutable canonical tag;
- the exact canonical commit;
- the explicitly supported binary target set;
- the archive name and SHA-256 digest for each target.

The v1 target set is intentionally bounded to targets for which canonical binaries are actually produced:

- `x86_64-unknown-linux-gnu`;
- `x86_64-apple-darwin`;
- `x86_64-pc-windows-msvc`.

Earlier source-build evaluation on aarch64 does not imply an aarch64 binary release. New binary targets require a reviewed contract change and real canonical artifacts.

## Nix contract

The binary-oriented flake will fetch immutable public release archives using fixed hashes from reviewed distribution metadata. It must install:

```text
$out/bin/invokrum
$out/share/man/man1/invokrum.1[.gz]
```

and expose stable package/app outputs without requiring access to the private canonical repository or a Rust implementation toolchain.

A clean consumer must be able to evaluate, build, and run the public flake with GitHub/private-source credentials unset.

## Promotion ordering

For a new product release:

1. canonical source/version changes are reviewed and merged;
2. the canonical immutable tag is created;
3. canonical CI builds, checks, attests, and publishes the release artifacts;
4. public `release.json` and the binary flake are reviewed against the exact canonical commit and artifact digests;
5. the exact reviewed public commit is smoke-tested against the canonical artifact before an immutable public tag is created;
6. the public tag is created only after that smoke succeeds;
7. the canonical files are uploaded byte-for-byte to a draft public release;
8. public and canonical copies are compared before publication;
9. the public release is published and anonymously downloadable bytes are compared again.

The public tag must not be burned by an untested package definition.

## Security boundary

Private implementation source is an intellectual-property and attacker-cost boundary, not an authorization or cryptographic boundary. Distributed executables are assumed reverse engineerable.

No secret, signing key, privileged credential, or authorization decision may rely on binary opacity. Checksums, signatures, SBOMs, and provenance are evidence about exact artifacts; they do not grant runtime or governance authority.

## Transition state

The current repository history contains previously public Invokrum implementation source. Removing buildable implementation from the current tree cannot revoke that historical disclosure. The binary-distribution cutover protects future implementation evolution prospectively.

Until the replacement binary flake is independently green, the existing public source-building installation path remains available. Source/build CI and implementation files must not be removed first.

The planned first release under the new topology is `v0.3.0`. Real canonical commit and artifact digests will be added only after that canonical release exists; placeholder release metadata is not accepted.
