# Bounded archive candidate ingestion

Invokrum's first archive acquisition adapter accepts a deliberately narrow **uncompressed POSIX ustar profile** and converts it into exact owned `CandidateFile` bytes. Archive parsing remains an outer acquisition concern; immutable-subject verification and installation continue through the existing `verify_candidate` and `VerifiedBundleStore` boundaries.

## Why uncompressed ustar first

The first slice intentionally excludes compression rather than introducing a decompressor and another dependency/parser attack surface. The accepted payload therefore has a structural compression ratio of 1:1. Recognized gzip, bzip2, xz, and zstd envelopes are rejected before archive expansion, so decompression bombs cannot enter this adapter.

Compressed tar formats may be considered later as a separate, explicitly bounded adapter decision. Adding one must preserve streaming expansion limits and must not weaken this contract.

## Accepted profile

The adapter accepts caller-supplied archive bytes only. It requires:

- 512-byte-aligned POSIX ustar records with `ustar\0` magic and `00` version;
- regular files (`typeflag` `0` or NUL) and explicit directories (`typeflag` `5`) only;
- portable ASCII names accepted by `BundlePath`;
- canonical octal numeric fields rather than base-256 or extension encodings;
- valid per-header ustar checksums;
- two zero terminator blocks; additional trailing zero padding is accepted, but any non-zero trailing or concatenated archive data is rejected;
- link-name and device metadata empty/zero for accepted entries.

PAX/GNU extension records, long-name records, sparse metadata, symbolic links, hard links, devices, FIFOs, and unknown entry kinds fail closed.

## Exact-tree semantics

The validated bundle manifest remains authoritative for which logical files exist. Archive metadata never authorizes content.

The adapter derives the set of declared files plus their parent directories from the manifest, then rejects:

- undeclared files;
- unrelated explicit directories;
- missing declared files;
- duplicate logical paths;
- ASCII case-fold collisions;
- absolute, traversal, empty-segment, backslash, platform-prefixed, non-ASCII, control-character, or otherwise invalid `BundlePath` names.

Directory entries are optional. They may describe only parent directories implied by declared files and may not carry payload bytes.

## Hard bounds

The initial profile enforces limits while parsing, before copying candidate payloads:

| Bound | Maximum |
| --- | ---: |
| Archive bytes | 34,603,008 bytes |
| Archive entries | 1,024 |
| Ustar metadata | 525,312 bytes |
| Directory nesting | 64 levels |
| One bundle file | 1,048,576 bytes |
| Aggregate expanded bundle bytes | 33,554,432 bytes |

The file and aggregate payload limits reuse the immutable `invokrum.pack-bundle/v1` maxima. The archive-level limits additionally bound directory/header overhead and block padding.

The adapter operates on already-owned caller bytes, never extracts unverified entries to the installation store, and never follows archive paths on the filesystem.

## Trust and installation boundary

Successful archive parsing proves only that the archive is structurally acceptable and that its logical tree can be represented as bounded `CandidateFile` values. It does **not** prove:

- immutable bundle subject identity;
- file digests;
- publisher identity or signature validity;
- host publisher authorization;
- freshness or rollback state.

Those claims remain separate. The handoff is:

```text
caller-owned ustar bytes
  -> bounded archive adapter
  -> Vec<CandidateFile>
  -> verify_candidate
  -> VerifiedBundle
  -> existing content-addressed installation store
  -> existing installed-root composition path
```

No network, registry, credential, trust-store, environment, clock, or signature-provider behavior is introduced by archive ingestion.

## Failure posture

Malformed checksums or numeric fields, truncated payload/padding, ambiguous metadata, unsupported extensions, special entries, resource-limit violations, and non-zero data after the archive terminator all fail before verification or installation. No rejected entry is materialized on disk.
