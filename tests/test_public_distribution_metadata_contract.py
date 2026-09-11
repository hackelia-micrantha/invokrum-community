from __future__ import annotations

import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCHEMA = ROOT / "schemas" / "invokrum-public-distribution-v1.schema.json"


class PublicDistributionMetadataContractTests(unittest.TestCase):
    def test_schema_is_strict_and_canonical_bound(self) -> None:
        schema = json.loads(SCHEMA.read_text(encoding="utf-8"))

        self.assertFalse(schema["additionalProperties"])
        self.assertEqual(
            schema["properties"]["schema"]["const"],
            "invokrum.public-distribution/v1",
        )
        self.assertEqual(
            schema["properties"]["canonical"]["properties"]["repository"]["const"],
            "hackelia-micrantha/invokrum",
        )

    def test_schema_requires_exact_supported_binary_target_set(self) -> None:
        schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
        artifacts = schema["properties"]["artifacts"]

        self.assertEqual(artifacts["minItems"], 3)
        self.assertEqual(artifacts["maxItems"], 3)
        self.assertEqual(
            set(artifacts["items"]["properties"]["target"]["enum"]),
            {
                "x86_64-unknown-linux-gnu",
                "x86_64-apple-darwin",
                "x86_64-pc-windows-msvc",
            },
        )
        contains = artifacts["allOf"]
        self.assertEqual(len(contains), 3)
        for constraint in contains:
            self.assertEqual(constraint["minContains"], 1)
            self.assertEqual(constraint["maxContains"], 1)

    def test_schema_requires_release_identity_and_digest_fields(self) -> None:
        schema = json.loads(SCHEMA.read_text(encoding="utf-8"))

        self.assertEqual(
            set(schema["required"]),
            {"schema", "version", "canonical", "artifacts"},
        )
        self.assertEqual(
            set(schema["properties"]["canonical"]["required"]),
            {"repository", "tag", "commit"},
        )
        self.assertEqual(
            set(schema["properties"]["artifacts"]["items"]["required"]),
            {"target", "archive", "sha256"},
        )


if __name__ == "__main__":
    unittest.main()
