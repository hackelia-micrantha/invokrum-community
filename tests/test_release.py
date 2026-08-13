from __future__ import annotations

import json
import tempfile
import unittest
import zipfile
from pathlib import Path

from scripts.release import build_spdx, package_release, validate_tag


class ReleaseTagTests(unittest.TestCase):
    def test_accepts_matching_stable_and_prerelease_tags(self) -> None:
        validate_tag("v1.2.3", "1.2.3")
        validate_tag("v1.2.3-rc.1", "1.2.3-rc.1")

    def test_rejects_malformed_or_mismatched_tags(self) -> None:
        with self.assertRaises(ValueError):
            validate_tag("1.2.3", "1.2.3")
        with self.assertRaises(ValueError):
            validate_tag("v1.2.4", "1.2.3")


class SpdxTests(unittest.TestCase):
    def test_spdx_output_is_stably_ordered(self) -> None:
        metadata = {
            "packages": [
                {
                    "id": "registry+example#dep@2.0.0",
                    "name": "dep",
                    "version": "2.0.0",
                    "license": "MIT",
                    "source": "registry+https://github.com/rust-lang/crates.io-index",
                },
                {
                    "id": "path+file:///repo/crates/invokrum-cli#0.1.0",
                    "name": "invokrum-cli",
                    "version": "0.1.0",
                    "license": "Apache-2.0",
                    "source": None,
                },
            ],
            "resolve": {
                "nodes": [
                    {
                        "id": "path+file:///repo/crates/invokrum-cli#0.1.0",
                        "deps": [{"pkg": "registry+example#dep@2.0.0"}],
                    },
                    {"id": "registry+example#dep@2.0.0", "deps": []},
                ]
            },
        }
        first = build_spdx(
            metadata,
            version="0.1.0",
            target="x86_64-unknown-linux-gnu",
            created="2026-01-01T00:00:00Z",
            source_revision="a" * 40,
        )
        second = build_spdx(
            json.loads(json.dumps(metadata)),
            version="0.1.0",
            target="x86_64-unknown-linux-gnu",
            created="2026-01-01T00:00:00Z",
            source_revision="a" * 40,
        )
        self.assertEqual(first, second)
        self.assertEqual(first["spdxVersion"], "SPDX-2.3")
        self.assertTrue(
            any(
                relationship["relationshipType"] == "DEPENDS_ON"
                for relationship in first["relationships"]
            )
        )


class PackagingTests(unittest.TestCase):
    def create_root(self, directory: str) -> tuple[Path, Path, Path]:
        root = Path(directory)
        (root / "Cargo.toml").write_text(
            "[workspace]\nmembers=[]\n\n[workspace.package]\nversion=\"0.1.0\"\n",
            encoding="utf-8",
        )
        (root / "README.md").write_text("readme\n", encoding="utf-8")
        (root / "LICENSE").write_text("license\n", encoding="utf-8")
        binary = root / "invokrum"
        binary.write_bytes(b"binary")
        sbom = root / "sbom.json"
        sbom.write_text("{}\n", encoding="utf-8")
        return root, binary, sbom

    def test_tar_gzip_is_reproducible(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root, binary, sbom = self.create_root(directory)
            first, first_checksum = package_release(
                root=root,
                target="x86_64-unknown-linux-gnu",
                binary=binary,
                sbom=sbom,
                dist=root / "first",
                epoch=1_700_000_000,
            )
            second, second_checksum = package_release(
                root=root,
                target="x86_64-unknown-linux-gnu",
                binary=binary,
                sbom=sbom,
                dist=root / "second",
                epoch=1_700_000_000,
            )
            self.assertEqual(first.read_bytes(), second.read_bytes())
            self.assertEqual(first_checksum.read_text(), second_checksum.read_text())

    def test_windows_zip_contains_expected_files(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root, binary, sbom = self.create_root(directory)
            windows_binary = root / "invokrum.exe"
            windows_binary.write_bytes(binary.read_bytes())
            archive, _ = package_release(
                root=root,
                target="x86_64-pc-windows-msvc",
                binary=windows_binary,
                sbom=sbom,
                dist=root / "dist",
                epoch=1_700_000_000,
            )
            with zipfile.ZipFile(archive) as zipped:
                self.assertEqual(
                    sorted(zipped.namelist()),
                    ["LICENSE", "README.md", "SBOM.spdx.json", "invokrum.exe"],
                )


if __name__ == "__main__":
    unittest.main()
