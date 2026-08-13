# Fuzzing strategy

Invokrum treats fuzzing as a targeted supplement to deterministic adversarial fixtures, not a replacement for them.

## Current blocking coverage

Every pull request runs bounded, deterministic tests for:

- recursive JSON and YAML duplicate-key handling;
- unsupported YAML aliases, anchors, merge keys, tags, block scalars, directives, and multiple documents;
- schema byte, depth, and declaration limits;
- portable path grammar and traversal rejection;
- Linux symlink, hard-link, device, identity, and root-change behavior;
- canonical lock decoding, noncanonical JSON, invalid digests, tampering, and size limits;
- CLI argument, diagnostic, output-path, golden-artifact, and drift behavior.

These tests are reviewable, reproduce immediately, and establish exact expected error categories. They remain blocking even after fuzz targets are introduced.

## Candidate fuzz targets

The first useful fuzz targets are:

1. **Schema preflight and decoding**
   - feed bounded arbitrary bytes to both JSON and YAML parsers;
   - require no panic, stack overflow, uncontrolled diagnostic growth, or acceptance outside the documented YAML subset;
   - preserve deterministic error categories for identical input.
2. **Portable pack-relative paths**
   - feed arbitrary UTF-8 and byte-derived strings to `PackRelativePath::parse`;
   - require accepted values to round-trip and contain no empty, current-directory, parent-directory, platform-prefix, backslash, or colon segments.
3. **Canonical lock decoding**
   - feed bounded JSON-like bytes to `decode_lockfile`;
   - require no panic and require every accepted value to re-encode to exactly the original bytes.

Filesystem race behavior is not a good libFuzzer target because it depends on coordinated operating-system state. It remains covered by explicit integration tests and may later use a dedicated fault-injection harness.

## Admission criteria

A fuzz target should be added only when it has:

- a narrow invariant that cannot be expressed as a small finite fixture set;
- a bounded input size and execution budget;
- a stable minimized corpus checked into a dedicated corpus directory;
- an identified maintainer and triage path;
- a reproducible command pinned to a tool version;
- a process for promoting every confirmed crash into a deterministic regression test.

Until those conditions are met, adding an unowned scheduled fuzzer would create noisy, non-blocking automation without a trustworthy response path.

## Execution model

When introduced, fuzzing will run only on GitHub-hosted Linux runners or an isolated external fuzzing service. Pull requests will not receive secrets, release permissions, or self-hosted runner access. Short smoke runs may become pull-request checks after they are stable; longer campaigns should run on a schedule and retain minimized crash artifacts for the shortest practical period.

## Triage

A crash or timeout must be classified as one of:

- security vulnerability;
- correctness defect;
- resource-limit defect;
- parser dependency defect;
- harness false positive.

Security-relevant results follow [`SECURITY.md`](../../SECURITY.md). Valid findings receive a minimized deterministic regression test before the issue is considered resolved. Corpus inputs must never contain live credentials or confidential pack content.
