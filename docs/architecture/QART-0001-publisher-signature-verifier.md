# QART-0001: First concrete publisher-signature verifier

- Status: Decided
- Date: 2026-08-12
- Issue: #63
- Follow-up: #70
- Related: [ADR-0002](ADR-0002-publisher-trust-and-acquisition-boundary.md)

## Question

Which concrete signature-verification mechanism should Invokrum implement first so that an acquisition host can cryptographically authenticate the publisher of an exact immutable `invokrum.pack-bundle/v1` subject without changing the provider-neutral distribution domain or coupling deterministic composition to network, clock, certificate, or registry state?

## Decision

Implement **raw Ed25519 verification with a pinned public-key fingerprint** as the first concrete verifier adapter.

The mechanism identifier is:

```text
ed25519-subject-v1
```

The signed message is a versioned, domain-separated ASCII representation of the immutable canonical bundle subject:

```text
invokrum.publisher-signature/v1\n
sha256:<64 lowercase hexadecimal subject>\n
```

The verifier accepts exactly:

- 32 raw Ed25519 public-key bytes;
- 64 raw Ed25519 signature bytes;
- one already-derived canonical `Sha256Digest` bundle subject.

On successful strict Ed25519 verification, it produces the existing provider-neutral `PublisherAssertion` with:

- `mechanism = ed25519-subject-v1`;
- `subject = <the exact verified bundle subject>`;
- `identity["key.sha256"] = <SHA-256 of the raw 32-byte public key>`.

The host must then separately call `TrustPolicy::authorize`. Cryptographic validity is not authorization.

This QART does **not** promote a new architectural boundary to an ADR. ADR-0002 already defines the stable provider-neutral trust/acquisition boundary; this decision selects the first outer adapter within that boundary.

## Invariants carried forward

The selected verifier must preserve all of the following:

1. Signature verification covers the exact immutable bundle subject, not a mutable locator, archive filename, repository, tag, or descriptive publisher field.
2. `PublisherAssertion` is created only after cryptographic verification succeeds.
3. Candidate pack data never supplies authoritative trust policy or authorizes its own key.
4. Host authorization is explicit and occurs after verification.
5. Provider-specific parsing and cryptography remain outside `invokrum-distribution` and all composition crates.
6. Parsing is bounded and failure categories are deterministic.
7. Verification requires no network, clock, environment, ambient credentials, or trust store.
8. Freshness, revocation, rollback, installation evidence, and semantic prompt safety remain separate claims.

## Why sign a domain-separated subject message

Signing the raw transport archive would make harmless repacking part of publisher identity and would couple authentication to one archive representation. Signing arbitrary manifest bytes would duplicate canonicalization knowledge in the verifier.

The canonical bundle contract already derives one immutable SHA-256 subject from the exact installable content. The verifier therefore signs a fixed domain-separated message containing that subject. This has three useful properties:

- **exactness:** changing canonical bundle content changes the subject and invalidates the signature;
- **protocol separation:** an Ed25519 signature from another protocol cannot be silently reinterpreted as an Invokrum publisher signature over the same digest bytes;
- **adapter isolation:** the verifier consumes the domain `Sha256Digest` value and does not parse bundle JSON or archives.

The message grammar is intentionally fixed for v1 rather than assembled from optional fields.

## Identity model

The first publisher identity is an immutable public-key fingerprint:

```text
key.sha256=<64 lowercase hexadecimal SHA-256 digest of raw public-key bytes>
```

The fingerprint is derived by the verifier from the exact public key that successfully verified the signature. It is never accepted from pack metadata.

A trust rule therefore resembles:

```text
mechanism = ed25519-subject-v1
key.sha256 = <pinned fingerprint>
```

A host can authorize one or more fingerprints during key rotation by changing host-owned policy. The adapter itself does not infer replacement keys, organizations, repositories, or ownership relationships.

## Alternatives evaluated

### 1. Raw Ed25519 with pinned public-key identity — selected

**Strengths**

- Direct verification of a small, fixed message with a mature pure-Rust implementation.
- Fully offline and deterministic after key/signature acquisition.
- Stable immutable identity from the public key itself.
- Very small protocol surface: fixed key length, signature length, message grammar, and fingerprint rule.
- Straightforward golden/adversarial vectors, including RFC 8032-compatible verification vectors.
- No certificate parser, transparency-log client, registry client, timestamp service, or identity-provider semantics.
- Maps directly into the existing `VerificationMechanism` / `PublisherIdentity` / `PublisherAssertion` boundary.
- Leaves later verifier adapters free to add richer identities without changing the distribution domain.

**Costs**

- Key distribution is a host/operator responsibility.
- Rotation and revocation require explicit host-policy changes; there is no globally discoverable revocation mechanism.
- The key fingerprint identifies a signing key, not an organization, workload, repository, or CI workflow.
- Signing ergonomics need a separate release-tooling story; the verifier issue deliberately includes no private-key management.

**Rust implementation evidence**

`ed25519-dalek` 3.0.0 supports Rust 1.85 / edition 2024, matching Invokrum's current MSRV, exposes strict verification, forbids unsafe code in the crate itself, and does not require signing/key-generation, PEM, PKCS#8, serde, RNG, or hazmat features for verification.

References:

- https://docs.rs/ed25519-dalek/3.0.0/ed25519_dalek/
- https://docs.rs/crate/ed25519-dalek/3.0.0

### 2. Minisign-compatible signatures — defer

**Strengths**

- Mature cross-platform CLI and familiar release-signing workflow.
- Ed25519-based and offline.
- Small verification implementations exist; `minisign-verify` is intentionally verification-only and zero-dependency.
- Existing users can exchange `.minisig` files and public-key strings.

**Why not first**

Minisign adds a second envelope/protocol grammar: public-key encoding, signature boxes, comments/metadata, prehashed/legacy modes, and compatibility choices. Invokrum would need to decide which subset is normative and which metadata is authoritative or ignored. That is useful interoperability work, but it is not needed to prove the first publisher-authentication boundary.

A later `minisign-v1` adapter can normalize a successfully verified Minisign key to the same provider-neutral identity model without changing `TrustPolicy`.

References:

- https://github.com/jedisct1/minisign
- https://docs.rs/minisign-verify/0.2.5/minisign_verify/

### 3. Sigstore identity/provenance verification — defer

**Strengths**

- Strong workload and CI identity model.
- Can bind repository/workflow/OIDC claims and transparency evidence rather than only a long-lived key.
- Well suited to public release provenance and keyless signing ecosystems.

**Why not first**

The security and operational surface is materially larger: certificates, OIDC identities, transparency-log evidence, trust roots, validity intervals, offline bundles, and freshness/revocation policy. The current Rust `sigstore` crate explicitly describes itself as experimental and its API as subject to change. That surface should be added only after the provider-neutral signed-install path is proven with a small verifier.

Sigstore remains a high-value future adapter because its identity attributes can map naturally into the existing normalized `PublisherIdentity` structure.

References:

- https://docs.sigstore.dev/
- https://docs.rs/sigstore/0.14.0/sigstore/

### 4. X.509/CMS — reject for the first implementation

Certificate/CMS verification would be justified by a concrete enterprise interoperability requirement, but none currently exists for Invokrum. Introducing certificate path construction, extension parsing, trust-anchor semantics, validity periods, and revocation before such a requirement would add security surface without improving the immediate local distribution model.

This option can be reconsidered when a host integration requires existing enterprise PKI.

## Decision matrix

Scoring: 5 is best fit for the first Invokrum verifier; 1 is poorest. Scores represent project fit, not absolute security quality.

| Criterion | Raw Ed25519 | Minisign | Sigstore | X.509/CMS |
| --- | ---: | ---: | ---: | ---: |
| Exact canonical-subject verification | 5 | 5 | 5 | 5 |
| Fully offline verification | 5 | 5 | 4 | 5 |
| Stable policy identity | 4 | 4 | 5 | 5 |
| Rotation/revocation model | 2 | 2 | 4 | 5 |
| Small parser/dependency surface | 5 | 4 | 1 | 2 |
| Deterministic failure categories | 5 | 4 | 3 | 3 |
| Cross-platform Rust implementation | 5 | 4 | 4 | 4 |
| CI/release ergonomics today | 3 | 5 | 5 | 3 |
| Published/golden-vector testability | 5 | 5 | 4 | 4 |
| Provider-neutral boundary fit | 5 | 5 | 5 | 5 |
| Immediate Invokrum interoperability need | 4 | 3 | 3 | 1 |
| **First-verifier fit** | **48/55** | **46/55** | **43/55** | **42/55** |

The close numerical totals do not imply equivalent complexity. The deciding factor is that raw Ed25519 proves the required authentication/authorization separation with the smallest new protocol and dependency surface. Richer identity systems remain additive adapters.

## Concrete adapter boundary

The implementation belongs in a dedicated outer crate, conceptually:

```text
invokrum-verifier-ed25519
    │
    ├── ed25519-dalek          concrete cryptography
    ├── invokrum-digest        public-key SHA-256 fingerprint
    └── invokrum-distribution  normalized assertion values
             │
             ▼
       PublisherAssertion
             │
             ▼
       TrustPolicy::authorize  host-owned authorization
```

Forbidden dependency direction:

```text
invokrum-distribution ──X──> invokrum-verifier-ed25519
composition crates     ──X──> invokrum-verifier-ed25519
```

The adapter performs no filesystem, network, archive, installation, clock, environment, key discovery, or credential behavior.

## Failure categories

The concrete adapter should expose a small stable error enum, at least:

- malformed public-key length;
- invalid public key;
- malformed signature length;
- signature verification failed;
- normalized assertion construction invariant failure.

Provider-library error text must not become the public machine contract.

Authorization failures continue to use `TrustError` and are intentionally not verifier errors.

## Golden and adversarial test strategy

The first implementation must include:

1. a published/RFC-compatible Ed25519 verification vector to detect crypto API misuse;
2. an Invokrum-specific golden vector for the exact domain-separated subject message;
3. a golden `key.sha256` fingerprint;
4. successful verification producing the exact expected `PublisherAssertion`;
5. one-bit mutation of subject, signature, and public key;
6. malformed 31/33-byte keys and 63/65-byte signatures;
7. cross-subject substitution;
8. valid signature by an unauthorized key, proving `TrustPolicy` denial occurs after crypto success;
9. architecture checks preventing concrete-crypto dependency leakage into domain/composition crates.

Test-only signing may be used to construct fixtures, but production code should expose verification only. Prefer fixed published/golden bytes for the core conformance tests so normal tests do not require randomness or private-key lifecycle code.

## Key rotation, revocation, freshness, and rollback

The first adapter intentionally does not solve these policy problems.

- **Rotation:** host policy may temporarily authorize old and new key fingerprints.
- **Revocation:** removing a fingerprint from current host policy denies new authorization. Historical installation evidence remains historical evidence.
- **Freshness:** no assertion is made about when the signature was created or whether the key is currently valid elsewhere.
- **Rollback:** subject signatures authenticate an exact subject but do not establish monotonic version/sequence semantics.

These claims require separate host/acquisition policy and must not be inferred from a valid Ed25519 signature.

## Installation evidence boundary

A successful verifier result establishes only:

> this exact bundle subject was signed by the private key corresponding to this public-key fingerprint.

`TrustPolicy::authorize` separately establishes:

> this host policy allows that verified key identity for this verification mechanism.

The installer may later persist bounded authenticated-publisher evidence after both steps succeed. Existing `publisher_authentication: not-provided` behavior remains correct for local/digest-only installs and must not change merely because the verifier crate exists.

## Non-goals

- private-key generation, storage, encryption, HSM/KMS integration, or signing CLI;
- Minisign file/envelope compatibility;
- Sigstore/Fulcio/Rekor verification;
- X.509/CMS;
- TOFU;
- key discovery from packs, registries, URLs, repositories, or environment variables;
- network transport or registry discovery;
- timestamping, certificate validity, transparency, revocation, freshness, or rollback policy;
- semantic prompt safety;
- changing composition behavior.

## Follow-up implementation

Issue #70 implements the bounded adapter and tests described above. Remote transport and registry work remain blocked until the verifier, host authorization, authenticated installation evidence, and signed-install path have been adversarially proven together.