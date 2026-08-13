//! Transport-neutral installation orchestration for verified Invokrum bundles.
//!
//! This application layer owns sequencing only. Candidate loading, concrete
//! publisher verification, and installed storage are injected ports; filesystem,
//! archive, network, clock, credential, registry, and signing-provider behavior
//! remains in outer adapters.

#![forbid(unsafe_code)]

use std::fmt;

use invokrum_acquisition::{
    CandidateFile, CandidateVerificationError, VerifiedBundle, verify_candidate,
};
use invokrum_distribution::{
    BundleManifest, PublisherAssertion, PublisherIdentity, Sha256Digest, TrustError, TrustPolicy,
    VerificationMechanism,
};

/// Loads exact candidate bytes for one validated bundle manifest.
pub trait CandidateLoader {
    type Error;

    /// Loads one exact candidate file set.
    ///
    /// # Errors
    ///
    /// Returns the concrete adapter failure when candidate loading cannot complete safely.
    fn load(&self, manifest: &BundleManifest) -> Result<Vec<CandidateFile>, Self::Error>;
}

/// Persists one already-verified bundle without reopening its original candidate source.
pub trait VerifiedBundleStore {
    type Receipt;
    type Error;

    /// Persists one verified bundle and returns its store receipt.
    ///
    /// # Errors
    ///
    /// Returns the concrete store failure when the verified bundle cannot be persisted safely.
    fn install(&self, bundle: &VerifiedBundle) -> Result<Self::Receipt, Self::Error>;
}

/// Concrete publisher-verification port used by authenticated installation.
///
/// Implementations are trusted outer adapters. They **must** return a
/// [`PublisherAssertion`] only after cryptographically verifying evidence for the
/// supplied immutable subject. The application layer deliberately knows nothing
/// about keys, signature encodings, certificates, transparency systems, or other
/// provider-specific inputs.
pub trait PublisherVerifier {
    type Error;

    /// Verifies publisher evidence for exactly `subject`.
    ///
    /// # Errors
    ///
    /// Returns the concrete verifier failure without creating publisher evidence.
    fn verify(&self, subject: &Sha256Digest) -> Result<PublisherAssertion, Self::Error>;
}

/// Host-authorized publisher evidence for one exact immutable subject.
///
/// Fields are private so outer stores cannot manufacture authenticated evidence
/// from descriptive metadata or an un-authorized assertion. Values are created
/// only by [`authorize_publisher`] after the injected verifier succeeds and the
/// explicit host [`TrustPolicy`] authorizes its normalized assertion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorizedPublisher {
    assertion: PublisherAssertion,
}

impl AuthorizedPublisher {
    #[must_use]
    pub const fn subject(&self) -> &Sha256Digest {
        self.assertion.subject()
    }

    #[must_use]
    pub const fn mechanism(&self) -> &VerificationMechanism {
        self.assertion.mechanism()
    }

    #[must_use]
    pub const fn identity(&self) -> &PublisherIdentity {
        self.assertion.identity()
    }
}

/// Store port for an installation whose publisher assertion was both verified
/// and explicitly authorized for the exact bundle subject.
pub trait AuthenticatedVerifiedBundleStore {
    type Receipt;
    type Error;

    /// Persists one verified bundle together with already-authorized publisher evidence.
    ///
    /// # Errors
    ///
    /// Returns the concrete store failure when the authenticated bundle cannot
    /// be persisted or exactly reused.
    fn install_authenticated(
        &self,
        bundle: &VerifiedBundle,
        publisher: &AuthorizedPublisher,
    ) -> Result<Self::Receipt, Self::Error>;
}

/// Stable orchestration stages around adapter-specific failures.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InstallWorkflowError<LoadError, StoreError> {
    Verification(CandidateVerificationError),
    Load(LoadError),
    Store(StoreError),
}

impl<LoadError: fmt::Display, StoreError: fmt::Display> fmt::Display
    for InstallWorkflowError<LoadError, StoreError>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Verification(error) => {
                write!(formatter, "candidate verification failed: {error}")
            }
            Self::Load(error) => write!(formatter, "candidate loading failed: {error}"),
            Self::Store(error) => write!(formatter, "bundle installation failed: {error}"),
        }
    }
}

impl<LoadError, StoreError> std::error::Error for InstallWorkflowError<LoadError, StoreError>
where
    LoadError: std::error::Error + 'static,
    StoreError: std::error::Error + 'static,
{
}

/// Publisher-authentication stages kept separate from candidate/store failures.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PublisherAuthenticationError<VerifierError> {
    Verification(VerifierError),
    Authorization(TrustError),
}

impl<VerifierError: fmt::Display> fmt::Display for PublisherAuthenticationError<VerifierError> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Verification(error) => {
                write!(formatter, "publisher verification failed: {error}")
            }
            Self::Authorization(error) => {
                write!(formatter, "publisher authorization failed: {error}")
            }
        }
    }
}

impl<VerifierError> std::error::Error for PublisherAuthenticationError<VerifierError> where
    VerifierError: std::error::Error + 'static
{
}

/// Stable stages for the authenticated installation workflow.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthenticatedInstallWorkflowError<VerifierError, LoadError, StoreError> {
    CandidateVerification(CandidateVerificationError),
    PublisherVerification(VerifierError),
    PublisherAuthorization(TrustError),
    Load(LoadError),
    Store(StoreError),
}

impl<VerifierError, LoadError, StoreError> fmt::Display
    for AuthenticatedInstallWorkflowError<VerifierError, LoadError, StoreError>
where
    VerifierError: fmt::Display,
    LoadError: fmt::Display,
    StoreError: fmt::Display,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CandidateVerification(error) => {
                write!(formatter, "candidate verification failed: {error}")
            }
            Self::PublisherVerification(error) => {
                write!(formatter, "publisher verification failed: {error}")
            }
            Self::PublisherAuthorization(error) => {
                write!(formatter, "publisher authorization failed: {error}")
            }
            Self::Load(error) => write!(formatter, "candidate loading failed: {error}"),
            Self::Store(error) => write!(formatter, "bundle installation failed: {error}"),
        }
    }
}

impl<VerifierError, LoadError, StoreError> std::error::Error
    for AuthenticatedInstallWorkflowError<VerifierError, LoadError, StoreError>
where
    VerifierError: std::error::Error + 'static,
    LoadError: std::error::Error + 'static,
    StoreError: std::error::Error + 'static,
{
}

/// Stable result shape for authenticated installation orchestration.
pub type AuthenticatedInstallResult<Receipt, VerifierError, LoadError, StoreError> =
    Result<Receipt, AuthenticatedInstallWorkflowError<VerifierError, LoadError, StoreError>>;

/// Invokes one trusted concrete verifier and then applies explicit host policy.
///
/// # Errors
///
/// Returns the concrete verifier error or a host-policy authorization failure.
/// No [`AuthorizedPublisher`] exists on either path.
pub fn authorize_publisher<Verifier>(
    verifier: &Verifier,
    policy: &TrustPolicy,
    expected_subject: &Sha256Digest,
) -> Result<AuthorizedPublisher, PublisherAuthenticationError<Verifier::Error>>
where
    Verifier: PublisherVerifier,
{
    let assertion = verifier
        .verify(expected_subject)
        .map_err(PublisherAuthenticationError::Verification)?;
    policy
        .authorize(&assertion, expected_subject)
        .map_err(PublisherAuthenticationError::Authorization)?;
    Ok(AuthorizedPublisher { assertion })
}

/// Loads, verifies, and installs one exact bundle candidate.
///
/// The immutable subject equality check occurs before candidate loading so an
/// untrusted source is not read when the caller already supplied contradictory
/// subject identities. The store receives only owned bytes that passed offline
/// verification; it never receives the original candidate location.
///
/// # Errors
///
/// Returns a stable verification stage or the concrete loader/store adapter
/// error without conflating content verification with publisher authentication.
pub fn install_candidate<Loader, Store>(
    loader: &Loader,
    store: &Store,
    manifest: &BundleManifest,
    manifest_subject: &Sha256Digest,
    expected_subject: &Sha256Digest,
) -> Result<Store::Receipt, InstallWorkflowError<Loader::Error, Store::Error>>
where
    Loader: CandidateLoader,
    Store: VerifiedBundleStore,
{
    if manifest_subject != expected_subject {
        return Err(InstallWorkflowError::Verification(
            CandidateVerificationError::SubjectMismatch,
        ));
    }

    let candidates = loader.load(manifest).map_err(InstallWorkflowError::Load)?;
    let verified = verify_candidate(manifest, manifest_subject, expected_subject, candidates)
        .map_err(InstallWorkflowError::Verification)?;
    store
        .install(&verified)
        .map_err(InstallWorkflowError::Store)
}

/// Verifies publisher evidence, applies explicit host policy, verifies candidate
/// bytes, and installs one exact authenticated bundle.
///
/// Contradictory manifest/expected subjects fail before invoking any adapter.
/// Publisher verification and authorization then complete before candidate I/O,
/// so an invalid or unauthorized publisher cannot mutate the store or cause an
/// untrusted candidate tree/archive to be loaded. The authenticated store receives
/// only exact verified bytes plus the opaque authorized-publisher value.
///
/// # Errors
///
/// Returns a stable stage for candidate identity, publisher verification,
/// publisher authorization, loading, or storage without conflating those claims.
pub fn install_authenticated_candidate<Verifier, Loader, Store>(
    verifier: &Verifier,
    policy: &TrustPolicy,
    loader: &Loader,
    store: &Store,
    manifest: &BundleManifest,
    manifest_subject: &Sha256Digest,
    expected_subject: &Sha256Digest,
) -> AuthenticatedInstallResult<Store::Receipt, Verifier::Error, Loader::Error, Store::Error>
where
    Verifier: PublisherVerifier,
    Loader: CandidateLoader,
    Store: AuthenticatedVerifiedBundleStore,
{
    if manifest_subject != expected_subject {
        return Err(AuthenticatedInstallWorkflowError::CandidateVerification(
            CandidateVerificationError::SubjectMismatch,
        ));
    }

    let publisher =
        authorize_publisher(verifier, policy, expected_subject).map_err(|error| match error {
            PublisherAuthenticationError::Verification(error) => {
                AuthenticatedInstallWorkflowError::PublisherVerification(error)
            }
            PublisherAuthenticationError::Authorization(error) => {
                AuthenticatedInstallWorkflowError::PublisherAuthorization(error)
            }
        })?;

    let candidates = loader
        .load(manifest)
        .map_err(AuthenticatedInstallWorkflowError::Load)?;
    let verified_bundle =
        verify_candidate(manifest, manifest_subject, expected_subject, candidates)
            .map_err(AuthenticatedInstallWorkflowError::CandidateVerification)?;

    if publisher.subject() != verified_bundle.subject() {
        return Err(AuthenticatedInstallWorkflowError::PublisherAuthorization(
            TrustError::SubjectMismatch,
        ));
    }

    store
        .install_authenticated(&verified_bundle, &publisher)
        .map_err(AuthenticatedInstallWorkflowError::Store)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use invokrum_acquisition::CandidateFile;
    use invokrum_distribution::{
        BUNDLE_FORMAT, BundleFile, BundleLimits, BundlePath, PublisherIdentity, SHA256_ALGORITHM,
        Sha256Digest, TrustRule, VerificationMechanism,
    };

    use super::*;

    fn path(value: &str) -> BundlePath {
        BundlePath::parse(value).expect("test path should be valid")
    }

    fn digest(character: char) -> Sha256Digest {
        Sha256Digest::parse(character.to_string().repeat(64)).expect("test digest should be valid")
    }

    fn manifest() -> BundleManifest {
        BundleManifest::new(
            BUNDLE_FORMAT,
            SHA256_ALGORITHM,
            path("pack.yaml"),
            vec![BundleFile::new(path("pack.yaml"), 0, digest('e'))],
            BundleLimits::default(),
        )
        .expect("test manifest should be valid")
    }

    fn publisher_assertion(subject: &Sha256Digest, fingerprint: &str) -> PublisherAssertion {
        PublisherAssertion::new(
            VerificationMechanism::parse("ed25519-subject-v1").expect("valid mechanism"),
            subject.clone(),
            PublisherIdentity::new(vec![("key.sha256".to_owned(), fingerprint.to_owned())])
                .expect("valid publisher identity"),
        )
    }

    fn policy(fingerprint: &str) -> TrustPolicy {
        TrustPolicy::new(vec![TrustRule::new(
            VerificationMechanism::parse("ed25519-subject-v1").expect("valid mechanism"),
            PublisherIdentity::new(vec![("key.sha256".to_owned(), fingerprint.to_owned())])
                .expect("valid publisher identity"),
        )])
        .expect("valid policy")
    }

    struct FakeLoader {
        called: Cell<bool>,
        candidate: Vec<CandidateFile>,
    }

    impl CandidateLoader for FakeLoader {
        type Error = &'static str;

        fn load(&self, _manifest: &BundleManifest) -> Result<Vec<CandidateFile>, Self::Error> {
            self.called.set(true);
            Ok(self.candidate.clone())
        }
    }

    struct FakeStore {
        called: Cell<bool>,
    }

    impl VerifiedBundleStore for FakeStore {
        type Receipt = Sha256Digest;
        type Error = &'static str;

        fn install(&self, bundle: &VerifiedBundle) -> Result<Self::Receipt, Self::Error> {
            self.called.set(true);
            Ok(bundle.subject().clone())
        }
    }

    struct FakeAuthenticatedStore {
        called: Cell<bool>,
        fingerprint: std::cell::RefCell<Option<String>>,
    }

    impl AuthenticatedVerifiedBundleStore for FakeAuthenticatedStore {
        type Receipt = Sha256Digest;
        type Error = &'static str;

        fn install_authenticated(
            &self,
            bundle: &VerifiedBundle,
            publisher: &AuthorizedPublisher,
        ) -> Result<Self::Receipt, Self::Error> {
            self.called.set(true);
            self.fingerprint
                .replace(publisher.identity().attributes().get("key.sha256").cloned());
            Ok(bundle.subject().clone())
        }
    }

    struct FakeVerifier {
        called: Cell<bool>,
        assertion: PublisherAssertion,
        fail: bool,
    }

    impl PublisherVerifier for FakeVerifier {
        type Error = std::io::Error;

        fn verify(&self, _subject: &Sha256Digest) -> Result<PublisherAssertion, Self::Error> {
            self.called.set(true);
            if self.fail {
                Err(std::io::Error::other("verification failed"))
            } else {
                Ok(self.assertion.clone())
            }
        }
    }

    #[test]
    fn subject_mismatch_fails_before_loading() {
        let loader = FakeLoader {
            called: Cell::new(false),
            candidate: Vec::new(),
        };
        let store = FakeStore {
            called: Cell::new(false),
        };

        let result = install_candidate(&loader, &store, &manifest(), &digest('a'), &digest('b'));

        assert_eq!(
            result,
            Err(InstallWorkflowError::Verification(
                CandidateVerificationError::SubjectMismatch
            ))
        );
        assert!(!loader.called.get());
        assert!(!store.called.get());
    }

    #[test]
    fn verified_bytes_are_the_only_store_handoff() {
        let empty_digest =
            Sha256Digest::parse("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
                .expect("published SHA-256 vector should parse");
        let manifest = BundleManifest::new(
            BUNDLE_FORMAT,
            SHA256_ALGORITHM,
            path("pack.yaml"),
            vec![BundleFile::new(path("pack.yaml"), 0, empty_digest)],
            BundleLimits::default(),
        )
        .expect("test manifest should be valid");
        let subject = digest('a');
        let loader = FakeLoader {
            called: Cell::new(false),
            candidate: vec![
                CandidateFile::new(path("pack.yaml"), Vec::new())
                    .expect("empty candidate should fit"),
            ],
        };
        let store = FakeStore {
            called: Cell::new(false),
        };

        let receipt = install_candidate(&loader, &store, &manifest, &subject, &subject)
            .expect("matching candidate should install");

        assert_eq!(receipt, subject);
        assert!(loader.called.get());
        assert!(store.called.get());
    }

    #[test]
    fn authenticated_workflow_authorizes_before_loading_and_storing() {
        let empty_digest =
            Sha256Digest::parse("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
                .expect("published SHA-256 vector should parse");
        let manifest = BundleManifest::new(
            BUNDLE_FORMAT,
            SHA256_ALGORITHM,
            path("pack.yaml"),
            vec![BundleFile::new(path("pack.yaml"), 0, empty_digest)],
            BundleLimits::default(),
        )
        .expect("test manifest should be valid");
        let subject = digest('a');
        let fingerprint = "f".repeat(64);
        let verifier = FakeVerifier {
            called: Cell::new(false),
            assertion: publisher_assertion(&subject, &fingerprint),
            fail: false,
        };
        let loader = FakeLoader {
            called: Cell::new(false),
            candidate: vec![
                CandidateFile::new(path("pack.yaml"), Vec::new())
                    .expect("empty candidate should fit"),
            ],
        };
        let store = FakeAuthenticatedStore {
            called: Cell::new(false),
            fingerprint: std::cell::RefCell::new(None),
        };

        let receipt = install_authenticated_candidate(
            &verifier,
            &policy(&fingerprint),
            &loader,
            &store,
            &manifest,
            &subject,
            &subject,
        )
        .expect("authorized exact candidate should install");

        assert_eq!(receipt, subject);
        assert!(verifier.called.get());
        assert!(loader.called.get());
        assert!(store.called.get());
        assert_eq!(
            store.fingerprint.borrow().as_deref(),
            Some(fingerprint.as_str())
        );
    }

    #[test]
    fn publisher_verification_failure_fails_before_candidate_io() {
        let subject = digest('a');
        let verifier = FakeVerifier {
            called: Cell::new(false),
            assertion: publisher_assertion(&subject, &"a".repeat(64)),
            fail: true,
        };
        let loader = FakeLoader {
            called: Cell::new(false),
            candidate: Vec::new(),
        };
        let store = FakeAuthenticatedStore {
            called: Cell::new(false),
            fingerprint: std::cell::RefCell::new(None),
        };

        let result = install_authenticated_candidate(
            &verifier,
            &policy(&"a".repeat(64)),
            &loader,
            &store,
            &manifest(),
            &subject,
            &subject,
        );

        assert!(matches!(
            result,
            Err(AuthenticatedInstallWorkflowError::PublisherVerification(_))
        ));
        assert!(verifier.called.get());
        assert!(!loader.called.get());
        assert!(!store.called.get());
    }

    #[test]
    fn unauthorized_publisher_fails_before_candidate_io() {
        let subject = digest('a');
        let verifier = FakeVerifier {
            called: Cell::new(false),
            assertion: publisher_assertion(&subject, &"a".repeat(64)),
            fail: false,
        };
        let loader = FakeLoader {
            called: Cell::new(false),
            candidate: Vec::new(),
        };
        let store = FakeAuthenticatedStore {
            called: Cell::new(false),
            fingerprint: std::cell::RefCell::new(None),
        };

        let result = install_authenticated_candidate(
            &verifier,
            &policy(&"b".repeat(64)),
            &loader,
            &store,
            &manifest(),
            &subject,
            &subject,
        );

        assert!(matches!(
            result,
            Err(AuthenticatedInstallWorkflowError::PublisherAuthorization(
                TrustError::PublisherNotAllowed
            ))
        ));
        assert!(verifier.called.get());
        assert!(!loader.called.get());
        assert!(!store.called.get());
    }

    #[test]
    fn publisher_subject_mismatch_fails_before_candidate_io() {
        let expected = digest('a');
        let verifier = FakeVerifier {
            called: Cell::new(false),
            assertion: publisher_assertion(&digest('b'), &"a".repeat(64)),
            fail: false,
        };
        let loader = FakeLoader {
            called: Cell::new(false),
            candidate: Vec::new(),
        };
        let store = FakeAuthenticatedStore {
            called: Cell::new(false),
            fingerprint: std::cell::RefCell::new(None),
        };

        let result = install_authenticated_candidate(
            &verifier,
            &policy(&"a".repeat(64)),
            &loader,
            &store,
            &manifest(),
            &expected,
            &expected,
        );

        assert!(matches!(
            result,
            Err(AuthenticatedInstallWorkflowError::PublisherAuthorization(
                TrustError::SubjectMismatch
            ))
        ));
        assert!(verifier.called.get());
        assert!(!loader.called.get());
        assert!(!store.called.get());
    }
}
