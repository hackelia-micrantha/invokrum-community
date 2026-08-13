#!/usr/bin/env python3
"""Fail when the documented threat model loses required structure or links."""

from __future__ import annotations

from collections import Counter
from dataclasses import dataclass
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[1]
THREAT_MODEL = ROOT / "docs" / "security" / "threat-model.md"

REQUIRED_HEADINGS = {
    "## Security objectives",
    "## Assets",
    "## Actors",
    "## Entry points",
    "## Trust boundaries",
    "## Assumptions",
    "## Threat and control status matrix",
    "## Security invariants",
    "## Abuse cases and required mitigations",
    "## Responsibility matrix",
    "## Security claim discipline",
    "## Residual risk",
    "## Vulnerability reporting",
}

REQUIRED_LINKS = {
    ROOT / "README.md": "docs/security/threat-model.md",
    ROOT / "SECURITY.md": "docs/security/threat-model.md",
    ROOT / "docs" / "README.md": "security/threat-model.md",
    ROOT / "docs" / "architecture" / "README.md": "../security/threat-model.md",
}

ALLOWED_STATUSES = {
    "Implemented",
    "Partial",
    "Planned",
    "Delegated",
    "Out of scope",
}
EXPECTED_THREATS = {f"T{number:02d}" for number in range(1, 21)}
THREAT_ROW_START = re.compile(r"^\|\s*T[^|]*\|")
THREAT_ID = re.compile(r"^T\d{2}$")


@dataclass(frozen=True)
class ThreatRow:
    threat_id: str
    description: str
    status: str
    control: str
    owner: str
    line_number: int


def parse_threat_rows(text: str) -> tuple[list[ThreatRow], list[str]]:
    """Parse all threat-looking table rows without hiding malformed values."""
    rows: list[ThreatRow] = []
    errors: list[str] = []

    for line_number, line in enumerate(text.splitlines(), start=1):
        if not THREAT_ROW_START.match(line):
            continue

        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if len(cells) != 5:
            errors.append(
                f"threat row on line {line_number} must contain exactly five columns"
            )
            continue

        threat_id, description, status, control, owner = cells
        if not THREAT_ID.fullmatch(threat_id):
            errors.append(
                f"invalid threat ID `{threat_id}` on line {line_number}; expected T followed by two digits"
            )

        rows.append(
            ThreatRow(
                threat_id=threat_id,
                description=description,
                status=status,
                control=control,
                owner=owner,
                line_number=line_number,
            )
        )

    return rows, errors


def validate_threat_rows(
    rows: list[ThreatRow], expected_threats: set[str]
) -> list[str]:
    """Validate identifiers, statuses, content cells, and table completeness."""
    errors: list[str] = []
    identifiers = [row.threat_id for row in rows]
    counts = Counter(identifiers)

    duplicates = sorted(threat_id for threat_id, count in counts.items() if count > 1)
    if duplicates:
        errors.append(f"duplicate threat IDs: {', '.join(duplicates)}")

    row_ids = set(counts)
    missing = sorted(expected_threats - row_ids)
    unexpected = sorted(row_ids - expected_threats)
    if missing:
        errors.append(f"missing threat IDs: {', '.join(missing)}")
    if unexpected:
        errors.append(f"unexpected threat IDs: {', '.join(unexpected)}")

    expected_order = sorted(expected_threats)
    if not missing and not unexpected and not duplicates and identifiers != expected_order:
        errors.append("threat IDs must appear once in ascending order")

    for row in rows:
        if row.status not in ALLOWED_STATUSES:
            errors.append(
                f"invalid status `{row.status}` for {row.threat_id} on line {row.line_number}"
            )
        if not row.description:
            errors.append(f"{row.threat_id} has an empty threat description")
        if not row.control:
            errors.append(f"{row.threat_id} has an empty control cell")
        if not row.owner:
            errors.append(f"{row.threat_id} has an empty owner/follow-up cell")

    return errors


def validate_document(text: str) -> list[str]:
    """Validate the threat-model document contract."""
    errors: list[str] = []

    for heading in sorted(REQUIRED_HEADINGS):
        if heading not in text:
            errors.append(f"threat model is missing required heading: {heading}")

    rows, parse_errors = parse_threat_rows(text)
    errors.extend(parse_errors)
    errors.extend(validate_threat_rows(rows, EXPECTED_THREATS))

    for path, required_link in REQUIRED_LINKS.items():
        if not path.is_file():
            errors.append(f"missing documentation file: {path.relative_to(ROOT)}")
            continue
        linked_text = path.read_text(encoding="utf-8")
        if required_link not in linked_text:
            errors.append(f"{path.relative_to(ROOT)} must link to {required_link}")

    return errors


def main() -> int:
    if not THREAT_MODEL.is_file():
        print(
            "threat-model check failed: missing docs/security/threat-model.md",
            file=sys.stderr,
        )
        return 1

    text = THREAT_MODEL.read_text(encoding="utf-8")
    errors = validate_document(text)

    if errors:
        print("threat-model check failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    rows, _ = parse_threat_rows(text)
    print(
        "threat-model check passed "
        f"({len(rows)} threats, {len(REQUIRED_LINKS)} required links)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
