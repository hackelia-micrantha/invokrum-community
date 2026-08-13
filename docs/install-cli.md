# Local install CLI

**Status:** Implemented Linux delivery for local directories and bounded uncompressed POSIX ustar candidates.  
**Security identity:** immutable `sha256:<bundle-subject>` only.  
**Authentication modes:** explicit digest-only (`invokrum.installation/v1`) or explicit `ed25519-subject-v1` + host trust (`invokrum.installation/v2`).

Invokrum exposes one explicit local installation command over the existing acquisition/authentication/install use cases:

```text
invokrum install <candidate> \
  --bundle-manifest <canonical-bundle.json> \
  --subject sha256:<immutable-subject> \
  [--candidate-format directory|ustar] \
  [--store <path>] \
  [--format human|json]
```

Authenticated publisher installation adds all three of these required inputs:

```text
  --publisher-public-key <raw-32-byte-key> \
  --publisher-signature <raw-64-byte-signature> \
  --trusted-key-sha256 sha256:<trusted-key-fingerprint>
```

The candidate path, bundle-manifest path, key path, and signature path are caller-selected inputs. They never become package or publisher authority. Installation requires a caller-supplied immutable bundle subject. Authenticated mode additionally requires a caller-supplied host trust fingerprint; candidate metadata cannot select or relax that rule.

## Candidate formats

`--candidate-format directory` is the default. It uses the existing fail-closed Linux directory acquisition adapter and therefore rejects links, hard links, unsupported files, root escapes, undeclared content, unstable identities, and other states prohibited by that adapter.

`--candidate-format ustar` reads the candidate under a bounded outer file-read limit and hands the exact bytes to `UstarCandidateSource`. Only the documented uncompressed POSIX ustar profile is accepted. No archive content is extracted to the host filesystem before verification.

Both formats converge on `CandidateLoader` and the same exact subject/path/length/SHA-256 verification use case. The CLI does not implement a second bundle verifier or installer.

## XDG layout

For ordinary user-scoped Linux use, the default durable store is:

```text
${XDG_DATA_HOME:-$HOME/.local/share}/invokrum/store
```

More precisely:

- a non-empty absolute `XDG_DATA_HOME` resolves to `$XDG_DATA_HOME/invokrum/store`;
- otherwise a non-empty absolute `HOME` resolves to `$HOME/.local/share/invokrum/store`;
- if neither provides an absolute base, the command requires explicit `--store <path>` instead of guessing.

The broader convention remains:

```text
${XDG_CONFIG_HOME:-$HOME/.config}/invokrum/       # host configuration / trust policy
${XDG_DATA_HOME:-$HOME/.local/share}/invokrum/
  packs/                                           # optional mutable source-pack convention
  store/sha256/<subject>/                          # immutable Invokrum-managed installs
${XDG_CACHE_HOME:-$HOME/.cache}/invokrum/          # disposable acquisition cache if added later
${XDG_STATE_HOME:-$HOME/.local/state}/invokrum/    # operational state/history if added later
```

`packs/` is not trusted by location. A candidate may live anywhere the caller explicitly selects.

## Digest-only mode

When no publisher options are supplied, the command runs the existing `install_candidate` workflow. Success produces `invokrum.installation/v1` evidence containing exactly:

```text
publisher_authentication: not-provided
```

The subject pin authenticates exact bundle bytes only relative to the trusted channel that supplied that digest. It does not identify a publisher.

## Authenticated Ed25519 mode

Authenticated mode requires all of:

- `--publisher-public-key`: raw 32-byte Ed25519 public key;
- `--publisher-signature`: raw 64-byte signature;
- `--trusted-key-sha256`: exact SHA-256 fingerprint that the host explicitly allows.

The CLI builds one explicit host `TrustPolicy` rule for `ed25519-subject-v1` and `key.sha256=<trusted fingerprint>`, then calls the existing `install_authenticated_candidate` orchestration.

**A valid signature is not authorization.** Cryptographic verification establishes a publisher assertion; the explicit host `TrustPolicy` must authorize that assertion for the exact subject before installation can claim publisher authentication.

Ordering is security-significant:

```text
canonical bundle subject
        ↓
Ed25519 verification
        ↓
explicit host TrustPolicy authorization
        ↓
candidate loading
        ↓
exact content verification
        ↓
store creation / installation
```

A malformed signature or cryptographically valid but unauthorized key therefore fails before candidate I/O or store mutation. Signature validity alone never grants authorization.

Successful authenticated installation produces `invokrum.installation/v2` evidence with normalized `verified-and-authorized` publisher evidence. The CLI does not accept TOFU, a candidate-declared key, or a mutable alias as trust policy.

## Result contract

Human output reports the immutable subject, installed root, installation record, reuse state, and authentication status. Authenticated results additionally report the verifier mechanism and normalized trusted key fingerprint.

`--format json` uses the existing `invokrum.cli/v1` envelope and returns a stable object such as digest-only:

```json
{
  "format": "invokrum.cli/v1",
  "command": "install",
  "installation_format": "invokrum.installation/v1",
  "subject": "sha256:<subject>",
  "root": "<installed-root>",
  "record": "<installation-record>",
  "reused": false,
  "publisher_authentication": "not-provided"
}
```

or authenticated:

```json
{
  "format": "invokrum.cli/v1",
  "command": "install",
  "installation_format": "invokrum.installation/v2",
  "subject": "sha256:<subject>",
  "root": "<installed-root>",
  "record": "<installation-record>",
  "reused": false,
  "publisher_authentication": {
    "status": "verified-and-authorized",
    "mechanism": "ed25519-subject-v1",
    "subject": "<subject>",
    "identity": {
      "key.sha256": "<fingerprint>"
    }
  }
}
```

A second exact installation reuses the same content-addressed root only when the installed tree and exact v1/v2 evidence match. Existing roots are never silently upgraded, downgraded, or assigned a different signer.

## Store creation

Store creation is deliberately lazy. The CLI does not create or mutate the store until the relevant workflow has passed its prior verification/authorization stages.

For a missing Linux store root, the final selected store directory is created with private `0700` permissions before opening `LinuxInstallStore`. Installer-owned files/evidence retain the store adapter's existing private permissions and atomic promotion semantics.

## Failure mapping

The command preserves existing CLI error categories:

- argument-shape and digest syntax failures → usage (`2`);
- candidate/archive/local input failures → input (`3`);
- subject/content/signature/authorization failures → validation (`4`);
- installation-store failures → output (`6`);
- internal serialization/invariant failures → internal (`7`).

Errors use stderr and never print candidate contents, private signing material, or secret values.

## Non-goals

This surface does not provide:

- HTTP/HTTPS acquisition;
- registry or mirror discovery;
- `latest`, channels, tags, or mutable package-selection authority;
- friendly-name security identities;
- trust-on-first-use;
- hidden key or trust-policy discovery;
- publisher signing/private-key management;
- freshness, expiry, revocation, transparency, or rollback/freeze selection policy;
- compressed archive support;
- non-Linux installation.

Those capabilities remain outside this explicit local installation boundary.
