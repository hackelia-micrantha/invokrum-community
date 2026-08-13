# Repository topology

## Decision

Invokrum uses a private-canonical/public-community repository model:

- `hackelia-micrantha/invokrum` is the canonical development repository and will become private.
- `hackelia-micrantha/invokrum-community` is the public community distribution.

The repositories are not peers and are not maintained by bidirectional mirroring. The canonical repository is upstream. Public changes are promoted deliberately into the community repository.

## Initial cutover

The first community code baseline MUST be an exact snapshot of a selected commit that was already public in `invokrum` before the visibility change, subject only to repository-specific metadata changes that are explicitly documented in the import commit.

The cutover procedure MUST record:

1. the exact upstream commit SHA;
2. the timestamp of the cutover;
3. the files changed solely for repository relocation, such as repository URLs or community-specific README text;
4. the Apache-2.0 license and attribution that applied to the public baseline;
5. a reproducible comparison showing that all non-relocation changes are accounted for.

Do not choose the cutover SHA until immediately before the final public snapshot is imported. Development may continue in `invokrum` before that point.

## Promotion direction

Normal flow is one way:

```text
private invokrum
      |
      | reviewed, allowlisted promotion
      v
public invokrum-community
```

Community contributions flow back through review, not by treating the community repository as an alternate upstream:

```text
community PR
    |
    | review / re-ingest into canonical source
    v
private invokrum
    |
    | later public promotion
    v
invokrum-community
```

A community contribution MAY be merged directly when it is intentionally community-only metadata or documentation. Shared implementation changes should be re-ingested into the canonical repository before the next promotion so the two implementations do not silently diverge.

## Public-surface policy

Promotion is allowlist-based. A new canonical path is private by default until explicitly classified for public distribution.

Current public implementation units are candidates for the initial community baseline because they are already publicly available under Apache-2.0. Their future changes are not automatically public merely because an older version was public.

Public promotion should normally include only reviewed changes to:

- stable community-facing Rust crates;
- schemas and compatibility fixtures;
- public examples and shell completions;
- public threat-model and architecture documentation;
- community CI/release configuration;
- public release notes and changelog material.

Private-only material may include, without creating a new product-layer dependency:

- unreleased implementation and experiments;
- internal roadmap and planning;
- private integrations and deployment configuration;
- internal benchmarks and evaluation artifacts;
- security research whose disclosure would create avoidable operational risk;
- private release/signing infrastructure;
- organization-specific policy, trust, or publisher configuration.

Secrets never belong in either repository.

## Fail-closed export rules

Any export tooling MUST satisfy these constraints:

1. **Allowlist, never denylist.** Unknown paths are not exported.
2. **No private-history mirroring.** The community repository receives selected content and explicit provenance, not future private Git history.
3. **No implicit recursive export of mixed-boundary directories.** Directories containing both private and public material require finer-grained classification or relocation.
4. **Repository-specific overlays are explicit.** README text, repository URLs, issue links, CI configuration, and release destinations may differ between repositories and must be reviewed as transformations rather than silently rewritten.
5. **Generated output is reviewable.** A promotion must produce a normal community pull request or equivalent reviewable diff before publication.
6. **Public CI is independent.** Community builds and releases must not require access to private repository state or private-only Actions.
7. **Provenance is retained.** Each promotion records its canonical source commit or commit range.

## Compatibility expectations

The community repository is a supported public distribution, not a documentation mirror. Publicly documented schemas, CLI behavior, machine-readable contracts, compatibility fixtures, and released artifacts must remain testable from the community checkout.

Private development may move ahead of the community distribution. When a public contract changes, the promotion must include the implementation, tests, compatibility documentation, and release notes needed to make the public state internally coherent.

## Community release identity

After cutover, public package/release metadata should resolve to `hackelia-micrantha/invokrum-community` where users need a browsable source repository. The private canonical repository may retain internal metadata separately where necessary.

Do not leave public package metadata pointing at a repository that public users cannot access.

## Cutover checklist

Before changing `invokrum` visibility to private:

- [ ] Select and record the final public cutover commit SHA.
- [ ] Import that snapshot into `invokrum-community`.
- [ ] Apply and document only necessary repository-relocation transformations.
- [ ] Verify Apache-2.0 license/attribution continuity.
- [ ] Verify the community checkout builds and tests independently.
- [ ] Verify public README, package metadata, links, CI, releases, issue templates, security reporting, and contribution instructions point at the community repository where appropriate.
- [ ] Record upstream provenance in the community repository.
- [ ] Compare the community baseline against the selected public upstream snapshot and account for every difference.
- [ ] Exercise one dry-run promotion from canonical to community through the allowlisted path.
- [ ] Only then change the canonical repository visibility.

## Non-goals

This split does not create separate security or protocol semantics for a "community edition." It is a repository/distribution boundary. Any intentional product-capability differences require their own architecture decision and compatibility policy.