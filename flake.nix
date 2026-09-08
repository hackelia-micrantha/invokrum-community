{
  description = "Invokrum deterministic prompt composition and attestation";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
    }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
      workspaceManifest = builtins.fromTOML (builtins.readFile ./Cargo.toml);
      version = workspaceManifest.workspace.package.version;
      mkPkgs = system: import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
      };
      mkRustPlatform = pkgs:
        let
          rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        in
        pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = mkPkgs system;
          rustPlatform = mkRustPlatform pkgs;
          invokrum = rustPlatform.buildRustPackage {
            pname = "invokrum";
            inherit version;
            src = self;
            cargoLock.lockFile = ./Cargo.lock;
            buildAndTestSubdir = "crates/invokrum-cli";
            doCheck = false;

            meta = {
              description = "Deterministic prompt-overlay composition and attestation";
              homepage = "https://github.com/hackelia-micrantha/invokrum-community";
              license = pkgs.lib.licenses.asl20;
              mainProgram = "invokrum";
              platforms = systems;
            };
          };
        in
        {
          default = invokrum;
          inherit invokrum;
        }
      );

      apps = forAllSystems (
        system:
        let
          invokrum = self.packages.${system}.invokrum;
        in
        {
          default = {
            type = "app";
            program = "${invokrum}/bin/invokrum";
            meta.description = "Run the Invokrum CLI";
          };
          invokrum = {
            type = "app";
            program = "${invokrum}/bin/invokrum";
            meta.description = "Run the Invokrum CLI";
          };
        }
      );

      checks = forAllSystems (
        system:
        let
          pkgs = mkPkgs system;
          rustPlatform = mkRustPlatform pkgs;
          common = {
            inherit version;
            src = self;
            cargoLock.lockFile = ./Cargo.lock;
            buildPhase = "true";
            doCheck = true;
            installPhase = ''
              mkdir -p "$out"
              printf 'ok\n' > "$out/result"
            '';
          };
          mkCargoCheck =
            name: command:
            rustPlatform.buildRustPackage (
              common
              // {
                pname = "invokrum-${name}-check";
                checkPhase = ''
                  runHook preCheck
                  export HOME="$TMPDIR/home"
                  mkdir -p "$HOME"
                  ${command}
                  runHook postCheck
                '';
              }
            );
          format = mkCargoCheck "format" "cargo fmt --all --check";
          lint = mkCargoCheck "lint" "cargo clippy --workspace --all-targets --all-features --locked -- -D warnings";
          test = mkCargoCheck "test" "cargo test --workspace --all-features --locked";
          smoke =
            let
              invokrum = self.packages.${system}.invokrum;
            in
            pkgs.runCommand "invokrum-smoke"
              {
                nativeBuildInputs = [ invokrum ];
              }
              ''
                set -euo pipefail
                export HOME="$TMPDIR/home"
                unset GH_TOKEN GITHUB_TOKEN SSH_AUTH_SOCK
                umask 077

                invokrum --help > "$TMPDIR/help.txt"
                invokrum --version > "$TMPDIR/version.txt"
                grep -q '^invokrum ' "$TMPDIR/version.txt"

                fixture="$TMPDIR/fixture"
                mkdir -p "$HOME" "$fixture/overlays"
                cat > "$fixture/pack.yaml" <<'YAML'
                schema: invokrum.dev/v1
                id: nix-smoke
                classes:
                  - id: core
                    order: 10
                    minimum: 1
                    maximum: 1
                  - id: mode
                    order: 20
                    minimum: 1
                    maximum: 1
                overlays:
                  - id: core-invariant
                    class: core
                    source: overlays/core.md
                  - id: review
                    class: mode
                    source: overlays/review.md
                profiles:
                  - id: smoke
                    selections:
                      core:
                        - core-invariant
                      mode:
                        - review
                variables: []
                YAML
                printf '%s' 'Invokrum core invariant.' > "$fixture/overlays/core.md"
                printf '%s' 'Read-only review mode.' > "$fixture/overlays/review.md"

                invokrum validate --pack "$fixture/pack.yaml" --profile smoke >/dev/null
                invokrum compose --pack "$fixture/pack.yaml" --profile smoke > "$fixture/context-1.md"
                invokrum compose --pack "$fixture/pack.yaml" --profile smoke > "$fixture/context-2.md"
                cmp "$fixture/context-1.md" "$fixture/context-2.md"
                printf '%s\n\n%s' 'Invokrum core invariant.' 'Read-only review mode.' > "$fixture/expected.md"
                cmp "$fixture/context-1.md" "$fixture/expected.md"

                invokrum lock --pack "$fixture/pack.yaml" --profile smoke > "$fixture/lock-1.json"
                invokrum lock --pack "$fixture/pack.yaml" --profile smoke > "$fixture/lock-2.json"
                cmp "$fixture/lock-1.json" "$fixture/lock-2.json"
                invokrum verify \
                  --lock "$fixture/lock-1.json" \
                  --pack "$fixture/pack.yaml" \
                  --profile smoke \
                  >/dev/null

                mkdir -p "$out"
                cp "$TMPDIR/help.txt" "$out/help.txt"
                cp "$TMPDIR/version.txt" "$out/version.txt"
                cp "$fixture/context-1.md" "$out/context.md"
                cp "$fixture/lock-1.json" "$out/invokrum.lock"
              '';
        in
        {
          inherit
            format
            lint
            smoke
            test
            ;
          package = self.packages.${system}.invokrum;
          default = pkgs.linkFarm "invokrum-checks" [
            {
              name = "package";
              path = self.packages.${system}.invokrum;
            }
            {
              name = "format";
              path = format;
            }
            {
              name = "lint";
              path = lint;
            }
            {
              name = "test";
              path = test;
            }
            {
              name = "smoke";
              path = smoke;
            }
          ];
        }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = mkPkgs system;
          rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        in
        {
          default = pkgs.mkShell {
            packages = [
              rustToolchain
              pkgs.coreutils
              pkgs.git
              pkgs.python3
            ];
          };
        }
      );

      formatter = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        pkgs.nixfmt
      );
    };
}
