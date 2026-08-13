from __future__ import annotations

from pathlib import Path
import unittest

from scripts.check_contract_changes import validate_changes


ROOT = Path(__file__).resolve().parents[1]
INSTALL_DOC = ROOT / "docs" / "security" / "linux-local-installation.md"


class LocalInstallationContractTests(unittest.TestCase):
    def test_document_keeps_security_claims_narrow(self) -> None:
        text = INSTALL_DOC.read_text(encoding="utf-8")
        for required in (
            "Implemented Linux local-directory and authenticated installation paths.",
            "VerifiedBundle",
            "sha256/<bundle-subject>",
            "invokrum.installation/v1",
            "invokrum.installation/v2",
            "invokrum.installed-tree/v1",
            "publisher_authentication: not-provided",
            "It does not authenticate a publisher.",
            "bounded POSIX ustar adapter",
            "No installation path performs network access",
            "stable mount namespace",
            "protected candidate/store parents",
            "verified-and-authorized",
            "ExistingInstallationMismatch",
            "Stale-lock recovery remains explicit host policy.",
        ):
            self.assertIn(required, text)

        for prohibited in (
            "archives are not accepted",
            "Authenticated publisher assertion bound into installation evidence | **Planned**",
        ):
            self.assertNotIn(prohibited, text)

    def test_installer_changes_require_docs_and_linux_tests(self) -> None:
        incomplete = validate_changes({"crates/invokrum-install-linux/src/store.rs"})
        self.assertTrue(any("linux-local-installation.md" in error for error in incomplete))
        self.assertTrue(any("test_install_architecture.py" in error for error in incomplete))
        self.assertTrue(any("invokrum-install-linux/tests" in error for error in incomplete))

        complete = validate_changes(
            {
                "crates/invokrum-install-linux/src/store.rs",
                "docs/security/linux-local-installation.md",
                "tests/test_install_architecture.py",
                "crates/invokrum-install-linux/tests/linux_install.rs",
            }
        )
        self.assertEqual(complete, [])


if __name__ == "__main__":
    unittest.main()
