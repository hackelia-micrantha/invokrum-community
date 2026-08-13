#!/usr/bin/env python3
"""Validate repository-local Markdown links without network access."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from urllib.parse import unquote, urlsplit

INLINE_LINK = re.compile(r"(?<!!)\[[^\]]*\]\(([^)]+)\)")
HTML_LINK = re.compile(r"\bhref=[\"']([^\"']+)[\"']", re.IGNORECASE)
EXTERNAL_SCHEMES = {"http", "https", "mailto", "tel", "data"}


def iter_markdown_links(text: str) -> list[tuple[int, str]]:
    links: list[tuple[int, str]] = []
    in_fence = False
    fence_marker = ""
    for line_number, line in enumerate(text.splitlines(), 1):
        stripped = line.lstrip()
        if stripped.startswith("```") or stripped.startswith("~~~"):
            marker = stripped[:3]
            if not in_fence:
                in_fence = True
                fence_marker = marker
            elif marker == fence_marker:
                in_fence = False
                fence_marker = ""
            continue
        if in_fence:
            continue
        links.extend((line_number, match.group(1).strip()) for match in INLINE_LINK.finditer(line))
        links.extend((line_number, match.group(1).strip()) for match in HTML_LINK.finditer(line))
    return links


def normalized_target(raw_target: str) -> str | None:
    target = raw_target
    if target.startswith("<") and target.endswith(">"):
        target = target[1:-1]
    if target.startswith("#") or not target:
        return None
    split = urlsplit(target)
    if split.scheme.lower() in EXTERNAL_SCHEMES or split.netloc:
        return None
    path = unquote(split.path)
    return path or None


def resolve_target(root: Path, source: Path, target: str) -> Path:
    if target.startswith("/"):
        return root / target.lstrip("/")
    return source.parent / target


def validate_file(root: Path, path: Path) -> list[str]:
    errors: list[str] = []
    try:
        text = path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        return [f"{path.relative_to(root)}: document is not valid UTF-8"]
    for line_number, raw_target in iter_markdown_links(text):
        target = normalized_target(raw_target)
        if target is None:
            continue
        resolved = resolve_target(root, path, target).resolve()
        try:
            resolved.relative_to(root)
        except ValueError:
            errors.append(
                f"{path.relative_to(root)}:{line_number}: local link escapes repository: {raw_target}"
            )
            continue
        if not resolved.exists():
            errors.append(
                f"{path.relative_to(root)}:{line_number}: local link target does not exist: {raw_target}"
            )
    return errors


def validate_repository(root: Path, paths: list[Path] | None = None) -> list[str]:
    candidates = paths or sorted(root.rglob("*.md"))
    errors: list[str] = []
    for path in candidates:
        absolute = path if path.is_absolute() else root / path
        if ".git" in absolute.parts or "target" in absolute.parts:
            continue
        if absolute.is_file():
            errors.extend(validate_file(root, absolute.resolve()))
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("paths", nargs="*", type=Path)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    arguments = parser.parse_args()
    root = arguments.root.resolve()
    errors = validate_repository(root, arguments.paths or None)
    if errors:
        print("local Markdown link validation failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1
    print("repository-local Markdown links are valid")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
