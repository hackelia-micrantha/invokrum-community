# Clean architecture, SOLID, dependency injection, and design patterns

This document defines mandatory design constraints for Invokrum implementation work.

## Dependency direction

```text
Delivery / hosts
    ↓
Infrastructure adapters
    ↓
Application use cases
    ↓
Domain model and rules
```

Dependencies point inward. Inner layers must not depend on outer-layer implementations.

## Layer responsibilities

### Domain

Owns behavior that can run without filesystem, process, network, serialization, clock, randomness, or environment access.

Examples include overlay/profile invariants, ordering, cardinality, compatibility rules, normalized errors, and validated value objects.

### Application

Coordinates domain behavior through explicit use cases such as validate, compose, inspect, lock, verify, and diff. Application code may depend on domain types and narrow ports, but not concrete infrastructure.

### Infrastructure

Implements ports for filesystem access, path canonicalization, YAML/JSON parsing, digest calculation, lockfile persistence, clocks, and process I/O. Infrastructure must not contain domain policy.

### Delivery and hosts

CLI, MCP, CI, Anthesis, and editor adapters translate external requests into application inputs and render outputs. They are composition roots and may select concrete adapters.

## SOLID requirements

- **Single responsibility:** modules and types should have one coherent reason to change.
- **Open/closed:** extend genuine variation points through stable contracts; do not pre-abstract hypothetical variants.
- **Liskov substitution:** every adapter must preserve the semantics and failure contract of its port.
- **Interface segregation:** prefer narrow capability traits over broad manager or service interfaces.
- **Dependency inversion:** application policy owns the interfaces it consumes; infrastructure implements them.

## Dependency injection

Use explicit constructor injection by default.

```rust
pub struct ComposeUseCase<R, H> {
    reader: R,
    hasher: H,
}

impl<R, H> ComposeUseCase<R, H> {
    pub fn new(reader: R, hasher: H) -> Self {
        Self { reader, hasher }
    }
}
```

Rules:

- wire concrete dependencies only in composition roots;
- inject behavior through narrow traits or generic parameters;
- prefer owned immutable dependencies;
- use `Arc<dyn Trait>` only when runtime polymorphism or shared ownership is actually required;
- do not read environment variables or global process state inside domain/application logic;
- do not use service locators, mutable global registries, hidden singletons, or dependency lookup by string.

## Design patterns

Patterns are allowed when they solve a concrete boundary, lifecycle, or variation problem. Every non-trivial pattern should be explainable in the PR that introduces it.

Appropriate candidates:

- **Ports and Adapters / Hexagonal Architecture** for external boundaries.
- **Strategy** for rendering, canonicalization, hashing, and rule-evaluation variants.
- **Command** for application use cases and CLI operations.
- **Factory** at composition roots when validated configuration selects concrete adapters.
- **Builder** for complex immutable domain values and readable test fixtures.
- **Adapter** for file formats, host APIs, and legacy compatibility.
- **Decorator** for tracing, metrics, or caching that preserves the wrapped contract.
- **Newtype / Value Object** for identifiers, versions, digests, names, and canonical paths.
- **Repository** only when there is a meaningful persistence abstraction; simple file reads do not justify generic CRUD repositories.

Avoid:

- service locator and singleton patterns;
- generic `Manager`, `Helper`, or `Service` objects with unrelated responsibilities;
- speculative interfaces with one trivial implementation and no test seam;
- inheritance-style hierarchies encoded through oversized trait trees;
- factories that only hide ordinary constructors;
- visitor/event-bus machinery where direct typed calls are clearer.

## Error boundaries

- domain errors describe invariant or rule failures;
- infrastructure errors retain causal context without leaking secrets;
- application errors map lower-level failures into stable operation-level categories;
- CLI/host layers own exit codes, presentation, and redaction.

## Review checklist

Every implementation PR must answer:

1. Which layer owns this behavior?
2. Do dependencies point inward?
3. Which ports are required, and who owns them?
4. Where are concrete dependencies constructed?
5. Which design pattern is used, what problem does it solve, and is a simpler direct design sufficient?
6. Can domain/application behavior run with in-memory or fake adapters?
7. Which unit, integration, and E2E tests prove the behavior?