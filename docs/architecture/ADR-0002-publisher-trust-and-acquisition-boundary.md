# ADR-0002: Separate publisher trust and acquisition from offline composition

- Status: Accepted
- Date: 2026-08-08
- Issue: #46

## Context

Invokrum v0.1 deliberately composes only local pack data. The engine can prove deterministic structure, exact-byte identity, and drift, but an unkeyed digest cannot answer who published a pack or whether a remote locator still refers to the reviewed artifact.

Remote acquisition introduces a different authority boundary from composition:

- a network location can be mutable or compromised;
- a valid signature is meaningless unless the signer identity is authorized by host policy;
- archive extraction can create path, link, collision, and resource attacks before Invokrum sees a local root;
- update aliases such as branches, `latest`, and floating tags do not identify immutable security artifacts;
- revocation, expiry, and freshness are time- and policy-dependent, while deterministic composition must remain independent of network and clock state.

Adding fetching directly to `invokrum-core`, the local filesystem adapter, or `invokrum rpc` would collapse these concerns and make deterministic composition depend on external trust infrastructure.

## Decision

Invokrum will keep **acquisition and publisher trust as a distinct outer-layer capability**. Verified acquisition produces a protected local installation. Existing composition consumes that local installation exactly as it consumes any other authorized local root.

```text
remote candidate
      │
      ▼
acquisition transport        untrusted bytes
      │
      ▼
trust policy + verifier      external authority decision
      │
      ▼
safe package validation      bounded paths/files/digests
      │
      ▼
quarantine installation
      │
      ▼
atomic promotion
      │
      ▼
content-addressed local root + installation record
      │
      ▼
existing offline Invokrum composition
```

### Composition remains offline

`invokrum-core`, schema parsing, integrity generation, local source reading, and normal `invokrum rpc` composition must not:

- fetch remote data;
- discover publishers;
- consult transparency logs;
- query revocation services;
- read ambient credential stores;
- decide which publisher is trusted;
- silently update installed packs.

Acquisition is invoked explicitly by a host or future acquisition command/component.

### Trust policy is external to the pack

A pack may state descriptive publisher metadata, but it cannot grant authority to its own signer or relax verification requirements.

The acquisition caller supplies an immutable trust policy for one operation. At minimum the design supports:

1. **exact digest pinning** — an expected artifact or signed-manifest digest supplied out of band;
2. **publisher assertion verification** — a pluggable verifier establishes a signer identity, and host policy decides whether that identity is authorized.

A cryptographically valid signature without an authorized identity match is a failure.

Trust-on-first-use is not a default mode. If a host implements TOFU, it must name that policy explicitly and persist the first-seen identity as host-owned state.

### Signing provider is an outer adapter

The application-level acquisition contract consumes a narrow verification result such as:

- verification mechanism/version;
- verified publisher identity;
- exact signed subject digest;
- relevant validity/freshness evidence;
- deterministic failure category.

Concrete providers such as Sigstore-style bundles, long-lived keys, enterprise PKI, or an offline verifier are infrastructure adapters. Core composition never depends on a signing provider SDK.

### Signed subject identifies content, not a mutable locator

The security identity of a distributed pack is an immutable digest-backed subject. URLs, repository names, branches, release names, registry coordinates, and tags are locators or metadata only.

A future signed pack bundle should bind a versioned canonical manifest that identifies:

- bundle format/version;
- pack entry point;
- every installable file path;
- exact byte length of every file;
- digest algorithm and digest of every file;
- optional descriptive package/version metadata that is not itself authority.

The signature covers the canonical manifest (or its exact digest). The transport archive is validated against that signed file manifest before installation. Repacking transport bytes is therefore distinguishable from changing signed pack content.

### Installation is fail-closed and content-addressed

Before promotion, acquisition must:

1. apply explicit limits to downloaded bytes, entries, per-file bytes, total expanded bytes, and path depth;
2. validate every path with portable pack-relative rules;
3. reject absolute paths, traversal, duplicate or normalization-colliding paths, links, devices, FIFOs, sockets, and other special entries;
4. unpack only into a newly created quarantine root;
5. read installed candidate files from that quarantine root and verify exact length/digest against the signed manifest;
6. create a host-owned installation record binding trust policy result, signed subject, file-tree identity, and pack entry point;
7. atomically promote into a content-addressed location derived from the verified subject;
8. refuse to replace an existing different installation in place.

The promoted directory must be protected from untrusted mutation by the host. Composition still re-hashes exact input bytes in its normal lock/evidence path; publisher authentication and composition integrity remain separate claims.

### Installation evidence

The installation record is host/acquisition evidence, not a pack-controlled file. It records enough information to answer:

- what immutable subject was verified;
- which publisher identity satisfied which trust policy;
- which verification mechanism produced that result;
- which exact local file tree and pack entry point were installed;
- whether freshness/revocation revalidation is required before future use.

The installation record must not be reconstructed from human-readable output when a machine contract exists.

### Update, freshness, and rollback semantics

An update is a new acquisition decision that produces a new immutable installation. Existing installations are never silently mutated.

- `latest`, branches, channels, and floating tags may locate candidates but are never sufficient proof of identity.
- A host may require a minimum accepted version/sequence or previously observed signed metadata to reject rollback.
- Expiry, certificate validity, revocation, transparency inclusion, and policy freshness are acquisition/host concerns.
- Composition does not perform network revalidation. A host that requires fresh publisher authorization checks acquisition evidence before selecting an installed root.
- A registry may advertise versions or locators, but registry metadata cannot override trust policy or signed subject identity.

### Error categories

Future acquisition APIs should expose stable categories rather than provider-specific text, including at least:

- `transport` — retrieval failed or exceeded bounds;
- `trust` — no authorized publisher identity or digest match;
- `signature` — malformed, invalid, expired, or unsupported verification material;
- `package` — unsafe archive/file structure or signed-manifest mismatch;
- `rollback` — candidate violates explicit monotonic/freshness policy;
- `install` — quarantine or atomic promotion failed;
- `internal` — invariant violation.

## Consequences

### Positive

- Deterministic composition remains independent of network, clock, credentials, registries, and signer infrastructure.
- Publisher authentication can evolve without changing the pack composition model.
- Hosts can use enterprise or public signing systems through adapters while sharing one acquisition use case.
- Digest pinning remains a simple high-assurance mode for CI and tightly controlled deployments.
- Secure extraction and immutable installation become explicit security operations instead of incidental archive handling.
- Registry compromise is constrained because discovery metadata cannot grant publisher authority.

### Costs

- Distribution requires additional metadata and installation evidence beyond the existing `invokrum.lock/v1` composition lock.
- Hosts that care about revocation or freshness must manage revalidation policy separately from composition.
- Multiple signing providers require adapter conformance tests and a stable normalized verification result.
- Content-addressed installs consume additional disk until garbage collection is explicitly designed.

## Rejected alternatives

### Fetch remote overlays during composition

Rejected because mutable network state would break deterministic composition and combine runtime availability with trust decisions.

### Treat SHA-256 lockfiles as publisher authentication

Rejected because anyone who can replace pack bytes can recompute unkeyed hashes.

### Trust any cryptographically valid signer

Rejected because authenticity requires an authorized identity, not merely a mathematically valid signature.

### Trust repository names, tags, or `latest`

Rejected because these names are mutable administrative references rather than immutable artifact identities.

### Let packs declare their own trust roots

Rejected because untrusted input cannot authorize itself.

### Extract first and verify later

Rejected because archive extraction itself is an attack surface. Verification and structural validation must occur in quarantine before promotion.

## Follow-up

- #46 records the accepted trust model and documentation baseline.
- A subsequent implementation issue should define the versioned signed bundle manifest and normalized verification result.
- Remote transport/registry discovery must remain blocked until the signed-install path is implemented and adversarially tested.
- Plugin execution remains separately blocked on a capability and sandbox model.