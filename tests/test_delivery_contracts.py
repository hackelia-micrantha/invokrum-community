from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from scripts.check_contract_changes import validate_changes
from scripts.check_markdown_links import validate_repository
from scripts.check_workflow_pins import validate_repository as validate_workflow_pins


class WorkflowPinTests(unittest.TestCase):
    def test_accepts_full_commit_and_local_actions(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            workflows = root / ".github" / "workflows"
            workflows.mkdir(parents=True)
            (workflows / "ci.yml").write_text(
                "steps:\n"
                "  - uses: actions/checkout@1111111111111111111111111111111111111111\n"
                "  - uses: ./local-action\n",
                encoding="utf-8",
            )
            self.assertEqual(validate_workflow_pins(root), [])

    def test_rejects_mutable_action_tag(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            workflows = root / ".github" / "workflows"
            workflows.mkdir(parents=True)
            (workflows / "ci.yml").write_text(
                "steps:\n  - uses: actions/checkout@v4\n", encoding="utf-8"
            )
            errors = validate_workflow_pins(root)
            self.assertTrue(any("full lowercase commit SHA" in error for error in errors))


class ContractChangeTests(unittest.TestCase):
    def test_requires_schema_docs_and_tests_for_parser_implementation(self) -> None:
        errors = validate_changes({"crates/invokrum-schema/src/strict.rs"})
        self.assertTrue(any("docs/schema-v1.md" in error for error in errors))
        self.assertTrue(any("schema/tests" in error for error in errors))

    def test_accepts_complete_integrity_cochange(self) -> None:
        errors = validate_changes(
            {
                "crates/invokrum-integrity/src/lockfile.rs",
                "crates/invokrum-integrity/tests/integration.rs",
                "docs/integrity-and-lockfiles.md",
            }
        )
        self.assertEqual(errors, [])

    def test_requires_host_docs_and_tests_for_rpc_changes(self) -> None:
        errors = validate_changes({"crates/invokrum-cli/src/rpc.rs"})
        self.assertTrue(any("docs/host-adapters.md" in error for error in errors))
        self.assertTrue(any("docs/usage.md" in error for error in errors))
        self.assertTrue(any("invokrum-cli/tests/rpc.rs" in error for error in errors))

    def test_accepts_complete_host_contract_cochange(self) -> None:
        errors = validate_changes(
            {
                "crates/invokrum-host/src/lib.rs",
                "crates/invokrum-cli/tests/rpc.rs",
                "docs/host-adapters.md",
                "docs/usage.md",
            }
        )
        self.assertEqual(errors, [])

    def test_requires_cli_docs_and_tests_for_machine_envelope_changes(self) -> None:
        errors = validate_changes({"crates/invokrum-cli/src/command.rs"})
        self.assertTrue(any("docs/usage.md" in error for error in errors))
        self.assertTrue(any("invokrum-cli/tests" in error for error in errors))

    def test_requires_release_docs_and_tests_for_packaging_changes(self) -> None:
        errors = validate_changes({"scripts/release.py"})
        self.assertTrue(any("docs/release.md" in error for error in errors))
        self.assertTrue(any("tests/test_release" in error for error in errors))


class MarkdownLinkTests(unittest.TestCase):
    def test_accepts_local_and_external_links(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "target.md").write_text("# Target\n", encoding="utf-8")
            (root / "source.md").write_text(
                "[local](target.md) [anchor](#section) [web](https://example.com)\n",
                encoding="utf-8",
            )
            self.assertEqual(validate_repository(root), [])

    def test_rejects_missing_and_escaping_links(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "source.md").write_text(
                "[missing](missing.md) [escape](../outside.md)\n", encoding="utf-8"
            )
            errors = validate_repository(root)
            self.assertTrue(any("does not exist" in error for error in errors))
            self.assertTrue(any("escapes repository" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
