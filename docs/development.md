# Development guide

## Prerequisites

The repository pins Rust through `rust-toolchain.toml`. A standard Rustup installation should select the expected toolchain automatically.

Recommended local tools:

- Rustup and Cargo;
- Git;
- `cargo fmt`;
- `cargo clippy`;
- Python 3 for repository boundary checks;
- a Markdown editor with link checking.

## Workspace

```text
crates/
├── invokrum-core/    parsing-neutral domain and engine
├── invokrum-schema/  strict YAML/JSON infrastructure adapter
└── invokrum-cli/     operator-facing delivery adapter
```

Each crate represents a durable dependency boundary:

- schema and delivery layers may depend inward on core;
- core must not depend on serialization or host libraries;
- CLI wiring belongs at a composition root rather than inside domain/application logic.

New crates should be added only when they create a meaningful dependency, compatibility, or trust boundary.

## Local checks

The standard local validation contract is:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 scripts/check_architecture.py
cargo test --workspace --lib --all-features --locked
cargo test -p invokrum-core --test integration --all-features --locked
cargo test -p invokrum-schema --test integration --all-features --locked
cargo test -p invokrum-cli --test e2e --all-features --locked
cargo doc --workspace --no-deps
```

Do not add a documented command to required CI until it works from a clean checkout.

## Design expectations

- Keep serialization, filesystem, process, network, and host dependencies outside `invokrum-core`.
- Keep Anthesis-specific policy outside all generic engine crates.
- Prefer typed domain states over repeated string validation.
- Avoid nondeterministic map iteration or declaration order in externally observable normalized output.
- Keep parser-library errors behind stable schema-boundary categories.
- Treat path resolution and canonicalization as security-sensitive.
- Separate human-readable CLI output from stable JSON contracts.
- Do not persist or log secret variable values by default.
- Avoid unsafe Rust unless an accepted architecture decision establishes a compelling need and review boundary.

## Testing strategy

Changes should use the smallest relevant combination of:

- unit tests for domain invariants;
- core integration tests for aggregate behavior and dependency-free execution;
- schema integration tests for strict DTO decoding and normalization;
- E2E tests that invoke the compiled CLI binary;
- golden fixtures for canonical output;
- cross-platform path fixtures;
- property tests for normalization and determinism;
- fuzz targets for concrete parser and path boundaries;
- compatibility fixtures derived from real consumers such as Anthesis.

Golden files are contracts, not snapshots to update blindly. Review the semantic reason for every change.

## Dependencies

Before adding a dependency, consider:

- which architecture layer owns it;
- whether the standard library is sufficient;
- maintenance and release activity;
- transitive dependency size;
- license compatibility with Apache-2.0;
- unsafe code and native build requirements;
- effect on reproducible static binaries;
- whether it becomes part of a public serialized contract.

The architecture check rejects known outer-layer dependencies in `invokrum-core`.

## Architecture decisions

Create an ADR under `docs/architecture/` for changes that affect:

- mechanism-versus-policy boundaries;
- canonicalization;
- schema or lockfile compatibility;
- trust boundaries;
- plugin execution;
- network behavior;
- persistent formats;
- public API stability.

## Commit and pull-request scope

Prefer focused changes with explicit validation. A PR should state:

- the problem and intended behavior;
- owning architecture layer and dependency direction;
- compatibility impact;
- security impact;
- tests performed at each relevant layer;
- documentation affected;
- follow-up work intentionally deferred.
