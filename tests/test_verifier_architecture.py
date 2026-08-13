from __future__ import annotations

from pathlib import Path
import tempfile
import unittest

from scripts.check_architecture import reject_concrete_verifier_dependency


ROOT = Path(__file__).resolve().parents[1]
COMPOSITION_MANIFESTS = (
    ROOT / "crates" / "invokrum-core" / "Cargo.toml",
    ROOT / "crates" / "invokrum-schema" / "Cargo.toml",
    ROOT / "crates" / "invokrum-fs" / "Cargo.toml",
    ROOT / "crates" / "invokrum-integrity" / "Cargo.toml",
    ROOT / "crates" / "invokrum-host" / "Cargo.toml",
    ROOT / "crates" / "invokrum-cli" / "Cargo.toml",
)


class VerifierArchitectureTests(unittest.TestCase):
    def test_composition_manifests_do_not_depend_on_concrete_verifier(self) -> None:
        for manifest_path in COMPOSITION_MANIFESTS:
            with self.subTest(manifest=manifest_path.parent.name):
                manifest = manifest_path.read_text(encoding="utf-8")
                errors: list[str] = []
                reject_concrete_verifier_dependency(
                    errors, manifest, manifest_path.parent.name
                )
                self.assertEqual(errors, [])

    def test_guard_rejects_crypto_and_verifier_dependencies(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            manifest_path = Path(directory) / "Cargo.toml"
            manifest_path.write_text(
                '[dependencies]\n'
                'ed25519-dalek = "3"\n'
                'invokrum-verifier-ed25519 = { path = "../verifier" }\n',
                encoding="utf-8",
            )
            errors: list[str] = []
            reject_concrete_verifier_dependency(
                errors, manifest_path.read_text(encoding="utf-8"), "fixture"
            )

        self.assertEqual(len(errors), 2)
        self.assertTrue(any("ed25519-dalek" in error for error in errors))
        self.assertTrue(any("invokrum-verifier-ed25519" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
