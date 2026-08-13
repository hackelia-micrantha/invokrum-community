# Governed code-review example

This example demonstrates a small, auditable Invokrum pack without requiring Anthesis knowledge.

The pack defines three authority layers:

1. `core` — exactly one required governance overlay;
2. `mode` — exactly one operating mode;
3. `concern` — up to two optional review concerns.

The valid `governed-review` profile resolves in this deterministic order:

1. `core-governance`;
2. `review`;
3. `security`;
4. `quality`.

Class order comes from each class's numeric `order`. Within `concern`, the profile's explicit `security`, then `quality` order is preserved.

> [!IMPORTANT]
> The current local filesystem and persistent-output adapters support Linux only. Run these commands from the repository root on a Linux host with a stable mount namespace.

## Validate the pack

```bash
cargo run -p invokrum-cli -- validate \
  --pack examples/governed-code-review/pack.yaml \
  --profile governed-review \
  --format json
```

The machine result is a single `invokrum.cli/v1` JSON object on stdout. Diagnostics, if any, use stderr.

## Inspect the resolved profile

```bash
cargo run -p invokrum-cli -- inspect \
  --pack examples/governed-code-review/pack.yaml \
  --profile governed-review \
  --format json \
  > /tmp/governed-inspect.json

cmp /tmp/governed-inspect.json \
  examples/governed-code-review/expected/inspect.json
```

The committed manifest records four ordered entries, **435 exact source bytes**, and **441 normalized output bytes**. The six additional bytes are the three two-line-feed separators between adjacent overlays.

## Compose exact context bytes

```bash
cargo run -p invokrum-cli -- compose \
  --pack examples/governed-code-review/pack.yaml \
  --profile governed-review \
  > /tmp/governed-context.md

cmp /tmp/governed-context.md \
  examples/governed-code-review/expected/context.md
```

Overlay files and the expected context intentionally have no trailing newline. Composition inserts exactly two line-feed bytes between adjacent source byte sequences and makes no other textual transformation.

## Generate and compare a lockfile

```bash
cargo run -p invokrum-cli -- lock \
  --pack examples/governed-code-review/pack.yaml \
  --profile governed-review \
  > /tmp/governed.lock

cmp /tmp/governed.lock \
  examples/governed-code-review/expected/invokrum.lock
```

The lock is exact canonical `invokrum.lock/v1` JSON without a trailing newline. It records structural identities and SHA-256 digests for the pack, selected profile, ordered overlays, engine inputs, rendered output, and manifest.

The unkeyed manifest digest detects corruption and drift. It is **not** a publisher signature, authorization decision, or proof that the prompt content is semantically safe.

## Verify committed evidence

```bash
cargo run -p invokrum-cli -- verify \
  --lock examples/governed-code-review/expected/invokrum.lock \
  --pack examples/governed-code-review/pack.yaml \
  --profile governed-review \
  --format json
```

A clean checkout returns exit code `0`, `"verified": true`, and an empty `drifts` array. Changing an overlay, profile, or pack declaration causes deterministic drift categories and exit code `5`.

## Reproduce an invalid composition

The `invalid-read-only-implementation` profile is structurally valid, but it selects both the `implementation` mode and the incompatible `read-only` concern:

```bash
cargo run -p invokrum-cli -- compose \
  --pack examples/governed-code-review/pack.yaml \
  --profile invalid-read-only-implementation
```

The command exits with code `4`, emits no composed bytes, and reports:

```text
error[composition]: overlay `implementation` is incompatible with `read-only`
```

Compatibility is evaluated before any selected overlay source is read, so an invalid combination cannot produce partial context.

## Committed contracts

- [`pack.yaml`](pack.yaml) — schema, classes, overlays, profiles, and incompatibility rules;
- [`overlays/`](overlays/) — deliberately small source overlays;
- [`expected/context.md`](expected/context.md) — exact rendered bytes;
- [`expected/inspect.json`](expected/inspect.json) — versioned resolved-manifest JSON;
- [`expected/invokrum.lock`](expected/invokrum.lock) — canonical lock bytes.

The CLI E2E suite reproduces every committed artifact from a clean checkout and checks the invalid profile failure path.
