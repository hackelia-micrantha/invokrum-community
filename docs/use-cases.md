# Use cases

Invokrum is useful when prompt context is assembled from multiple sources and the composition itself must be reviewed, reproduced, or governed.

## Governed agent sessions

A platform can define a required core overlay, one execution mode, and optional security or quality overlays. Invokrum resolves the profile before the host starts the session and returns a digest that the host binds to execution evidence.

**Invokrum provides:** ordering, validation, rendering, and a resolved manifest.

**The host provides:** authorization, approvals, tool access, execution, and audit storage.

## Reproducible fresh-context handoff

Long-running agent runtimes may deliberately discard a failed or stale model context and instantiate a fresh executor from structured task state. Invokrum can make the prompt/context portion of that handoff reproducible without becoming the workflow-state authority.

A host can bind a task/run revision to an Invokrum lock and resolved manifest, then reconstruct the exact bounded prompt composition for a replacement executor:

```text
governed task revision
  -> Invokrum pack/profile/lock
  -> exact rendered-context identity
  -> fresh executor context
```

This is useful for runtime designs where retries should not inherit an entire failed conversation or scratch history.

**Invokrum provides:** deterministic context selection, source/output digests, lock verification, and a manifest identifying what entered the reconstructed context.

**The host/governance layer provides:** current task truth, retained versus superseded state, execution identity, verifier evidence, completion semantics, authorization, and recovery policy.

Important boundaries:

- an Invokrum lock proves context composition identity, not task correctness;
- context integrity does not prove that an executor performed the task successfully;
- a resolved manifest is evidence about inputs, not authorization for an effect;
- Invokrum does not decide which prior facts or results are trusted enough to carry into a fresh attempt;
- task completion and trusted-state promotion remain outside Invokrum.

This pattern is compatible with the state-integrity motivation in arXiv:2608.01964 (LongHorizon-Harness), but does not require Invokrum to implement that paper's manager/executor/auditor architecture.

## CI validation for prompt packs

A repository can validate that:

- referenced overlays exist;
- required classes are present;
- exclusive classes contain exactly one selection;
- incompatible overlays are rejected;
- rendered output and lockfiles have not drifted.

This supports reviewable prompt changes without granting CI permission to execute an agent.

## Reproducible evaluations

An evaluation harness can pin the exact prompt pack, profile, variables, and rendered digest used for a benchmark. Results can then distinguish model changes from context-composition changes.

## Secure code-review context

A code-review profile may combine:

- a non-relaxable review invariant;
- a read-only mode;
- repository-specific security constraints;
- output-format requirements;
- evidence and quality gates.

An implementation profile that conflicts with the read-only overlay should fail before invocation.

## Environment-specific context

A consumer may define mutually exclusive local, CI, and production environment overlays while retaining common governance and security layers. Invokrum validates the selection but does not decide which environment is authorized.

## Host and plugin adapters

Anthesis, an MCP server, a GitHub Action, or an editor extension can consume stable library or JSON output rather than reimplementing resolution rules. Adapters must preserve validation and provenance results rather than parsing human-oriented CLI text.

## Cases that do not require Invokrum

Invokrum may be unnecessary when:

- a prompt is a single static file with no composition rules;
- reproducibility and provenance are not required;
- a host already provides an equivalent deterministic composition contract;
- the desired behavior is unrestricted text templating rather than governed layering.

## Misuse boundaries

Invokrum validation does not prove that prompt content is trustworthy, free from prompt injection, legally compliant, or authorized for execution. It validates declared structure and integrity; semantic review and runtime controls remain separate responsibilities.
