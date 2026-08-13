//! Deterministic integrity, lockfile, and verification support for Invokrum.
//!
//! This outward adapter owns canonical evidence encoding and SHA-256. It depends
//! inward on `invokrum-core`; core does not depend on serialization or hashing.

#![forbid(unsafe_code)]

mod canonical;
mod lockfile;
mod sha256;

pub use lockfile::{
    CANONICALIZATION_FORMAT, Digester, DriftKind, IntegrityError, LOCKFILE_FORMAT, LockedManifest,
    LockedOutput, LockedOverlay, LockedPack, LockedProfile, Lockfile, MAX_LOCKED_OVERLAYS,
    MAX_LOCKFILE_BYTES, SHA256_ALGORITHM, Sha256Digester, VerificationReport, build_lockfile,
    decode_lockfile, encode_lockfile, verify, verify_with,
};
