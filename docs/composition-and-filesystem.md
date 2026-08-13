# Deterministic composition and local filesystem contract

## Composition ownership

`invokrum-core` owns the composition use case and the narrow `OverlaySource` port. It resolves a validated profile without filesystem, network, process, environment, clock, randomness, or serialization access.

Resolution order is:

1. classes by validated numeric `order`;
2. selected overlays within each class in explicit profile order.

The use case rejects an unknown profile, an inconsistent aggregate reference, selected incompatibilities, excessive overlay counts, oversized source bytes, and excessive normalized output. Compatibility is directional: if either selected overlay declares the other incompatible, composition fails when that declaration is encountered in deterministic composition order. Self-incompatibility also fails.

Overlay prose never determines structural authority. The result retains ordered `ResolvedSegment` values containing exact source bytes and separate class, overlay, and source identities. `normalized_context` joins adjacent segments with exactly two LF bytes; even empty segments retain a separator boundary. Hashes and persistent manifest formats remain part of issue #5.

Default limits are:

- 256 selected overlays;
- 1 MiB per overlay source;
- 8 MiB normalized output.

Hosts may choose stricter explicit limits.

## Source port failure contract

Infrastructure adapters return stable `SourceFailureKind` categories and do not pass host parser or operating-system error strings into application diagnostics. The validated source path remains available as structured data, but human `CompositionError` text deliberately does not echo it. Delivery layers remain responsible for any later path presentation and must escape control characters.

## Linux local filesystem adapter

`invokrum-fs::LocalPackSource` is the v0.1 concrete adapter. It supports Linux only and fails closed elsewhere.

At root establishment it requires:

- the selected root itself is not a symbolic link;
- the selected root is a directory;
- the root can be canonicalized;
- the root device and inode can be pinned;
- `/proc/self/fd` is available for opened-descriptor inspection.

For each overlay it:

1. verifies that the canonical root path still names the pinned directory device and inode;
2. walks every declared path component with `symlink_metadata`;
3. rejects symbolic links at every component;
4. rejects intermediate non-directories;
5. rejects filesystem device changes from the established root;
6. canonicalizes the candidate and verifies component-aware containment below the root;
7. requires a regular file with exactly one hard link;
8. opens the file and verifies that the opened device/inode matches the pre-open candidate;
9. resolves `/proc/self/fd/<fd>` and verifies the opened target remains below the root;
10. reads at most the configured file limit plus one byte;
11. compares device, inode, size, modification time, and change time before and after reading;
12. verifies the current path still names the same non-symlink, regular, single-link inode;
13. verifies that the root path still names the pinned directory after the read.

The adapter returns the bytes from that one opened file. Composition, later hashing, and manifests must use those returned bytes rather than reopening the path.

## Supported-platform boundary

Windows junction and reparse-point behavior is not approximated. Non-Linux platforms receive `UnsupportedPlatform` until an adapter can enforce an equivalent contract with platform-native no-follow and identity primitives.

## Host preconditions and residual risk

The adapter cannot prove a trustworthy filesystem namespace against a privileged actor that can remap mounts during the operation. The host must provide:

- a stable mount namespace;
- mounted procfs with working `/proc/self/fd`;
- a pack-root parent that prevents unauthorized root rename or replacement;
- no pre-existing same-device bind mount or equivalent namespace alias below the selected root.

Device-ID checks detect cross-device mount boundaries but do not identify same-device bind mounts. Root device/inode checks make path replacement fail closed once detected, but they do not replace the namespace preconditions above.

A hostile kernel, `/proc` implementation, filesystem, or privileged mount-namespace controller remains outside the v0.1 guarantee. Filesystems whose metadata does not provide stable Linux device/inode/time semantics may fail closed or require a future specialized adapter.

No network acquisition occurs in composition or `invokrum-fs`. Pack acquisition, publisher authentication, quarantine, and update policy remain host responsibilities.
