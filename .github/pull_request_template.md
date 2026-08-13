## Problem

<!-- What concrete problem does this change solve? -->

## Approach

<!-- Summarize the implementation and important alternatives considered. -->

## Architecture

- [ ] The owning layer is identified: domain, application, infrastructure, or delivery/host.
- [ ] Dependencies point inward.
- [ ] Domain/application code does not directly access filesystem, network, process environment, clock, or global state.
- [ ] External behavior is behind a narrow port owned by the consuming layer.
- [ ] Concrete dependencies are wired at a composition root.
- [ ] No service locator, hidden singleton, or generic manager/service object was introduced.

### Dependency injection

<!-- List new or changed injected dependencies and their composition root. -->

### Design patterns

<!-- Name any non-trivial pattern used, the problem it solves, and why a simpler direct design was insufficient. Write "None" when not applicable. -->

## Compatibility

- [ ] Schema, lockfile, manifest, canonicalization, JSON, exit-code, public API, and adapter impacts are identified.
- [ ] Breaking or experimental changes are documented.

## Security

- [ ] Trust-boundary and sensitive-data impacts are identified.
- [ ] Path handling, canonicalization, redaction, and failure behavior were reviewed where applicable.

## Testing

- [ ] Unit tests cover domain/application rules and negative cases.
- [ ] Integration tests cover affected real adapters and boundaries.
- [ ] E2E tests cover affected public CLI behavior.
- [ ] Security-sensitive behavior includes adversarial tests.
- [ ] Tests are deterministic, offline by default, and isolated from user state.

### Validation performed

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features --locked
```

<!-- Add any focused unit, integration, or E2E commands. Explain why a test layer is not applicable rather than silently omitting it. -->

## Documentation and follow-up

- [ ] User or architecture documentation reflects the implemented behavior.
- [ ] Deferred work and residual risk are linked to issues.