from __future__ import annotations

from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = ROOT / ".github" / "workflows" / "distribution.yml"


class DistributionWorkflowContractTests(unittest.TestCase):
    def test_install_delivery_changes_trigger_distribution_contracts(self) -> None:
        text = WORKFLOW.read_text(encoding="utf-8")
        self.assertGreaterEqual(
            text.count('"crates/invokrum-install-delivery/**"'),
            2,
            "install delivery must trigger both pull-request and main-push distribution checks",
        )

    def test_install_delivery_crate_runs_in_focused_contract_suite(self) -> None:
        text = WORKFLOW.read_text(encoding="utf-8")
        self.assertIn(
            "cargo test -p invokrum-install-delivery --all-targets --all-features --locked",
            text,
        )

    def test_contract_test_itself_triggers_the_specialized_workflow(self) -> None:
        text = WORKFLOW.read_text(encoding="utf-8")
        self.assertGreaterEqual(
            text.count('"tests/test_distribution_workflow_contract.py"'),
            2,
        )


if __name__ == "__main__":
    unittest.main()
