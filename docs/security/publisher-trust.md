# Publisher trust and signed pack installation

**Status:** Local authenticated installation is implemented. Bundle identity, host trust policy, bounded directory/archive acquisition, exact candidate verification, Ed25519 subject verification, authenticated installation evidence, and Linux content-addressed promotion are implemented; remote acquisition, freshness/revocation, rollback selection, and registry integration remain planned.  
**Scope:** Pack-bundle identity, publisher assertions and host trust policy, local/archive and future remote candidates, signatures or digest pins, quarantine installation, immutable local promotion, update/freshness policy, and the handoff to existing offline composition.  
**Decision:** [ADR-0002](../architecture/ADR-0002-publisher-trust-and-acquisition-boundary.md)

Invokrum can establish and bind three independent facts for a supported local installation: exact candidate bytes match an explicit immutable subject, a concrete verifier established a publisher assertion for that exact subject, and host-owned policy authorized the normalized publisher identity. The Linux installer then records the authenticated assertion together with the verified installed-tree identity without moving cryptography or trust policy into filesystem code.

Remote transport, registry discovery, and current-trust/freshness state are still outside this path.

## Implementation status

| Capability | Status | Evidence |
| --- | --- | --- |
| `invokrum.pack-bundle/v1` domain and hard limits | **Implemented** | `invokrum-distribution`, unit tests |
| Strict canonical bundle JSON and JSON Schema | **Implemented** | `invokrum-distribution-json`, schema/golden/adversarial tests |
| SHA-256 immutable bundle-subject derivation | **Implemented** | `invokrum-digest`, published vectors, golden subject |
| Normalized verification mechanism and publisher identity | **Implemented** | `invokrum-distribution` tests |
| Host-owned trust rules and deterministic subject/identity matching | **Implemented** | `TrustPolicy::authorize` tests |
| Linux already-local candidate tree loading | **Implemented** | `invokrum-acquisition-linux`, adversarial filesystem tests |
| Bounded uncompressed POSIX ustar archive ingestion | **Implemented** | `invokrum-acquisition-archive`, adversarial archive tests |
| Offline expected-subject and exact-byte verification | **Implemented** | `invokrum-acquisition`, bounded verification tests |
| Linux private quarantine and content-addressed installation | **Implemented** | `invokrum-install`, `invokrum-install-linux`, end-to-end tests |
| Concrete Ed25519 subject-signature verification | **Implemented** | `invokrum-verifier-ed25519`, RFC/protocol golden and adversarial tests |
| Authenticated publisher assertion bound into installation evidence | **Implemented** | `AuthorizedPublisher`, `invokrum.installation/v2`, local/archive integration tests |
| Remote/network candidate transport | **Planned** | no HTTP/network acquisition adapter |
| Freshness, revocation, transparency, rollback policy | **Planned** | no clock/network/update state |
| Registry discovery | **Planned** | no registry integration |

Only capabilities marked **Implemented** are current runtime guarantees.

## Claim pipeline

Publisher trust remains an acquisition/install concern, not a composition feature.

```text
local directory or bounded ustar
              |
              v
      exact candidate bytes
              |
              v
       verify_candidate --------------------+
              |                              |
              v                              |
        VerifiedBundle                       |
                                             |
raw key/signature -> concrete verifier       |
                         |                    |
                         v                    |
                PublisherAssertion           |
                         |                    |
                         v                    |
                  host TrustPolicy           |
                         |                    |
                         v                    |
                 AuthorizedPublisher         |
                         |                    |
                         +----------+---------+
                                    |
                                    v
                         Linux quarantine/CAS
                                    |
                  +-----------------+-----------------+
                  |                                   |
                  v                                   v
     invokrum.installation/v1           invokrum.installation/v2
      auth = not-provided               verified + authorized
                  |                                   |
                  +-----------------+-----------------+
                                    |
                                    v
                         existing composition
```

No stage silently implies another. A valid signature is not authorization. Authorization is not installation. An installed root is not fresh merely because it was authenticated when installed.

## Claims are deliberately separate

| Claim | Evidence | Owner / status |
| --- | --- | --- |
| These local bytes compose deterministically | `invokrum.lock/v1`, manifest, output digest | composition/integrity — implemented |
| These distributed bytes have this immutable identity | canonical `invokrum.pack-bundle/v1` + subject digest | distribution — implemented |
| This exact subject was signed by this Ed25519 key identity | `ed25519-subject-v1` + `key.sha256` | verifier adapter — implemented |
| This verified assertion satisfies this explicit host rule | `PublisherAssertion` + `TrustPolicy` | distribution domain — implemented |
| These installed bytes match this expected subject | installed-tree evidence | installer — implemented |
| This installed root was produced with this verified and authorized publisher evidence | `invokrum.installation/v2` | install orchestration/store — implemented |
| Publisher authentication was not supplied for this installed root | `invokrum.installation/v1`, `publisher_authentication: not-provided` | installer — implemented |
| This publisher is still trusted/fresh now | current host freshness/revocation policy | host/acquisition policy — planned |
| These prompt instructions are safe to execute | approvals, semantic review, sandbox/capability policy | runtime host — delegated |

## Trust policy

Trust policy is explicit caller or host configuration. It is never read from candidate pack data as authority.

`invokrum-distribution` defines provider-neutral values:

- verification-mechanism identifier;
- immutable SHA-256 subject;
- bounded normalized publisher-identity attributes;
- explicit host-owned allow rules;
- deterministic `SubjectMismatch` and `PublisherNotAllowed` failures.

The concrete `invokrum-verifier-ed25519` adapter creates `PublisherAssertion` only after strict verification of the versioned subject message. `invokrum-install::authorize_publisher` then applies `TrustPolicy` and creates opaque `AuthorizedPublisher` only after policy success.

Authorization requires:

1. assertion subject equals the expected canonical bundle subject; and
2. assertion mechanism and normalized identity satisfy an explicit host-owned trust rule.

A cryptographically valid assertion from an unrecognized identity is denied before candidate loading or store mutation in the authenticated workflow.

### Exact digest pin

An explicit expected subject authenticates exact bytes relative to the trusted channel that supplied that digest. It does not identify a human or organizational publisher by itself.

Directory and supported ustar candidates are normalized to exact owned bytes, then checked against the same canonical manifest/subject contract before installation.

### Ed25519 publisher assertion

The first concrete mechanism is `ed25519-subject-v1`. It verifies exactly:

```text
invokrum.publisher-signature/v1\n
sha256:<64 lowercase hexadecimal subject>\n
```

against a raw 32-byte Ed25519 public key and raw 64-byte signature. Successful strict verification normalizes:

```text
key.sha256=<SHA-256 of raw public-key bytes>
```

Weak keys, malformed lengths, invalid signatures, and cross-subject substitution fail closed. Provider error text does not become the domain contract.

### Authenticated installation evidence

Unsigned/digest-only installs retain `invokrum.installation/v1` and exactly:

```text
publisher_authentication: not-provided
```

Authenticated installs use `invokrum.installation/v2`, whose structured publisher evidence contains:

- `status = verified-and-authorized`;
- exact verification mechanism;
- exact bundle subject;
- normalized publisher identity attributes.

The v2 record is constructible only from `AuthorizedPublisher` plus the same `VerifiedBundle` being installed. The Linux store remains cryptography-blind.

Existing CAS subject roots require exact evidence bytes for reuse. Consequently an existing v1 root cannot be silently upgraded to v2, an existing v2 root cannot be silently downgraded to v1, and a v2 root cannot silently change signer identity/mechanism while preserving the same subject path.

See [Linux local installation](linux-local-installation.md).

### Trust-on-first-use

TOFU is not an Invokrum default. No TOFU persistence mechanism is implemented. A future host that adds TOFU must label it explicitly and must not present it as equivalent to preconfigured publisher trust.

## Publisher identity is not pack metadata

Pack-declared authors, repository names, URLs, branches, releases, tags, or registry names are descriptive inputs only. They are never sufficient publisher authentication or authorization.

## Security requirements

The following are normative:

- Composition performs no implicit acquisition or publisher verification.
- Trust policy is host-owned and cannot be relaxed by untrusted pack data.
- Mutable locators are discovery inputs, never final security identities.
- Supported archive ingestion returns bounded owned candidate bytes and never treats archive paths or metadata as authority.
- Concrete signature verification must precede cryptographic publisher evidence.
- Host authorization is explicit and separate from signature validity.
- Authenticated installation requires the authorized assertion and verified candidate to identify the same immutable subject.
- Promotion remains atomic into the existing content-addressed protected root.
- Existing subject roots require exact installation evidence; provenance is not rewritten in place.
- Freshness and revocation policy is checked outside deterministic composition.

## Delivery order

1. **Done:** canonical `invokrum.pack-bundle/v1` and immutable subject.
2. **Done:** normalized publisher assertion and host trust-policy values.
3. **Done:** already-local Linux candidate verification.
4. **Done:** Linux private quarantine/content-addressed installation with v1 evidence.
5. **Done:** bounded POSIX ustar ingestion.
6. **Done:** concrete `ed25519-subject-v1` verification.
7. **Done:** bind verified + host-authorized publisher evidence into authenticated v2 installation without changing unsigned v1 semantics.
8. **Next delivery layer:** expose authenticated install through the explicit API/CLI contract (#64).
9. **Later:** remote transport, registry discovery, and freshness/revocation/rollback policy after the signed local/archive path remains adversarially proven.
