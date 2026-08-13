# Roadmap

Invokrum is being built from architecture and compatibility contracts outward. Milestones are intentionally capability-oriented rather than date-based.

## v0.1 — deterministic local composition

- [x] Accept the mechanism-versus-policy architecture boundary.
- [x] Implement typed pack, overlay, class, profile, and rule models.
- [x] Publish the first versioned YAML/JSON schema.
- [x] Validate references, cardinality, compatibility, and pack-relative paths.
- [x] Resolve deterministic overlay order.
- [x] Render canonical context bytes.
- [x] Emit stable inspection JSON and diagnostics.
- [x] Add hashes, resolved manifests, lockfiles, verification, and structural diffing.
- [x] Provide the initial CLI.
- [x] Add a minimal example pack.
- [x] Establish CI and reproducible prerelease artifacts.
- [x] Publish and independently verify `v0.1.0`.
- [x] Prove independent subprocess consumption with the reference host.

Anthesis conformance is not a v0.1 release gate. Invokrum remains independently useful and does not make Anthesis policy part of its public model.

## v0.2 — distribution, integration, and reference-consumer validation

Completed distribution/security slices:

- [x] exercise the public subprocess JSON contract through an independent reference host;
- [x] accept the publisher-trust and acquisition boundary without adding network access to composition;
- [x] define `invokrum.pack-bundle/v1`, canonical bundle identity, and golden vectors;
- [x] define normalized publisher assertions and host-owned trust-policy values;
- [x] verify exact already-local Linux directory candidates against an explicit immutable subject;
- [x] install verified Linux candidates through private quarantine into a content-addressed store;
- [x] persist deterministic `invokrum.installation/v1` evidence and prove installed-root handoff into ordinary offline composition;
- [x] add bounded uncompressed POSIX ustar candidate ingestion (#62);
- [x] complete QART for the first concrete publisher-signature verifier (#63);
- [x] implement strict `ed25519-subject-v1` verification (#70);
- [x] bind concrete verifier success to explicit host `TrustPolicy` authorization;
- [x] bind verified-and-authorized publisher evidence into `invokrum.installation/v2` without changing unsigned v1 semantics (#72/#73);
- [x] reject existing-root provenance upgrade/downgrade and signer/mechanism substitution in place;
- [x] establish the private-canonical/public-community repository topology and fail-closed public export policy (#75/#76);
- [x] reconcile public capability/status documentation with the implemented authenticated-install path (#77/#78).

The supported local directory/archive installation path can now establish exact content identity and, in authenticated mode, concrete Ed25519 publisher evidence plus explicit host authorization for the same immutable subject. That claim remains separate from freshness/revocation and from prompt semantic safety.

### Current next-up queue

1. [ ] **P1 — explicit install API/CLI** (#64): expose the proven local-directory and bounded-ustar digest-only/authenticated workflows through a stable host-facing contract and operator CLI, including Linux XDG default store-root resolution.
2. [ ] **P0 — complete repository cutover** (#75): after the intended final public development boundary is reached, select the exact final public commit, import and validate that baseline in `invokrum-community`, record provenance, prove independent CI/release behavior, then change canonical visibility. Topology scaffolding and the capability-status refresh are complete.
3. [ ] **P2 — portable invocation-state/evidence boundary** (#68): define model-neutral task-context, working-state, and execution-evidence ownership without standardizing private model state.
4. [ ] **P2 — trust-boundary/security-property execution metadata** (#69): extend execution contracts only as needed for a concrete security pilot while remaining policy-engine agnostic.

### Distribution sequence after the current queue

- [ ] add remote/HTTPS transport only after the signed local/archive install path remains adversarially proven through the public install boundary;
- [ ] add authenticated freshness, expiry/revocation, transparency, and rollback/freeze selection semantics before any secure mutable update or `latest` behavior;
- [ ] add registry discovery only after registry/mirror compromise cannot bypass publisher policy or immutable artifact verification;
- [ ] consider additional signature providers only with provider-specific QART, bounded parsing, compatibility identifiers, and golden/conformance vectors.

Other candidate v0.2 integration work:

- review the stable library API;
- evolve the subprocess JSON request/response contract from real consumer feedback;
- represent selected Anthesis behavior as an external reference-consumer conformance pack;
- add an Anthesis adapter without Anthesis-specific branches in the engine;
- add a read-only MCP adapter;
- add a GitHub Action for validation and drift checks.

The Anthesis conformance suite is intended to test the extraction boundary, not to establish permanent product coupling or compatibility guarantees.

## Later exploration

These are not commitments:

- WASM builds for editors or browser tooling;
- language bindings;
- hosted pack registries or marketplaces;
- policy-authoring diagnostics;
- richer conditional rule expressions;
- profile composition or inheritance;
- additional signature providers and transparency-log integrations.

## Explicit sequencing constraints

- Remote pack loading must not bypass the implemented signed/digest-pinned installation boundary.
- Registry discovery must not be able to override publisher trust policy or immutable artifact identity.
- Mutable aliases such as branches, floating tags, or `latest` must not be treated as final security identities.
- Authenticated freshness/update metadata must precede any security claim around mutable update selection.
- A valid signature must never be treated as host authorization, freshness, or semantic prompt approval.
- Plugin code execution must not precede a capability and sandbox model.
- Profile inheritance must not precede deterministic merge semantics.
- Public JSON and evidence-format changes require compatibility review, golden vectors, and executable validation.
- Consumer-specific conformance work must not introduce consumer-specific policy into the core engine.
- The community repository must receive only reviewed allowlisted content and must never mirror future private Git history.

The issue tracker is the authoritative source for active work and dependencies.
