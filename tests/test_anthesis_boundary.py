from pathlib import Path
import unittest


REPO_ROOT = Path(__file__).resolve().parents[1]
CORE_SOURCE = REPO_ROOT / "crates" / "invokrum-core" / "src"
ALLOWED_REFERENCE = (
    "crates/invokrum-core/src/lib.rs",
    "//! It must not encode Anthesis-specific policy, approval, or runtime behavior.",
)


class AnthesisBoundaryTests(unittest.TestCase):
    def test_core_contains_no_anthesis_specific_policy(self) -> None:
        references = []
        for path in sorted(CORE_SOURCE.rglob("*.rs")):
            relative = str(path.relative_to(REPO_ROOT))
            for line in path.read_text(encoding="utf-8").splitlines():
                if "anthesis" in line.lower():
                    references.append((relative, line))
        self.assertEqual(
            references,
            [ALLOWED_REFERENCE],
            "Anthesis-specific identifiers or policy must remain outside invokrum-core",
        )


if __name__ == "__main__":
    unittest.main()
