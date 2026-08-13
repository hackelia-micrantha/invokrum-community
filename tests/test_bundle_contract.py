from __future__ import annotations

import json
from pathlib import Path
import unittest

from scripts.check_contract_changes import validate_changes


ROOT = Path(__file__).resolve().parents[1]
BUNDLE_DOC = ROOT / "docs" / "bundle-format-v1.md"
ARCH_DOC = ROOT / "docs" / "architecture" / "bundle-distribution-boundary.md"
SECURITY_DOC = ROOT / "docs" / "security" / "bundle-manifest-security.md"
SCHEMA = ROOT / "schemas" / "invokrum-pack-bundle-v1.schema.json"
GOLDEN = ROOT / "tests" / "fixtures" / "bundle" / "minimal-bundle.json"


class BundleContractTests(unittest.TestCase):
    def test_schema_and_golden_use_exact_v1_identity(self) -> None:
        schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
        golden = json.loads(GOLDEN.read_text(encoding="utf-8"))

        self.assertEqual(schema["properties"]["format"]["const"], "invokrum.pack-bundle/v1")
        self.assertEqual(schema["properties"]["digest_algorithm"]["const"], "sha256")
        self.assertEqual(golden["format"], "invokrum.pack-bundle/v1")
        self.assertEqual(golden["digest_algorithm"], "sha256")
        self.assertEqual(golden["entry_point"], "pack.yaml")

    def test_docs_keep_bundle_identity_separate_from_authentication_and_composition(self) -> None:
        bundle = BUNDLE_DOC.read_text(encoding="utf-8")
        architecture = ARCH_DOC.read_text(encoding="utf-8")
        security = SECURITY_DOC.read_text(encoding="utf-8")

        self.assertIn("It does **not** identify or authorize a publisher", bundle)
        self.assertIn("composition ──X──> acquisition/distribution", architecture)
        self.assertIn("ed25519-subject-v1 PublisherAssertion", architecture)
        self.assertIn("A valid signature is not authorization", security)
        self.assertIn("publisher_authentication: not-provided", security)
        self.assertIn("invokrum.installation/v2", security)
        self.assertIn("valid but host-unrecognized signer is denied", security)
        self.assertNotIn("signature-provider verification\n    = NOT YET IMPLEMENTED", security)

    def test_bundle_contract_changes_require_docs_and_tests(self) -> None:
        incomplete = validate_changes({"crates/invokrum-distribution/src/lib.rs"})
        self.assertTrue(any("docs/bundle-format-v1.md" in error for error in incomplete))
        self.assertTrue(any("bundle-distribution-boundary.md" in error for error in incomplete))
        self.assertTrue(any("bundle-manifest-security.md" in error for error in incomplete))
        self.assertTrue(any("tests/fixtures/bundle/" in error for error in incomplete))

        complete = validate_changes(
            {
                "crates/invokrum-distribution/src/lib.rs",
                "docs/bundle-format-v1.md",
                "docs/architecture/bundle-distribution-boundary.md",
                "docs/security/bundle-manifest-security.md",
                "crates/invokrum-distribution-json/tests/integration.rs",
                "tests/fixtures/bundle/minimal-bundle.json",
            }
        )
        self.assertEqual(complete, [])


if __name__ == "__main__":
    unittest.main()
