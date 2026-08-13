# Bundle distribution and acquisition boundary

**Status:** Implemented distribution identity, bounded local directory/archive acquisition, exact candidate verification, Ed25519 publisher verification, explicit host authorization, and Linux content-addressed installation. Remote transport, registry discovery, freshness/revocation/transparency, and rollback-selection policy remain planned.

ADR-0002 defines acquisition as an outer concern relative to deterministic composition. The implemented dependency graph keeps distribution identity, acquisition verification, publisher verification/authorization, installation, and composition policy separate over narrow contracts and one neutral digest primitive.

```text
Linux directory / bounded POSIX ustar / future remote adapter
              ↓
      candidate source adapters
              ↓ owned CandidateFile bytes
      invokrum-acquisition
    exact subject/content verification
        ↓               ↓
invokrum-distribution  invokrum-digest
 bundle + trust domain   SHA-256 primitive
        ↑               ↑
        │               │
invokrum-distribution-json
 canonical JSON + subject identity

raw Ed25519 evidence
        ↓
invokrum-verifier-ed25519
        ↓ PublisherAssertion
host TrustPolicy
        ↓ AuthorizedPublisher
invokrum-install
        ↓
invokrum-install-linux
 private quarantine / CAS
        ↓
protected local root
        ↓
ordinary composition

invokrum-integrity ────> invokrum-digest
 composition locks
        ↓
   invokrum-core
```

`invokrum-distribution` owns parsing-neutral bundle and publisher-policy values. It has no dependency on composition, digest, serialization, filesystem, network, clock, environment, credentials, signing providers, or acquisition application policy.

`invokrum-distribution-json` is an outward serialization adapter. It depends on the distribution domain, neutral digest primitive, and Serde/JSON. It does not depend on composition or acquisition and performs no network/filesystem access.

`invokrum-acquisition` is an application-layer exact-verification use case. It depends only on provider-neutral bundle values and digest primitives. It accepts exact in-memory candidate bytes from outer source adapters, verifies subject/path/length/digest identity, and returns those same verified byte buffers as `VerifiedBundle`. It performs no filesystem, archive, network, serialization, process, environment, clock, credential, trust-store, or provider-specific signature access.

`invokrum-acquisition-linux` and `invokrum-acquisition-archive` are outer candidate-source adapters. The Linux adapter enforces the documented fail-closed local filesystem policy. The archive adapter accepts only the bounded uncompressed POSIX ustar profile and returns owned candidate bytes without filesystem extraction. Both converge on the same `invokrum-acquisition` verification contract.

`invokrum-verifier-ed25519` is the first concrete publisher-verification adapter. It verifies only the versioned `ed25519-subject-v1` message for the exact immutable subject and creates normalized `PublisherAssertion` evidence only after cryptographic success.

Host-owned `TrustPolicy` authorization remains distinct from signature validity. `invokrum-install` creates `AuthorizedPublisher` only after explicit policy success for the same subject, then composes exact candidate verification and installation without moving policy into filesystem code.

`invokrum-install-linux` materializes only verified owned bytes through private quarantine, staged re-verification, versioned installation evidence, and atomic content-addressed promotion. It is cryptography-blind and trust-policy-blind.

`invokrum-digest` is dependency-free and provides shared deterministic SHA-256 primitives. Sharing the digest implementation does not merge distribution identity, acquisition verification, publisher evidence, installation evidence, and composition-lock claims.

The composition dependency graph does not depend on distribution or acquisition:

```text
candidate source → distribution identity → acquisition verification → installation → local root → composition
composition ──X──> acquisition/distribution
```

## Implemented contracts

- versioned `invokrum.pack-bundle/v1` manifest identity;
- normalized printable-ASCII bundle paths and lowercase SHA-256 digests;
- deterministic sorted file records and explicit entry-point membership;
- hard bundle/file/path resource limits;
- strict canonical JSON with duplicate/unknown-field rejection;
- immutable bundle subject derived from canonical bytes;
- provider-neutral verification-mechanism and publisher-identity values;
- explicit host-owned allow rules and deterministic policy failures;
- dependency-free SHA-256 primitive with published test vectors;
- bounded Linux local-directory candidate loading;
- bounded uncompressed POSIX ustar candidate ingestion without filesystem extraction;
- offline candidate verification against expected bundle subject;
- deterministic missing/extra/duplicate/length/digest failure categories;
- `VerifiedBundle` retaining the exact byte buffers that were hashed;
- strict concrete `ed25519-subject-v1` verification and normalized `key.sha256` identity;
- explicit host authorization producing `AuthorizedPublisher` only after verifier/policy success;
- Linux private quarantine, staged verification, and content-addressed atomic promotion;
- deterministic `invokrum.installation/v1` evidence for installs where publisher authentication was not provided;
- structured `invokrum.installation/v2` evidence for verified-and-authorized publisher installs;
- exact existing-root reuse semantics that reject silent authentication upgrade/downgrade or signer/mechanism substitution.

## Exact-byte handoff

The acquisition verifier deliberately consumes owned candidate bytes and returns owned verified bytes. Installation uses those verified buffers rather than reopening the original candidate source while preserving the earlier verification claim.

This provides a clean TOCTOU boundary:

```text
mutable source / bounded archive
        ↓ copy/read once under source policy
CandidateFile owned bytes
        ↓ exact verify
VerifiedBundle owned bytes
        ↓ install these bytes
content-addressed local root
```

A source adapter has its own race/containment or archive-parsing problem while constructing `CandidateFile`; it solves that at its boundary. The acquisition use case does not pretend an untrusted path handle or archive entry is stable authority.

## Claim separation

The implemented path intentionally keeps these claims independent:

```text
bundle subject
    = exact distributed file-set identity

VerifiedBundle
    = exact candidate bytes match that subject

ed25519-subject-v1 PublisherAssertion
    = this exact subject was signed by this verified key identity

AuthorizedPublisher
    = explicit host policy accepted that verified assertion

installation/v1
    = exact installed bytes; publisher authentication not provided

installation/v2
    = exact installed bytes + verified-and-authorized publisher evidence

invokrum.lock/v1
    = exact resolved-composition identity
```

A valid signature is not authorization. Authorization is not freshness. Installation evidence is not semantic prompt approval. Composition lock identity is not publisher provenance.

## Deliberately absent

- remote/HTTPS candidate transport;
- registry/mirror discovery;
- clock/freshness/expiry/revocation/transparency state;
- rollback/freeze version-selection policy;
- signature mechanisms beyond the implemented `ed25519-subject-v1` adapter;
- non-Linux installation adapters;
- the final public install API/CLI delivery surface tracked in #64.

Those remain outer infrastructure/application concerns and must be introduced behind narrow contracts rather than folded into composition or distribution values.
