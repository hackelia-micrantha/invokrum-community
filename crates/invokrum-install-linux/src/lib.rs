//! Linux outer adapters for local Invokrum bundle acquisition and installation.
//!
//! This crate owns filesystem behavior only. It securely snapshots an already-local
//! candidate directory into owned `CandidateFile` bytes and materializes only a
//! `VerifiedBundle` into a private content-addressed store. It performs no archive,
//! network, registry, credential, trust-store, clock, or signature-provider access.

#![forbid(unsafe_code)]

mod loader;
mod store;

pub use loader::{LinuxCandidateLoader, LocalCandidateError};
pub use store::{
    AUTHENTICATED_INSTALLATION_RECORD_FORMAT, INSTALLATION_RECORD_FORMAT, INSTALLATION_RECORD_PATH,
    INSTALLED_TREE_DIGEST_FORMAT, InstalledBundle, LinuxInstallStore, LinuxInstallStoreError,
    MAX_INSTALLATION_RECORD_BYTES,
};
