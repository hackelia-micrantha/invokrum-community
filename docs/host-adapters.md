# Host adapters and subprocess integration

Invokrum v0.1 exposes a read-only integration boundary for libraries, subprocess hosts, Anthesis, MCP servers, CI systems, and editors. The boundary composes validated local inputs and derives evidence; it does not fetch packs, write files, invoke models, execute tools, or grant capabilities.

## Contract layers

### Rust library façade

`invokrum-host` is the transport-neutral application façade. A host supplies:

- a validated `OverlayPack`;
- a validated profile identifier;
- an `OverlaySource` implementation;
- explicit `CompositionLimits`;
- optionally, a `Digester` implementation.

`resolve_bundle` returns one `ResolvedBundle` containing:

- exact normalized context bytes;
- the ordered resolved manifest;
- the complete composition and source segments;
- the canonical lock value;
- exact canonical `invokrum.lock/v1` bytes.

All evidence is derived from one in-memory composition. Lock generation and verification do not reopen overlay sources. `verify_bundle` returns current bytes plus an ordered drift report. A host must not execute current bytes under the expected lock identity when verification reports drift.

The façade intentionally has no dependency on filesystem, schema, JSON, process, network, environment, clock, or runtime adapters.

### JSON subprocess adapter

`invokrum rpc` reads one JSON request from stdin and writes one JSON response to stdout. The protocol discriminator is `invokrum.host/v1`.

Request schema:

- [`schemas/invokrum-host-request-v1.schema.json`](../schemas/invokrum-host-request-v1.schema.json)

Response schema:

- [`schemas/invokrum-host-response-v1.schema.json`](../schemas/invokrum-host-response-v1.schema.json)

The adapter rejects:

- requests over 1 MiB;
- JSON nesting deeper than 32 mappings or sequences;
- duplicate known fields;
- unknown fields;
- unsupported protocol identifiers;
- empty or overlong request identifiers;
- malformed or noncanonical RFC 4648 base64;
- malformed, noncanonical, or oversized expected locks;
- invalid packs, profiles, compatibility declarations, or local sources.

RPC failures still produce one JSON response on stdout. Stderr remains empty after argument parsing succeeds. The process exit code identifies the broad failure category, while `error.code` supplies the stable machine category.

## Operations

### `capabilities`

Returns supported operations, default resource limits, and explicit negative capabilities:

```json
{
  "protocol": "invokrum.host/v1",
  "request_id": "cap-1",
  "operation": "capabilities"
}
```

The v1 adapter reports:

- `network_access: false`;
- `persistent_writes: false`;
- `runtime_invocation: false`.

### `resolve`

```json
{
  "protocol": "invokrum.host/v1",
  "request_id": "resolve-1",
  "operation": "resolve",
  "pack": "./pack.yaml",
  "profile": "secure-review"
}
```

The result includes:

- `context_base64`: exact normalized context bytes;
- `lock_base64`: exact canonical lock bytes;
- `output_digest`: digest of exact context bytes;
- a deterministic manifest with ordered source identities and byte counts.

Binary values use canonical padded RFC 4648 base64. Hosts must decode them as bytes rather than reinterpret them as text.

### `verify`

```json
{
  "protocol": "invokrum.host/v1",
  "request_id": "verify-1",
  "operation": "verify",
  "pack": "./pack.yaml",
  "profile": "secure-review",
  "expected_lock_base64": "..."
}
```

The response includes current resolved bytes, current canonical evidence, `verified`, and ordered drift categories. A drift result is a successful protocol operation, not a malformed request. Hosts decide whether drift blocks execution; the recommended default is to block.

## Capability and trust model

| Capability | V1 subprocess adapter | Owner |
| --- | --- | --- |
| Parse and validate a local pack | Read-only | Invokrum schema adapter |
| Read declared local overlays | Read-only, fail-closed Linux adapter | Invokrum filesystem adapter and protected host namespace |
| Compose exact context | Read-only | Invokrum core/host façade |
| Generate canonical lock evidence | In-memory response only | Invokrum integrity adapter |
| Verify expected evidence | Read-only | Invokrum host façade |
| Persist context or locks | Not available | Host or normal CLI with explicit output policy |
| Fetch packs or schemas | Not available | Host acquisition layer |
| Invoke a model or tool | Not available | Host runtime layer |
| Authorize an operation | Not available | Host policy layer |
| Authenticate a publisher | Not available | Host distribution/signature policy |

The subprocess adapter receives arbitrary local paths from its caller. The caller must authorize which roots may be accessed before invoking Invokrum. Invokrum proves local containment under its adapter policy; it does not decide whether the caller is entitled to read that pack.

## Binding evidence to execution

A host that invokes a model or tool should record:

1. the exact decoded `context_base64` bytes;
2. the returned `output_digest`;
3. canonical lock bytes or their authenticated storage identity;
4. the pack and profile selected;
5. the host policy and capability set;
6. the runtime request identity;
7. any transformation performed after Invokrum.

Any transformation of the context creates a new artifact. The original Invokrum digest must not be represented as covering appended instructions, templating, encoding conversion, or host-added wrappers.

## Anthesis adapter design

The initial Anthesis integration should call `resolve_bundle` directly where Rust embedding is practical, or use `invokrum rpc` otherwise. Anthesis remains responsible for:

- actor identity and authorization;
- policy and approval evaluation;
- capability grants;
- runtime sandboxing;
- provenance storage and retention;
- binding exact Invokrum bytes and digest to an execution event.

An Anthesis adapter should preserve the entire returned manifest and lock bytes without reconstructing them from human output. Anthesis-specific overlay declarations belong in the compatibility pack tracked separately; they do not belong in the generic host façade.

## MCP scope

An initial MCP server should expose read-only tools only:

- list declared profiles from an already authorized pack;
- resolve a selected profile;
- inspect the resolved manifest;
- verify caller-supplied expected lock evidence.

It should not expose pack fetching, arbitrary file browsing, persistent writes, model invocation, shell execution, or automatic approval. Pack roots should be configured or authorized by the MCP host rather than supplied without policy by an untrusted model.

## GitHub Actions guidance

- Run on GitHub-hosted runners for pull requests from untrusted forks.
- Check out the pack repository with read-only permissions.
- Use `invokrum rpc` or stable `--format json` output; never parse human output.
- Decode returned context and lock bytes exactly.
- Store evidence only when repository policy permits it.
- Do not expose release, repository-write, OIDC, or secret permissions to workflows that execute untrusted pack changes.
- Treat lock drift as a blocking check unless the workflow explicitly reviews and updates evidence.

## Editor guidance

An editor extension should use a long-lived editor process but a short-lived Invokrum subprocess per request. It should:

- keep pack-root authorization in editor configuration;
- call `capabilities` before relying on optional operations;
- debounce repeated resolves;
- display manifests and drift without silently invoking tools;
- avoid logging decoded context when it may contain sensitive data;
- never rewrite returned bytes while presenting the original digest.

## Compatibility

The compatibility identifier covers request and response field meaning, operation names, base64 representation, and error categories. Additive optional response fields may be introduced within v1. Removing or reinterpreting fields, changing canonical binary representation, or changing operation semantics requires a new protocol identifier.

Resource policy may become stricter without changing the protocol identifier. Hosts should discover current defaults through `capabilities` and handle deterministic limit failures.
