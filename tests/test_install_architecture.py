from __future__ import annotations

from pathlib import Path
import tomllib
import unittest


ROOT = Path(__file__).resolve().parents[1]
CRATES = ROOT / "crates"
ACQUISITION_LINUX = CRATES / "invokrum-acquisition-linux"
ACQUISITION_ARCHIVE = CRATES / "invokrum-acquisition-archive"
INSTALL = CRATES / "invokrum-install"
INSTALL_LINUX = CRATES / "invokrum-install-linux"
VERIFIER_ED25519 = CRATES / "invokrum-verifier-ed25519"


def manifest_dependencies(crate: Path, section: str = "dependencies") -> set[str]:
    data = tomllib.loads((crate / "Cargo.toml").read_text(encoding="utf-8"))
    return set((data.get(section) or {}).keys())


def production_rust(crate: Path) -> str:
    return "\n".join(
        source.read_text(encoding="utf-8").split("#[cfg(test)]", maxsplit=1)[0]
        for source in sorted((crate / "src").rglob("*.rs"))
    )


class InstallArchitectureTests(unittest.TestCase):
    def test_install_application_depends_only_on_inner_distribution_contracts(self) -> None:
        self.assertEqual(
            manifest_dependencies(INSTALL),
            {"invokrum-acquisition", "invokrum-distribution"},
        )
        production = production_rust(INSTALL)
        for token in (
            "serde::",
            "serde_json::",
            "std::env::",
            "std::fs::",
            "std::net::",
            "std::process::",
            "std::time::",
            "reqwest::",
            "tokio::",
            "ed25519_dalek::",
            "invokrum_verifier_ed25519",
        ):
            self.assertNotIn(token, production, f"install application crosses {token}")

        self.assertIn("pub trait PublisherVerifier", production)
        self.assertIn("pub struct AuthorizedPublisher", production)
        self.assertIn("pub trait AuthenticatedVerifiedBundleStore", production)
        self.assertIn("pub type AuthenticatedInstallResult", production)
        self.assertIn("TrustPolicy", production)

    def test_linux_acquisition_adapter_is_the_single_candidate_filesystem_policy(self) -> None:
        self.assertEqual(
            manifest_dependencies(ACQUISITION_LINUX),
            {"invokrum-acquisition", "invokrum-distribution"},
        )
        canonical = (ACQUISITION_LINUX / "src" / "lib.rs").read_text(encoding="utf-8")
        self.assertIn("O_NOFOLLOW", canonical)
        self.assertIn("/proc/self/fd", canonical)
        self.assertIn("LinuxLocalCandidateSource", canonical)

        installer_loader = (INSTALL_LINUX / "src" / "loader.rs").read_text(encoding="utf-8")
        self.assertIn("LinuxLocalCandidateSource", installer_loader)
        for prohibited in (
            "std::fs::",
            "std::os::fd::",
            "std::os::unix::fs::MetadataExt",
            "symlink_metadata",
            "read_dir(",
        ):
            self.assertNotIn(prohibited, installer_loader)

    def test_archive_adapter_owns_parsing_without_verification_install_or_filesystem_policy(
        self,
    ) -> None:
        self.assertEqual(
            manifest_dependencies(ACQUISITION_ARCHIVE),
            {"invokrum-acquisition", "invokrum-distribution"},
        )
        source = (ACQUISITION_ARCHIVE / "src" / "lib.rs").read_text(encoding="utf-8")
        production = source.split("#[cfg(test)]", maxsplit=1)[0]
        self.assertIn("UstarCandidateSource", production)
        self.assertIn("CandidateFile", production)
        for prohibited in (
            "verify_candidate(",
            "VerifiedBundle",
            "LinuxInstallStore",
            "compose(",
            "std::env::",
            "std::fs::",
            "std::net::",
            "std::process::",
            "reqwest::",
            "tokio::",
        ):
            self.assertNotIn(prohibited, production)

    def test_linux_install_store_keeps_crypto_and_policy_outside_filesystem_boundary(self) -> None:
        self.assertEqual(
            manifest_dependencies(INSTALL_LINUX),
            {
                "invokrum-acquisition",
                "invokrum-acquisition-linux",
                "invokrum-digest",
                "invokrum-distribution",
                "invokrum-install",
                "serde",
                "serde_json",
            },
        )
        self.assertEqual(
            manifest_dependencies(INSTALL_LINUX, "dev-dependencies"),
            {
                "invokrum-acquisition-archive",
                "invokrum-core",
                "invokrum-fs",
                "invokrum-schema",
                "invokrum-verifier-ed25519",
            },
        )
        self.assertNotIn("invokrum-verifier-ed25519", manifest_dependencies(INSTALL_LINUX))
        self.assertNotIn("ed25519-dalek", manifest_dependencies(INSTALL_LINUX))

        production = production_rust(INSTALL_LINUX)
        self.assertIn("AuthenticatedVerifiedBundleStore", production)
        self.assertIn("AuthorizedPublisher", production)
        self.assertIn("invokrum.installation/v1", production)
        self.assertIn("invokrum.installation/v2", production)
        self.assertIn("verified-and-authorized", production)
        for token in (
            "TrustPolicy",
            "PublisherAssertion",
            "ed25519_dalek::",
            "invokrum_verifier_ed25519",
            "reqwest::",
            "tokio::",
            "std::net::",
            "std::process::",
        ):
            self.assertNotIn(token, production, f"Linux store crosses {token}")

    def test_concrete_verifier_remains_an_outer_adapter_not_an_install_dependency(self) -> None:
        verifier_dependencies = manifest_dependencies(VERIFIER_ED25519)
        self.assertEqual(
            verifier_dependencies,
            {"ed25519-dalek", "invokrum-digest", "invokrum-distribution"},
        )
        self.assertNotIn("invokrum-install", verifier_dependencies)
        self.assertNotIn("invokrum-install-linux", verifier_dependencies)

        for crate in (INSTALL, INSTALL_LINUX):
            dependencies = manifest_dependencies(crate)
            self.assertNotIn("invokrum-verifier-ed25519", dependencies)
            self.assertNotIn("ed25519-dalek", dependencies)

    def test_inner_runtime_crates_do_not_depend_back_on_installation_or_archive_adapter(
        self,
    ) -> None:
        for name in (
            "invokrum-core",
            "invokrum-schema",
            "invokrum-fs",
            "invokrum-digest",
            "invokrum-integrity",
            "invokrum-host",
            "invokrum-distribution",
            "invokrum-distribution-json",
            "invokrum-acquisition",
        ):
            dependencies = manifest_dependencies(CRATES / name)
            self.assertNotIn("invokrum-install", dependencies, name)
            self.assertNotIn("invokrum-install-linux", dependencies, name)
            self.assertNotIn("invokrum-acquisition-archive", dependencies, name)
            self.assertNotIn("invokrum-verifier-ed25519", dependencies, name)
            self.assertNotIn("ed25519-dalek", dependencies, name)

    def test_installer_production_source_does_not_claim_provider_or_remote_behavior(self) -> None:
        text = "\n".join(
            production_rust(crate)
            for crate in (ACQUISITION_LINUX, ACQUISITION_ARCHIVE, INSTALL, INSTALL_LINUX)
        ).lower()
        for prohibited in (
            "sigstore",
            "ed25519-subject-v1",
            "ed25519_dalek",
            "x.509",
            "x509",
            "trust-on-first-use",
            "tofu",
            "http://",
            "https://",
        ):
            self.assertNotIn(prohibited, text)


if __name__ == "__main__":
    unittest.main()
