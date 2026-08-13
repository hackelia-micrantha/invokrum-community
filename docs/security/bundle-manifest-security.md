# Bundle manifest security contract

**Status:** Bundle identity, local directory/archive acquisition, exact candidate verification, concrete Ed25519 verification, host authorization, and Linux installation evidence are implemented. Remote transport, registry discovery, freshness/revocation/transparency, and rollback-selection policy remain planned.

The bundle manifest is the immutable distributed-content subject used by digest-pinned and publisher-authenticated installation flows. It does not itself authenticate a publisher and it does not authorize execution.

## Implemented controls

- exact `invokrum.pack-bundle/v1` format identifier;
- exact `sha256` digest algorithm identifier;
- lowercase 64-character digest validation;
- portable bounded relative paths;
- deterministic path sorting;
- duplicate-path rejection;
- explicit enumerated entry point;
- bounded file count, per-file size, aggregate expanded size, and manifest bytes;
- duplicate JSON object-key rejection before semantic decoding;
- unknown-field rejection;
- exact canonical re-encoding requirement;
- SHA-256 subject digest over canonical manifest bytes;
- bounded Linux local-directory candidate loading;
- bounded uncompressed POSIX ustar candidate ingestion without filesystem extraction;
- exact candidate path-set, length, and SHA-256 verification;
- concrete `ed25519-subject-v1` verification over the exact immutable subject;
- provider-neutral publisher assertions created only after concrete verifier success;
- host-owned trust rules that cannot be supplied by pack metadata;
- fail-closed subject mismatch and unrecognized-publisher decisions;
- Linux private quarantine, staged re-verification, and content-addressed atomic promotion;
- deterministic `invokrum.installation/v1` evidence for installs without publisher authentication, carrying exactly `publisher_authentication: not-provided`;
- structured `invokrum.installation/v2` evidence for verified-and-authorized publisher installs;
- exact existing-root evidence comparison that rejects silent provenance upgrade, downgrade, or signer substitution.

## Threats reduced by the implemented path

The current contracts remove ambiguity and authority confusion across local installation:

- a signer cannot sign one file list while an installer interprets another ordering;
- duplicate JSON keys cannot create parser-dependent bundle meaning;
- a pack cannot add a self-declared publisher field and use it as authority;
- a cryptographically valid assertion cannot authorize a different bundle subject;
- a valid but host-unrecognized signer is denied;
- supported directory/ustar candidates cannot introduce undeclared paths, links, special entries, traversal, logical collisions, or content that does not match the canonical manifest;
- installation consumes owned verified bytes rather than reopening the candidate source;
- existing content-addressed roots cannot silently change v1/v2 authentication evidence;
- mutable repository/tag/registry names are absent from the immutable bundle identity.

## Threats not yet mitigated

The following remain **Planned** or delegated under the accepted publisher-trust architecture:

- remote/network candidate transport;
- registry discovery and mirror selection;
- freshness, expiry, revocation, transparency-log integration, and rollback/freeze selection policy;
- signature mechanisms other than the implemented `ed25519-subject-v1` adapter;
- non-Linux installation adapters;
- semantic prompt safety and runtime capability authorization.

The explicit public install API/CLI delivery surface is also not yet exposed; it is tracked in #64. That delivery gap does not make the underlying local acquisition/authentication/install use cases unimplemented.

## Claim separation

```text
bundle subject digest
    = exact distributed file-set identity

ed25519-subject-v1 verification
    = this exact subject was signed by this verified key identity

publisher assertion + host TrustPolicy
    = verified signer evidence satisfied explicit host authorization

invokrum.installation/v1
    = installed exact bytes; publisher_authentication: not-provided

invokrum.installation/v2
    = installed exact bytes with verified-and-authorized publisher evidence

invokrum.lock/v1
    = exact resolved-composition identity
```

These claims remain intentionally independent. A valid signature is not authorization; authorization is not freshness; installation evidence is not prompt semantic approval; and a composition lock is not a publisher signature.

See [publisher trust and signed pack installation](publisher-trust.md), [ADR-0002](../architecture/ADR-0002-publisher-trust-and-acquisition-boundary.md), [pack bundle format v1](../bundle-format-v1.md), and the [threat model](threat-model.md).
