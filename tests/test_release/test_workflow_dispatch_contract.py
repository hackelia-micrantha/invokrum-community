from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
TAG_WORKFLOW = ROOT / ".github" / "workflows" / "create-release-tag.yml"
RELEASE_WORKFLOW = ROOT / ".github" / "workflows" / "release.yml"


class ReleaseWorkflowDispatchContractTests(unittest.TestCase):
    def test_release_supports_external_tag_push_and_explicit_dispatch(self) -> None:
        workflow = RELEASE_WORKFLOW.read_text(encoding="utf-8")

        self.assertIn('tags:\n      - "v*"', workflow)
        self.assertIn("workflow_dispatch:", workflow)
        self.assertIn('group: release-${{ github.ref }}', workflow)
        self.assertIn('validate-tag --tag "$GITHUB_REF_NAME"', workflow)

    def test_release_gh_commands_are_repository_scoped(self) -> None:
        workflow = RELEASE_WORKFLOW.read_text(encoding="utf-8")

        self.assertIn(
            'gh release edit "$GITHUB_REF_NAME" --repo "$GITHUB_REPOSITORY" --draft=false --prerelease',
            workflow,
        )
        self.assertIn('gh release view "$GITHUB_REF_NAME" --repo "$GITHUB_REPOSITORY"', workflow)
        self.assertIn('--repo "$GITHUB_REPOSITORY"', workflow)

    def test_tag_workflow_dispatches_release_at_created_immutable_tag(self) -> None:
        workflow = TAG_WORKFLOW.read_text(encoding="utf-8")

        self.assertIn("actions: write", workflow)
        self.assertIn("contents: write", workflow)
        self.assertIn('gh workflow run release.yml --ref "$TAG"', workflow)
        self.assertIn('-f ref="refs/tags/$TAG"', workflow)
        self.assertIn('if git show-ref --verify --quiet "refs/tags/$tag"', workflow)
        self.assertNotIn("secrets.", workflow)
        self.assertNotIn("--force", workflow)

    def test_tag_workflow_does_not_mutate_reviewed_version_files(self) -> None:
        workflow = TAG_WORKFLOW.read_text(encoding="utf-8")

        self.assertNotIn("git commit", workflow)
        self.assertNotIn("git push", workflow)
        self.assertNotIn("cargo set-version", workflow)
        self.assertIn(
            'selected version $selected_version does not match reviewed workspace version $workspace_version',
            workflow,
        )


if __name__ == "__main__":
    unittest.main()
