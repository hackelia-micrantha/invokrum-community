from __future__ import annotations

from pathlib import Path
import unittest

from scripts.check_contract_changes import validate_changes
from scripts.check_threat_model import parse_threat_rows, validate_document


ROOT = Path(__file__).resolve().parents[1]
TRUST_DOC = ROOT / "docs" / "security" / "publisher-trust.md"
THREAT_MODEL = ROOT / "docs" / "security" / "threat-model.md"
ADR = ROOT / "docs" / "architecture" / "ADR-0002-publisher-trust-and-acquisition-boundary.md"


class PublisherTrustContractTests(unittest.TestCase):
    def test_boundary_keeps_composition_offline_and_policy_external(self) -> None:
        trust = TRUST_DOC.read_text(encoding="utf-8")
        adr = ADR.read_text(encoding="utf-8")

        for required in (
            "Composition performs no implicit acquisition or publisher verification.",
            "Trust policy is host-owned and cannot be relaxed by untrusted pack data.",
            "Mutable locators are discovery inputs, never final security identities.",
            "Promotion remains atomic into the existing content-addressed protected root.",
            "Concrete signature verification must precede cryptographic publisher evidence.",
            "Freshness and revocation policy is checked outside deterministic composition.",
        ):
            self.assertIn(required, trust)

        self.assertIn("- Status: Accepted", adr)
        self.assertIn("Trust policy is external to the pack", adr)
        self.assertIn("Composition remains offline", adr)

    def test_archive_signature_and_authenticated_install_claims_are_precise(self) -> None:
        trust = TRUST_DOC.read_text(encoding="utf-8")

        for implemented in (
            "`invokrum.pack-bundle/v1` domain and hard limits | **Implemented**",
            "Strict canonical bundle JSON and JSON Schema | **Implemented**",
            "SHA-256 immutable bundle-subject derivation | **Implemented**",
            "Host-owned trust rules and deterministic subject/identity matching | **Implemented**",
            "Linux already-local candidate tree loading | **Implemented**",
            "Bounded uncompressed POSIX ustar archive ingestion | **Implemented**",
            "Offline expected-subject and exact-byte verification | **Implemented**",
            "Linux private quarantine and content-addressed installation | **Implemented**",
            "Concrete Ed25519 subject-signature verification | **Implemented**",
            "Authenticated publisher assertion bound into installation evidence | **Implemented**",
        ):
            self.assertIn(implemented, trust)

        for still_planned in (
            "Remote/network candidate transport | **Planned**",
            "Freshness, revocation, transparency, rollback policy | **Planned**",
            "Registry discovery | **Planned**",
        ):
            self.assertIn(still_planned, trust)

        self.assertIn("publisher_authentication: not-provided", trust)
        self.assertIn("`invokrum.installation/v2`", trust)
        self.assertIn("verified-and-authorized", trust)
        self.assertIn("A valid signature is not authorization.", trust)
        self.assertIn("cannot be silently upgraded to v2", trust)
        self.assertIn("cannot be silently downgraded to v1", trust)
        self.assertNotIn("Authenticated publisher assertion bound into installation evidence | **Planned**", trust)

    def test_acquisition_and_publisher_threat_status_matches_executable_controls(self) -> None:
        text = THREAT_MODEL.read_text(encoding="utf-8")
        rows, parse_errors = parse_threat_rows(text)
        self.assertEqual(parse_errors, [])
        by_id = {row.threat_id: row for row in rows}

        self.assertEqual(by_id["T06"].status, "Partial")
        self.assertEqual(by_id["T18"].status, "Implemented")
        self.assertEqual(by_id["T19"].status, "Implemented")
        self.assertEqual(by_id["T20"].status, "Partial")

        self.assertIn("authenticated local install", by_id["T06"].control.lower())
        self.assertIn("remote retrieval is not", by_id["T06"].control.lower())
        self.assertIn("authorizedpublisher", by_id["T18"].control.lower())
        self.assertIn("unauthorized paths fail", by_id["T18"].control.lower())
        self.assertIn("archive", by_id["T19"].description.lower())
        self.assertIn("auth upgrade/downgrade", by_id["T19"].control.lower())
        self.assertIn("freshness", by_id["T20"].control.lower())

    def test_repository_threat_model_contract_still_validates(self) -> None:
        text = THREAT_MODEL.read_text(encoding="utf-8")
        self.assertEqual(validate_document(text), [])

    def test_trust_architecture_changes_require_threat_model_and_tests(self) -> None:
        incomplete = validate_changes({"docs/security/publisher-trust.md"})
        self.assertTrue(any("docs/security/threat-model.md" in error for error in incomplete))
        self.assertTrue(any("tests/test_publisher_trust" in error for error in incomplete))

        complete = validate_changes(
            {
                "docs/security/publisher-trust.md",
                "docs/security/threat-model.md",
                "tests/test_publisher_trust_contract.py",
            }
        )
        self.assertEqual(complete, [])


if __name__ == "__main__":
    unittest.main()
