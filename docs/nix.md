# Nix package surface

Invokrum Community exposes the released public CLI as a credential-free Nix flake. The flake packages only source contained in this repository and does not require access to the private canonical repository, provider credentials, signer keys, remote packs, or host mutation.

## Exported surface

```text
packages.<system>.default
packages.<system>.invokrum
apps.<system>.default
apps.<system>.invokrum
checks.<system>.default
checks.<system>.package
checks.<system>.format
checks.<system>.lint
checks.<system>.test
checks.<system>.smoke
devShells.<system>.default
formatter.<system>
```

The package version is read from the workspace `Cargo.toml`. Rust is pinned by `rust-toolchain.toml`; Nix dependencies are pinned by `flake.lock`; Cargo dependencies are pinned by `Cargo.lock`.

Supported Nix systems are `x86_64-linux`, `aarch64-linux`, `x86_64-darwin`, and `aarch64-darwin`.

## Validation

```sh
nix flake check --show-trace
nix build --no-link .#packages.x86_64-linux.invokrum
nix run .#invokrum -- --version
```

The `smoke` check exercises the packaged CLI with no credentials in its environment. It validates a bounded local pack, composes it twice and requires byte-identical output, creates two deterministic locks, and verifies the resulting lock against the same pack/profile.

## Downstream composition

Downstream consumers should pin an exact reviewed revision of this repository in their own flake lock and consume `packages.${system}.invokrum`. Installing the CLI does not grant acquisition, installation, signing, deployment, or other runtime authority.
