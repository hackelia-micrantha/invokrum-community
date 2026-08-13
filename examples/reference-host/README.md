# Independent reference host

This example proves that Invokrum can be consumed as a standalone released executable through `invokrum rpc`. It does not import Invokrum workspace crates and has no Anthesis dependency.

## Trust boundary

The reference host owns pack-path authorization, subprocess lifetime, persistence, and the decision to block on drift. Invokrum owns strict request parsing, local source containment, deterministic composition, and canonical evidence generation.

The host:

- invokes the executable with a structured argument vector (`[invokrum, "rpc"]`), never a shell command string;
- sends exactly one bounded JSON request on stdin and expects exactly one JSON object on stdout;
- treats any RPC stderr as a host integration failure;
- uses a five-second default subprocess timeout and a 1 MiB response limit;
- validates protocol identity, request correlation, operation identity, required capabilities, exact negative capabilities, canonical base64, and digest shape;
- persists decoded context and lock bytes itself with owner-only permissions where the platform supports them;
- does not execute, template, or reinterpret composed prompt bytes.

A production host should additionally apply OS-level process, memory, CPU, filesystem, and namespace restrictions appropriate to its deployment environment.

## Run against v0.1.0

Download and verify the appropriate `v0.1.0` release binary first. Then:

```bash
cd examples/reference-host
python3 reference_host.py --invokrum /path/to/invokrum capabilities
```

Resolve the standalone example profile and persist exact bytes:

```bash
python3 reference_host.py \
  --invokrum /path/to/invokrum \
  resolve \
  --pack pack.yaml \
  --profile default \
  --context-out .state/context.txt \
  --lock-out .state/invokrum.lock.json
```

Verify the persisted lock:

```bash
python3 reference_host.py \
  --invokrum /path/to/invokrum \
  verify \
  --pack pack.yaml \
  --profile default \
  --lock .state/invokrum.lock.json
```

The command exits `0` when verified and `2` when deterministic drift is reported.

To demonstrate drift, edit `overlays/review.md` and run the verify command again. The host surfaces Invokrum's ordered drift categories without updating the expected lock.

## Adapter pattern

This example deliberately stops at the host boundary:

```text
consumer policy / authorization
          |
          v
reference_host.py
  capabilities -> resolve -> persist evidence -> verify
          |
          v
    invokrum rpc
```

An MCP server, editor extension, or CI adapter can reuse the same pattern: authorize a pack root outside Invokrum, negotiate capabilities, invoke a short-lived subprocess, preserve exact returned bytes, and block runtime use when expected evidence drifts.

Do not expose arbitrary pack paths directly to an untrusted model or tool caller. Path authorization belongs in the consuming host.
