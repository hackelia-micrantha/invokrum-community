from __future__ import annotations

from pathlib import Path
import unittest

from scripts.check_contract_changes import validate_changes


ROOT = Path(__file__).resolve().parents[1]
ARCHITECTURE = ROOT / "docs" / "architecture" / "bundle-distribution-boundary.md"
SECURITY = ROOT / "docs" / "security" / "offline-candidate-verification.md"
ACQUISITION = ROOT / "crates" / "invokrum-acquisition" / "src" / "lib.rs"


class AcquisitionContractTests(unittest.TestCase):
    def test_offline_verifier_preserves_exact_byte_and_authority_boundaries(self) -> None:
        architecture = ARCHITECTURE.read_text(encoding="utf-8")
        security = SECURITY.read_text(encoding="utf-8")
        source = ACQUISITION.read_text(encoding="utf-8")

        self.assertIn("returns those same verified byte buffers", architecture)
        self.assertIn("publisher authorization remains a separate", security.lower())
        self.assertIn("subject mismatch fails before candidate content is processed", security)
        self.assertIn("pub struct VerifiedBundle", source)

        for forbidden in (
            "std::fs::",
            "std::net::",
            "std::process::",
            "std::env::",
            "std::time::",
            "reqwest::",
            "serde::",
        ):
            self.assertNotIn(forbidden, source)

    def test_acquisition_changes_require_docs_contract_test_and_integration_test(self) -> None:
        incomplete = validate_changes({"crates/invokrum-acquisition/src/lib.rs"})
        self.assertTrue(any("bundle-distribution-boundary.md" in error for error in incomplete))
        self.assertTrue(any("offline-candidate-verification.md" in error for error in incomplete))
        self.assertTrue(any("tests/test_acquisition_contract.py" in error for error in incomplete))
        self.assertTrue(any("invokrum-acquisition/tests/" in error for error in incomplete))

        complete = validate_changes(
            {
                "crates/invokrum-acquisition/src/lib.rs",
                "crates/invokrum-acquisition/tests/integration.rs",
                "docs/architecture/bundle-distribution-boundary.md",
                "docs/security/offline-candidate-verification.md",
                "tests/test_acquisition_contract.py",
            }
        )
        self.assertEqual(complete, [])


if __name__ == "__main__":
    unittest.main()
