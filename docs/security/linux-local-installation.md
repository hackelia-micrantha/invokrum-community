# Linux local candidate loading and installation

**Status:** Implemented Linux local-directory and authenticated installation paths.  
**Scope:** Already-local candidate directories, exact-byte verification, optional externally verified and host-authorized publisher evidence, private quarantine, deterministic installation evidence, content-addressed promotion, and exact reuse.  
**Related:** [publisher trust](publisher-trust.md), [bounded archive ingestion](archive-candidate-ingestion.md), [ADR-0002](../architecture/ADR-0002-publisher-trust-and-acquisition-boundary.md).

Invokrum's Linux installer materializes only already-owned `VerifiedBundle` bytes into a protected content-addressed root. Publisher authentication is a separate application-layer claim: an injected verifier must first establish a normalized `PublisherAssertion`, host-owned `TrustPolicy` must authorize that assertion for the exact subject, and only then can the installer persist authenticated installation evidence.

No installation path performs network access, registry discovery, credential loading, TOFU, clock/freshness checks, revocation, signing, or private-key management.

## Architecture

```text
validated BundleManifest + expected subject
                |
                +-----------------------------+
                |                             |
                v                             v
         CandidateLoader               PublisherVerifier port
                |                             |
                v                             v
        exact candidate bytes            verified assertion
                |                             |
                v                             v
         verify_candidate              host TrustPolicy
                |                             |
                v                             v
          VerifiedBundle              AuthorizedPublisher
                |                             |
                +---------------+-------------+
                                |
                                v
                       invokrum-install
                  authenticated orchestration
                                |
                                v
                   AuthenticatedVerifiedBundleStore
                                |
                                v
                   invokrum-install-linux
                  one quarantine/CAS policy
                                |
                                v
                    sha256/<bundle-subject>
                                |
                                v
                     existing composition
```

The unauthenticated path uses the same application/store components without `AuthorizedPublisher` and preserves its existing behavior.

`invokrum-install` is provider- and transport-neutral. It owns sequencing and narrow DI ports only. It has no concrete Ed25519, filesystem, archive, network, process, environment, clock, registry, credential, or signing dependency. `invokrum-install-linux` owns Linux filesystem mutation and deterministic installation-record serialization; it does not verify signatures or choose trust policy.

## Authentication/authorization boundary

`PublisherVerifier` is an outer application port. An implementation may return `PublisherAssertion` only after actually verifying provider-specific cryptographic evidence for the supplied immutable subject.

`authorize_publisher` then applies explicit host-owned `TrustPolicy`. Only after both steps succeed does the application construct opaque `AuthorizedPublisher` evidence. The public type exposes the already-authorized subject, verification mechanism, and normalized identity, but has no public constructor.

The authenticated workflow fails before candidate loading or store mutation when:

- manifest subject and expected subject disagree;
- publisher verification fails;
- the assertion subject differs from the expected subject;
- the assertion mechanism/identity is not explicitly authorized by host policy.

Candidate bytes are then loaded and verified normally. A final defensive subject-equality check requires `AuthorizedPublisher.subject == VerifiedBundle.subject` before store handoff.

Cryptographic validity is therefore not authorization, and authorization is not installation.

## Candidate-root contract

`invokrum-acquisition-linux::LinuxLocalCandidateSource` remains the single Linux directory-reading policy. `invokrum-install-linux::LinuxCandidateLoader` is a thin implementation of `CandidateLoader` and contains no second traversal policy.

The canonical Linux source:

- validates a non-symlink root and revalidates device/inode identity;
- bounds traversal depth and visible-entry collection;
- enumerates opened directory descriptors through `/proc/self/fd`;
- opens declared files with no-follow semantics;
- rejects unrepresentable names, case-fold collisions, links, special files, device crossings, unrelated files/directories, and replacement races within the supported host assumptions;
- bounds per-file reads;
- returns exact owned `CandidateFile` bytes.

`verify_candidate` retains subject equality, path-set equality, file-count/aggregate limits, byte-length checks, and SHA-256 content verification.

The bounded POSIX ustar adapter documented in [archive-candidate-ingestion.md](archive-candidate-ingestion.md) produces the same owned `CandidateFile` representation and therefore converges on the same authenticated installation semantics. Archive parsing is not duplicated in the installer.

## Verified-byte handoff

After candidate verification, installation receives only:

- immutable bundle subject;
- bundle entry point;
- deterministic ordered file records;
- exact owned bytes and verified file digests.

The store never reopens the original directory or archive. A later transformation is a different artifact and requires a different verified identity.

## Store layout

The host supplies an existing protected store root. The Linux adapter pins that root by canonical path/device/inode, requires it not to be group/world writable, and creates/verifies private internal directories:

```text
<store>/
  .locks/                         0700
    <subject>.lock                0600
  .staging/                       0700
    <subject>.<attempt>/          0700
  sha256/                         0700
    <subject>/                    0700
      <verified bundle files>     0600
      .invokrum-installation.json 0600
```

The final root identity is always:

```text
sha256/<bundle-subject>
```

Authentication does not change the content-addressed root name. It changes only which exact installation-evidence record is valid for that root.

## Quarantine and promotion

Both unsigned and authenticated installation use the same code path:

1. acquire the per-subject `0600` lock;
2. create a private same-filesystem staging directory;
3. materialize only verified owned bundle bytes using create-new semantics;
4. reopen staged bundle files through the canonical Linux acquisition source;
5. run `verify_candidate` again against the same subject;
6. write deterministic installation evidence;
7. reread and byte-compare that evidence;
8. recheck private file/directory modes;
9. atomically rename staging to `sha256/<subject>`.

No signature, trust-policy, or provider logic exists in this filesystem sequence.

## Installation evidence versions

The evidence path remains:

```text
.invokrum-installation.json
```

All records are deterministic compact JSON followed by one newline and remain under the independent 2 MiB installer-evidence bound.

### `invokrum.installation/v1` — authentication not provided

The existing unsigned/digest-only path remains byte-for-byte compatible with v1 and records:

```json
"publisher_authentication":"not-provided"
```

The record also binds the bundle subject, entry point, `sha256/<subject>` root identity, `invokrum.installed-tree/v1` digest, and ordered per-file path/length/SHA-256 records.

This record proves exact materialization of an expected immutable subject. It does not authenticate a publisher.

### `invokrum.installation/v2` — verified and authorized publisher

Authenticated installation uses v2 because v1's `publisher_authentication` field is a single literal string; introducing a packed sub-grammar into v1 would make compatibility and canonicalization ambiguous.

V2 preserves the same bundle/tree evidence and replaces the authentication literal with structured normalized evidence:

```json
{
  "format":"invokrum.installation/v2",
  "bundle_subject":"<subject>",
  "entry_point":"pack.yaml",
  "root_identity":"sha256/<subject>",
  "installed_tree_digest_format":"invokrum.installed-tree/v1",
  "installed_tree_digest":"<digest>",
  "publisher_authentication":{
    "status":"verified-and-authorized",
    "mechanism":"ed25519-subject-v1",
    "subject":"<same subject>",
    "identity":{
      "key.sha256":"<normalized verified key fingerprint>"
    }
  },
  "files":[...]
}
```

The concrete identity attributes are already normalized by the verifier/domain boundary. The Linux store does not infer identity from pack metadata and cannot construct `AuthorizedPublisher` itself.

## Exact reuse and provenance immutability

An existing `sha256/<subject>` root is reopened through the secure local acquisition source and checked for:

- exact bundle file set, lengths, and digests;
- exact installation-record bytes;
- exact private modes and regular-file/link-count/device requirements.

Only an exact match is reusable. This intentionally makes provenance state immutable with the CAS root:

- v1 -> same v1 evidence: reusable;
- v2 -> same v2 publisher evidence: reusable;
- existing v1 + later authenticated request: `ExistingInstallationMismatch`;
- existing v2 + later unsigned request: `ExistingInstallationMismatch`;
- existing v2 + different mechanism/identity for the same subject: `ExistingInstallationMismatch`.

The store never upgrades, downgrades, rewrites, repairs, or substitutes publisher provenance in place. If policy requires a different provenance state, the host must make that lifecycle explicit rather than presenting old evidence as newly authenticated.

## Composition handoff

The final subject directory remains an ordinary local pack root plus one reserved installer metadata file. Existing schema/filesystem/core composition code consumes the installed bundle exactly as before; production composition crates have no acquisition, installer, trust-policy, or concrete verifier dependency.

Integration coverage proves both directory and bounded-ustar candidates can reach the same authenticated store contract, and archive-installed roots still compose through the unchanged schema/`LocalPackSource`/core path.

## Host preconditions

The Linux implementation assumes:

- stable mount namespace during loading and installation;
- protected candidate/store parents when resistance to privileged replacement is required;
- functional `/proc/self/fd` inspection;
- one filesystem for staging and final `sha256/` promotion;
- trust policy and cryptographic verification inputs are supplied independently from candidate pack authority.

The per-subject lock is a serialization primitive, not liveness/freshness evidence. Stale-lock recovery remains explicit host policy.

## Security claims

Implemented claims:

- exact candidate bytes match one explicit expected bundle subject;
- `ed25519-subject-v1` or another future injected verifier may establish a normalized publisher assertion without entering installer/domain composition code;
- host `TrustPolicy` must explicitly authorize that assertion for the exact subject before `AuthorizedPublisher` exists;
- authenticated v2 evidence binds that authorized assertion to the same verified bundle subject and installed-tree identity;
- unsigned v1 evidence continues to state `publisher_authentication: not-provided`;
- CAS reuse requires exact bytes, modes, and provenance evidence, preventing silent authentication upgrade/downgrade or signer substitution.

Not claimed:

- current key freshness, revocation, transparency, timestamping, or rollback ordering;
- semantic safety of prompt content;
- remote retrieval or registry authenticity;
- TOFU;
- private-key/signing security.

Those remain separate host/acquisition concerns under [publisher trust](publisher-trust.md).
