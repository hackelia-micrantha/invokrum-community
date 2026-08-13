from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github" / "workflows" / "publish-existing-release.yml"


class PublishExistingReleaseContractTests(unittest.TestCase):
    def test_recovery_is_manual_and_repository_scoped(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")

        self.assertIn("workflow_dispatch:", workflow)
        self.assertIn("contents: write", workflow)
        self.assertIn('--repo "$GITHUB_REPOSITORY"', workflow)
        self.assertIn('confirmation must be exactly PUBLISH RELEASE', workflow)

    def test_recovery_requires_existing_complete_draft(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")

        self.assertIn('git/ref/tags/$TAG', workflow)
        self.assertIn('isDraft,isPrerelease,tagName,assets', workflow)
        self.assertIn('release $TAG is not a draft', workflow)
        self.assertIn('missing required release asset', workflow)
        self.assertIn('x86_64-unknown-linux-gnu.tar.gz', workflow)
        self.assertIn('x86_64-apple-darwin.tar.gz', workflow)
        self.assertIn('x86_64-pc-windows-msvc.zip', workflow)
        self.assertIn('.sha256', workflow)
        self.assertIn('.spdx.json', workflow)
        self.assertIn('--draft=false --prerelease', workflow)

    def test_recovery_does_not_move_tags_or_rebuild(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")

        self.assertNotIn("git push", workflow)
        self.assertNotIn("git tag", workflow)
        self.assertNotIn("cargo build", workflow)
        self.assertNotIn("--force", workflow)


if __name__ == "__main__":
    unittest.main()
