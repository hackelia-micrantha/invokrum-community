#!/usr/bin/env python3
"""Reject mutable third-party action references in GitHub workflows."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ACTION_REFERENCE = re.compile(r"^\s*-?\s*uses:\s*([^\s#]+)")
FULL_COMMIT = re.compile(r"^[0-9a-f]{40}$")


def validate_workflow(path: Path) -> list[str]:
    errors: list[str] = []
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        match = ACTION_REFERENCE.match(line)
        if not match:
            continue
        reference = match.group(1).strip("'\"")
        if reference.startswith("./"):
            continue
        if "@" not in reference:
            errors.append(f"{path}:{line_number}: action reference has no revision: {reference}")
            continue
        action, revision = reference.rsplit("@", 1)
        if not action or not FULL_COMMIT.fullmatch(revision):
            errors.append(
                f"{path}:{line_number}: action must use a full lowercase commit SHA: {reference}"
            )
    return errors


def validate_repository(root: Path) -> list[str]:
    workflow_directory = root / ".github" / "workflows"
    if not workflow_directory.is_dir():
        return [f"workflow directory does not exist: {workflow_directory}"]
    errors: list[str] = []
    for path in sorted(workflow_directory.glob("*.y*ml")):
        errors.extend(validate_workflow(path))
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    arguments = parser.parse_args()
    errors = validate_repository(arguments.root.resolve())
    if errors:
        print("workflow pin validation failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1
    print("workflow action references are pinned to immutable commits")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
