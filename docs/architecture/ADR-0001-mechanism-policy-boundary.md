# ADR-0001: Separate composition mechanism from consumer policy

- Status: Accepted
- Date: 2026-08-02
- Issue: #1

## Context

Invokrum originates from the prompt-overlay system used by Anthesis. That system combines two different concerns:

1. a reusable mechanism for loading, validating, ordering, composing, hashing, and inspecting layered prompt context;
2. Anthesis-specific policy, including its overlay taxonomy, authority precedence, STOP semantics, approvals, evidence binding, and runtime behavior.

Publishing a standalone engine is useful only if the reusable mechanism can evolve without importing Anthesis policy or weakening the guarantees expected by Anthesis and other consumers.

## Decision

Invokrum will be a policy-neutral, offline-first composition engine implemented as a Rust workspace.

The initial workspace contains:

- `invokrum-core`: domain types and deterministic composition behavior;
- `invokrum-cli`: the operator and subprocess interface over the core.

Additional crates may be introduced only when they represent a durable boundary rather than packaging convenience.

### Invokrum owns

- versioned pack, overlay, profile, rule, and resolved-manifest models;
- parsing and normalization of supported local formats;
- deterministic ordering and diagnostics;
- cardinality and compatibility validation;
- canonical path containment and explicit symlink behavior;
- canonical serialization, hashing, lockfiles, and verification;
- rendering from validated, resolved inputs;
- stable machine-readable CLI output and exit-code contracts.

### Consumers own

- the semantic meaning and authority of overlay classes;
- policy content and prompt text;
- approval, identity, and authorization decisions;
- execution of agents or tools;
- evidence retention and audit policy;
- remote acquisition and trust decisions for packs;
- binding a rendered-context digest to a runtime execution.

Anthesis therefore remains responsible for its core invariant, fixed taxonomy, STOP semantics, governed sessions, approvals, evidence, and provenance records. Its integration with Invokrum will be an adapter and policy pack, not a branch in the generic engine.

## Core invariants

1. Identical supported inputs produce byte-identical normalized output.
2. Filesystem enumeration and unordered map iteration never determine composition order.
3. Ambiguous, unsupported, or unresolved configurations fail closed.
4. Composition performs no network access.
5. Pack-relative paths cannot escape the canonical pack root.
6. Sensitive variable values are not persisted in manifests, lockfiles, or diagnostics by default.
7. Human-oriented output is not an integration API; adapters consume typed library results or versioned JSON.
8. The engine never interprets validated prompt content as executable instructions.

## Error model

Public errors will be grouped into stable categories while retaining detailed internal causes:

- `input`: unreadable, malformed, oversized, or unsupported input;
- `schema`: invalid schema or unsupported schema version;
- `path`: missing file, traversal, root escape, or prohibited symlink behavior;
- `resolution`: missing, duplicate, or ambiguous references;
- `cardinality`: minimum or maximum class constraints violated;
- `compatibility`: required, prohibited, or mutually incompatible selections;
- `render`: deterministic rendering could not complete;
- `verification`: lockfile, digest, or canonicalization mismatch;
- `internal`: invariant violation or unexpected engine failure.

The CLI will map these categories to documented exit codes. Exact diagnostic wording is not a compatibility guarantee; category, rule identifier, source location, and JSON fields are.

## Compatibility model

Four versions are independent and must be recorded explicitly:

- engine version;
- schema family and version;
- pack version;
- consumer or adapter version.

The v0.x project may evolve quickly, but persisted schemas, lockfiles, JSON output, and exit codes require migration notes whenever changed. A future stable release will publish a compatibility matrix and deprecation window.

Unknown major schema versions fail closed. Unknown fields within a supported schema version are rejected by default unless the schema explicitly marks an extension point.

## v0.1 non-goals

The initial release will not:

- execute models, agents, tools, or plugins;
- fetch prompt packs over the network;
- define a universal prompt-policy taxonomy;
- provide a hosted registry or marketplace;
- evaluate the truth, safety, or effectiveness of prompt prose;
- resolve conflicts using an LLM;
- provide secret storage;
- replace consumer authorization or approval systems;
- guarantee semantic equivalence across different rendering templates.

## Consequences

### Positive

- Anthesis becomes a real compatibility consumer without controlling the core architecture.
- Other tools can adopt deterministic prompt composition without adopting Anthesis governance.
- Security-sensitive path and serialization behavior is centralized and testable.
- A static Rust CLI is suitable for local development, CI, and host adapters.

### Costs

- Engine, schema, pack, and adapter versions create an explicit compatibility matrix.
- Some consumer policies cannot be represented until the generic rule model supports them.
- Strict offline and fail-closed defaults reduce convenience compared with implicit remote loading or permissive parsing.

## Follow-up

- #2 defines the typed domain model.
- #3 defines the portable schema.
- #4 implements deterministic composition and validation.
- #5 defines hashing and lockfiles.
- #6 establishes the CLI contract.
- #7 proves compatibility with selected Anthesis fixtures.
- #9 expands the threat model.
- #12 defines host-adapter contracts.
