#!/usr/bin/env python3
"""Require documentation and tests when compatibility-sensitive files change."""

from __future__ import annotations

import argparse
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class Policy:
    name: str
    triggers: tuple[str, ...]
    required_files: tuple[str, ...]
    required_prefixes: tuple[str, ...]


POLICIES = (
    Policy(
        name="pack schema",
        triggers=(
            "schemas/invokrum-pack-v1.schema.json",
            "crates/invokrum-schema/src/lib.rs",
            "crates/invokrum-schema/src/limits.rs",
            "crates/invokrum-schema/src/strict.rs",
        ),
        required_files=("docs/schema-v1.md",),
        required_prefixes=("crates/invokrum-schema/tests/", "tests/fixtures/schema/"),
    ),
    Policy(
        name="bundle distribution contract",
        triggers=(
            "schemas/invokrum-pack-bundle-v1.schema.json",
            "crates/invokrum-distribution/src/lib.rs",
            "crates/invokrum-distribution-json/src/lib.rs",
        ),
        required_files=(
            "docs/bundle-format-v1.md",
            "docs/architecture/bundle-distribution-boundary.md",
            "docs/security/bundle-manifest-security.md",
        ),
        required_prefixes=(
            "crates/invokrum-distribution-json/tests/",
            "tests/fixtures/bundle/",
        ),
    ),
    Policy(
        name="offline acquisition contract",
        triggers=("crates/invokrum-acquisition/src/lib.rs",),
        required_files=(
            "docs/architecture/bundle-distribution-boundary.md",
            "docs/security/offline-candidate-verification.md",
            "tests/test_acquisition_contract.py",
        ),
        required_prefixes=("crates/invokrum-acquisition/tests/",),
    ),
    Policy(
        name="Linux local acquisition contract",
        triggers=("crates/invokrum-acquisition-linux/src/lib.rs",),
        required_files=(
            "docs/security/offline-candidate-verification.md",
            "tests/test_install_architecture.py",
        ),
        required_prefixes=("crates/invokrum-acquisition-linux/tests/",),
    ),
    Policy(
        name="bounded archive acquisition contract",
        triggers=("crates/invokrum-acquisition-archive/src/lib.rs",),
        required_files=(
            "docs/security/archive-candidate-ingestion.md",
            "tests/test_install_architecture.py",
        ),
        required_prefixes=("crates/invokrum-install-linux/tests/",),
    ),
    Policy(
        name="Linux local installation contract",
        triggers=(
            "crates/invokrum-install/src/lib.rs",
            "crates/invokrum-install-linux/src/lib.rs",
            "crates/invokrum-install-linux/src/loader.rs",
            "crates/invokrum-install-linux/src/store.rs",
        ),
        required_files=(
            "docs/security/linux-local-installation.md",
            "tests/test_install_architecture.py",
        ),
        required_prefixes=("crates/invokrum-install-linux/tests/",),
    ),
    Policy(
        name="integrity format",
        triggers=(
            "crates/invokrum-integrity/src/canonical.rs",
            "crates/invokrum-integrity/src/lockfile.rs",
        ),
        required_files=("docs/integrity-and-lockfiles.md",),
        required_prefixes=("crates/invokrum-integrity/tests/",),
    ),
    Policy(
        name="host integration contract",
        triggers=(
            "crates/invokrum-host/src/lib.rs",
            "crates/invokrum-cli/src/rpc.rs",
            "schemas/invokrum-host-request-v1.schema.json",
            "schemas/invokrum-host-response-v1.schema.json",
        ),
        required_files=("docs/host-adapters.md", "docs/usage.md"),
        required_prefixes=("crates/invokrum-host/src/", "crates/invokrum-cli/tests/rpc.rs"),
    ),
    Policy(
        name="publisher trust architecture",
        triggers=(
            "docs/security/publisher-trust.md",
            "docs/architecture/ADR-0002-publisher-trust-and-acquisition-boundary.md",
        ),
        required_files=("docs/security/threat-model.md",),
        required_prefixes=("tests/test_publisher_trust",),
    ),
    Policy(
        name="CLI compatibility surface",
        triggers=(
            "crates/invokrum-cli/src/args.rs",
            "crates/invokrum-cli/src/command.rs",
            "crates/invokrum-cli/src/lib.rs",
        ),
        required_files=("docs/usage.md",),
        required_prefixes=("crates/invokrum-cli/tests/",),
    ),
    Policy(
        name="release workflow",
        triggers=(
            ".github/workflows/release.yml",
            "scripts/release.py",
        ),
        required_files=("docs/release.md",),
        required_prefixes=("tests/test_release",),
    ),
)


def changed_paths(root: Path, base: str, head: str) -> set[str]:
    command = ["git", "diff", "--name-only", "--diff-filter=ACMRT", f"{base}...{head}"]
    result = subprocess.run(
        command,
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    )
    return {line.strip() for line in result.stdout.splitlines() if line.strip()}


def validate_changes(changed: set[str]) -> list[str]:
    errors: list[str] = []
    for policy in POLICIES:
        triggered = sorted(set(policy.triggers) & changed)
        if not triggered:
            continue
        missing_files = [path for path in policy.required_files if path not in changed]
        has_required_prefix = any(
            path.startswith(prefix) for prefix in policy.required_prefixes for path in changed
        )
        if missing_files:
            errors.append(
                f"{policy.name}: changes to {', '.join(triggered)} require "
                f"{', '.join(missing_files)}"
            )
        if policy.required_prefixes and not has_required_prefix:
            errors.append(
                f"{policy.name}: changes to {', '.join(triggered)} require a test change under "
                f"{', '.join(policy.required_prefixes)}"
            )
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", required=True)
    parser.add_argument("--head", default="HEAD")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    arguments = parser.parse_args()
    root = arguments.root.resolve()
    try:
        changed = changed_paths(root, arguments.base, arguments.head)
    except subprocess.CalledProcessError as error:
        print(error.stderr, file=sys.stderr)
        return error.returncode or 1
    errors = validate_changes(changed)
    if errors:
        print("compatibility contract cochange validation failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1
    print("compatibility-sensitive changes include required documentation and tests")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
