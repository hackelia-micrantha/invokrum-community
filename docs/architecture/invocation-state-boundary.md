# Invocation State Boundary

Status: architecture proposal  
Date: 2026-08-12  
Tracking: #68

## Purpose

Invokrum should provide a model-neutral invocation boundary that is precise about portable task state without standardizing runtime-private computation.

The portable contract distinguishes:

- **task context** — structured state supplied to the invocation;
- **working state** — runtime-private intermediate computation;
- **execution evidence** — structured observable records produced around execution.

This is an ownership boundary, not yet a commitment to a new public schema version.

## Contract direction

```text
InvocationContract
  identity / version
  inputs
  task_context
  capabilities
  constraints
  budget
  termination
  output_contract
  evidence_requirements
          |
          v
       runtime
   private working state
          |
          v
ExecutionEvidence
```

A host may render parts of the contract into a prompt, recurrent context, symbolic program, RPC request, or another runtime-specific form. The rendered form is derived material, not the portable semantic source of truth.

## Task context

Task context is serializable, bounded, and exact where it affects authority, reproducibility, or compatibility.

Candidate logical content:

```yaml
task_context:
  task_ref: task:...
  state_revision: 17
  state_digest: sha256:...
  source_ref: git:...
  selected_context_refs: []
  selected_example_refs: []
  memory_refs: []
  delegation_ref: null
```

The eventual schema should reuse existing Invokrum vocabulary and canonicalization rules rather than adopt these field names mechanically.

Rules:

- mutable ambient history is not authoritative task context;
- selected memory/context enters by explicit reference or bounded value;
- material revision changes produce a new task-context identity;
- a child/delegated invocation receives a narrowed context view;
- task context may request capability/constraint semantics but does not authenticate the caller or grant authority by itself.

## Working state

Working state is owned by the runtime.

Examples include model-specific intermediate vectors, scratch data, search frontiers, planner state, temporary specialist hypotheses, and other algorithm-specific computation.

Invokrum rules:

- do not define a public `latent_state` field;
- do not require working state to be serializable;
- do not treat working state as authority or evidence merely because it exists;
- do not make working-state persistence a composition-core concern;
- optional resume/checkpoint handles, if later justified, must be opaque and explicitly versioned as host/runtime capability.

## Execution evidence

Execution evidence attributes observable execution to the invocation contract without becoming policy authority.

Candidate logical content:

```yaml
execution_evidence:
  invocation_ref: invocation:...
  task_context_digest: sha256:...
  effective_runtime_ref: runtime:...
  execution_principal_ref: principal:...
  effect_refs: []
  artifact_refs: []
  evaluation_refs: []
  consumed_budget_ref: budget:...
  outcome: completed | partial | failed | cancelled | indeterminate
```

Provider-specific evidence stays provider-specific unless normalized through an explicit adapter/profile.

## Retry and resume

Retry and resume must preserve identity and accounting boundaries:

- retry does not mint new capabilities;
- resume does not reset consumed budget;
- a changed authority-relevant task revision requires a new/re-evaluated invocation subject;
- stale checkpoints cannot be used to bypass current constraints;
- evidence identifies whether an execution was fresh, retried, resumed, or delegated.

## Ownership

```text
Calathea  -> authored workflow / project state
Invokrum  -> portable invocation/task contract
Dubnium   -> working state / execution / persistence / recovery
Keylix    -> authenticated execution principal / delegated PoP
Anthesis  -> policy / approval / evidence interpretation
```

Invokrum must not become any of the neighboring systems merely to make the contract expressive.

## Security invariants

- Evidence cannot authorize a future effect.
- Caller-supplied identity fields cannot override authenticated runtime identity.
- Sensitive raw prompts, complete transcripts, credentials, and runtime-private state are not mandatory contract fields.
- Task-context canonicalization must not admit ambiguous authority-bearing representations.
- Unsupported resume or evidence capabilities fail explicitly rather than being silently approximated.
- Hosts must not treat mutable aliases or ambient state as immutable invocation identity.

## Compatibility targets

The same conceptual contract should support at least:

1. an ordinary stateless request/response model host;
2. a Dubnium supervisor/specialist run with bounded state and recovery;
3. a recurrent/iterative or other runtime whose internal computation is not externally serialized.

## Research motivation

BDH-CQ (arXiv:2608.09888) is a useful signal because it separates contextual task acquisition from active recurrent computation. Invokrum's architectural response is not to implement that model, but to avoid a portable contract that assumes all meaningful state is prompt text or an external reasoning sequence.

## Next step

Issue #68 owns QART against the current schema and host-adapter surfaces. Prefer the smallest stable extension: architecture guidance or profile first, schema/ADR promotion only when a concrete consumer proves the need.