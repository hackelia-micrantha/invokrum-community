use std::path::Path;

use invokrum_acquisition::CandidateFile;
use invokrum_acquisition_linux::LinuxLocalCandidateSource;
pub use invokrum_acquisition_linux::LocalCandidateError;
use invokrum_distribution::BundleManifest;
use invokrum_install::CandidateLoader;

/// Installation-port adapter over the canonical Linux acquisition source.
#[derive(Clone, Debug)]
pub struct LinuxCandidateLoader {
    source: LinuxLocalCandidateSource,
}

impl LinuxCandidateLoader {
    /// Opens and validates one local candidate root.
    ///
    /// # Errors
    ///
    /// Returns [`LocalCandidateError`] when the root cannot satisfy the Linux
    /// acquisition policy.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, LocalCandidateError> {
        let source = LinuxLocalCandidateSource::open(root.as_ref().to_path_buf())?;
        Ok(Self { source })
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        self.source.root()
    }
}

impl CandidateLoader for LinuxCandidateLoader {
    type Error = LocalCandidateError;

    fn load(&self, manifest: &BundleManifest) -> Result<Vec<CandidateFile>, Self::Error> {
        self.source.load(manifest)
    }
}
