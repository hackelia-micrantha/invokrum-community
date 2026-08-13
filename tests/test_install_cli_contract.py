from __future__ import annotations

from pathlib import Path
import tomllib
import unittest


ROOT = Path(__file__).resolve().parents[1]
CLI = ROOT / "crates" / "invokrum-cli"
DELIVERY = ROOT / "crates" / "invokrum-install-delivery"
DOC = ROOT / "docs" / "install-cli.md"
USAGE = ROOT / "docs" / "usage.md"


class InstallCliContractTests(unittest.TestCase):
    def test_document_keeps_content_authentication_and_authorization_separate(self) -> None:
        text = DOC.read_text(encoding="utf-8")
        for required in (
            "invokrum install <candidate>",
            "--candidate-format directory|ustar",
            "--bundle-manifest",
            "--subject sha256:<immutable-subject>",
            "--publisher-public-key",
            "--publisher-signature",
            "--trusted-key-sha256",
            "publisher_authentication: not-provided",
            "invokrum.installation/v1",
            "invokrum.installation/v2",
            "verified-and-authorized",
            "Signature validity alone never grants authorization.",
            "fails before candidate I/O or store mutation",
            "$XDG_DATA_HOME/invokrum/store",
            "$HOME/.local/share/invokrum/store",
        ):
            self.assertIn(required, text)

        for prohibited in (
            "XDG path is a trusted identity",
            "friendly name is a security identity",
            "trust-on-first-use is enabled",
            "candidate metadata selects the trusted key",
        ):
            self.assertNotIn(prohibited, text)

    def test_composition_cli_routes_install_through_dedicated_delivery_adapter(self) -> None:
        manifest = tomllib.loads((CLI / "Cargo.toml").read_text(encoding="utf-8"))
        dependencies = set(manifest["dependencies"])
        self.assertIn("invokrum-install-delivery", dependencies)
        for forbidden in (
            "invokrum-acquisition",
            "invokrum-acquisition-archive",
            "invokrum-distribution",
            "invokrum-distribution-json",
            "invokrum-install",
            "invokrum-install-linux",
            "invokrum-verifier-ed25519",
            "ed25519-dalek",
            "clap",
        ):
            self.assertNotIn(forbidden, dependencies)

        wrapper = (CLI / "src" / "install.rs").read_text(encoding="utf-8")
        self.assertIn("invokrum_install_delivery::execute", wrapper)
        self.assertNotIn("Ed25519SubjectVerifier", wrapper)
        self.assertNotIn("TrustPolicy", wrapper)
        self.assertNotIn("LinuxInstallStore", wrapper)
        self.assertNotIn("UstarCandidateSource", wrapper)

    def test_delivery_adapter_owns_existing_acquisition_verifier_and_install_wiring(self) -> None:
        manifest = tomllib.loads((DELIVERY / "Cargo.toml").read_text(encoding="utf-8"))
        dependencies = set(manifest["dependencies"])
        for dependency in (
            "invokrum-acquisition",
            "invokrum-acquisition-archive",
            "invokrum-distribution",
            "invokrum-distribution-json",
            "invokrum-install",
            "invokrum-install-linux",
            "invokrum-verifier-ed25519",
        ):
            self.assertIn(dependency, dependencies)
        self.assertNotIn("clap", dependencies)
        self.assertNotIn("ed25519-dalek", dependencies)

        source = (DELIVERY / "src" / "lib.rs").read_text(encoding="utf-8")
        for required in (
            'std::env::var_os("XDG_DATA_HOME")',
            'std::env::var_os("HOME")',
            "UstarCandidateSource",
            "LinuxCandidateLoader",
            "LinuxInstallStore",
            "install_candidate",
            "install_authenticated_candidate",
            "Ed25519SubjectVerifier",
            "TrustPolicy",
            "CliInstallStore",
        ):
            self.assertIn(required, source)
        self.assertNotIn("reqwest", source)
        self.assertNotIn("http://", source)
        self.assertNotIn("https://", source)

    def test_public_usage_preserves_existing_cli_and_marks_rpc_read_only(self) -> None:
        text = USAGE.read_text(encoding="utf-8")
        for command in (
            "## Install",
            "## Validate",
            "## Compose",
            "## Inspect",
            "## Lock",
            "## Verify",
            "## Diff",
            "## Read-only host RPC",
        ):
            self.assertIn(command, text)
        self.assertIn("is not silently added to the read-only host RPC contract", text)


if __name__ == "__main__":
    unittest.main()
