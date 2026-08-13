#!/usr/bin/env python3
"""Fail when workspace crates cross documented clean-architecture boundaries."""

from __future__ import annotations

from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
CORE = ROOT / "crates" / "invokrum-core"
SCHEMA = ROOT / "crates" / "invokrum-schema"
FILESYSTEM = ROOT / "crates" / "invokrum-fs"
DIGEST = ROOT / "crates" / "invokrum-digest"
INTEGRITY = ROOT / "crates" / "invokrum-integrity"
HOST = ROOT / "crates" / "invokrum-host"
CLI = ROOT / "crates" / "invokrum-cli"
DISTRIBUTION = ROOT / "crates" / "invokrum-distribution"
DISTRIBUTION_JSON = ROOT / "crates" / "invokrum-distribution-json"
ACQUISITION = ROOT / "crates" / "invokrum-acquisition"
VERIFIER_ED25519 = ROOT / "crates" / "invokrum-verifier-ed25519"

CONCRETE_VERIFIER_DEPENDENCIES = {
    "ed25519-dalek",
    "invokrum-verifier-ed25519",
}

FORBIDDEN_CORE_DEPENDENCIES = {
    "clap",
    "ed25519-dalek",
    "invokrum-acquisition",
    "invokrum-digest",
    "invokrum-distribution",
    "invokrum-distribution-json",
    "invokrum-fs",
    "invokrum-host",
    "invokrum-integrity",
    "invokrum-schema",
    "invokrum-verifier-ed25519",
    "reqwest",
    "serde",
    "serde_json",
    "serde_yaml",
    "serde_yaml_ng",
    "tokio",
    "tracing-subscriber",
}

FORBIDDEN_CORE_SOURCE_TOKENS = {
    "serde::",
    "serde_json::",
    "serde_yaml::",
    "serde_yaml_ng::",
    "std::env::",
    "std::fs::",
    "std::net::",
    "std::process::",
}

FORBIDDEN_HOST_DEPENDENCIES = {
    "ed25519-dalek",
    "invokrum-acquisition",
    "invokrum-digest",
    "invokrum-distribution",
    "invokrum-distribution-json",
    "invokrum-fs",
    "invokrum-schema",
    "invokrum-verifier-ed25519",
    "reqwest",
    "serde",
    "serde_json",
    "tokio",
}

FORBIDDEN_HOST_SOURCE_TOKENS = {
    "serde::",
    "serde_json::",
    "std::env::",
    "std::fs::",
    "std::net::",
    "std::process::",
}

FORBIDDEN_DISTRIBUTION_DEPENDENCIES = {
    "ed25519-dalek",
    "invokrum-acquisition",
    "invokrum-core",
    "invokrum-digest",
    "invokrum-fs",
    "invokrum-host",
    "invokrum-integrity",
    "invokrum-schema",
    "invokrum-verifier-ed25519",
    "reqwest",
    "serde",
    "serde_json",
    "tokio",
}

FORBIDDEN_DISTRIBUTION_SOURCE_TOKENS = {
    "ed25519_dalek::",
    "serde::",
    "serde_json::",
    "std::env::",
    "std::fs::",
    "std::net::",
    "std::process::",
}

FORBIDDEN_DIGEST_DEPENDENCIES = {
    "ed25519-dalek",
    "invokrum-acquisition",
    "invokrum-core",
    "invokrum-distribution",
    "invokrum-distribution-json",
    "invokrum-fs",
    "invokrum-host",
    "invokrum-integrity",
    "invokrum-schema",
    "invokrum-verifier-ed25519",
    "reqwest",
    "serde",
    "serde_json",
    "tokio",
}

FORBIDDEN_DIGEST_SOURCE_TOKENS = {
    "ed25519_dalek::",
    "serde::",
    "serde_json::",
    "std::env::",
    "std::fs::",
    "std::net::",
    "std::process::",
}

FORBIDDEN_ACQUISITION_DEPENDENCIES = {
    "ed25519-dalek",
    "invokrum-core",
    "invokrum-distribution-json",
    "invokrum-fs",
    "invokrum-host",
    "invokrum-integrity",
    "invokrum-schema",
    "invokrum-verifier-ed25519",
    "reqwest",
    "serde",
    "serde_json",
    "serde_yaml",
    "serde_yaml_ng",
    "tokio",
}

FORBIDDEN_ACQUISITION_SOURCE_TOKENS = {
    "ed25519_dalek::",
    "serde::",
    "serde_json::",
    "std::env::",
    "std::fs::",
    "std::net::",
    "std::process::",
    "std::time::",
}

FORBIDDEN_VERIFIER_DEPENDENCIES = {
    "invokrum-acquisition",
    "invokrum-core",
    "invokrum-distribution-json",
    "invokrum-fs",
    "invokrum-host",
    "invokrum-install",
    "invokrum-install-linux",
    "invokrum-integrity",
    "invokrum-schema",
    "reqwest",
    "serde",
    "serde_json",
    "tokio",
}

FORBIDDEN_VERIFIER_SOURCE_TOKENS = {
    "std::env::",
    "std::fs::",
    "std::net::",
    "std::process::",
    "std::time::",
}


def contains_dependency(manifest: str, dependency: str) -> bool:
    return f"{dependency} =" in manifest or f'"{dependency}"' in manifest


def reject_concrete_verifier_dependency(
    errors: list[str], manifest: str, boundary_name: str
) -> None:
    for dependency in sorted(CONCRETE_VERIFIER_DEPENDENCIES):
        if contains_dependency(manifest, dependency):
            errors.append(
                f"{boundary_name} contains forbidden concrete verifier dependency: {dependency}"
            )


def require_inward_adapter(errors: list[str], crate: Path, name: str) -> None:
    manifest_path = crate / "Cargo.toml"
    if not manifest_path.is_file():
        errors.append(f"{name} adapter crate is missing")
        return

    manifest = manifest_path.read_text(encoding="utf-8")
    reject_concrete_verifier_dependency(errors, manifest, f"{name} adapter")
    if "invokrum-core" not in manifest:
        errors.append(f"{name} adapter must depend inward on invokrum-core")
    if name == "filesystem" and any(
        dependency in manifest for dependency in ["invokrum-integrity", "invokrum-schema"]
    ):
        errors.append("filesystem adapter must depend only on core workspace policy")
    if name == "schema" and any(
        dependency in manifest for dependency in ["invokrum-fs", "invokrum-integrity"]
    ):
        errors.append("schema adapter must not depend on filesystem or integrity adapters")
    if name == "integrity" and any(
        dependency in manifest for dependency in ["invokrum-fs", "invokrum-schema"]
    ):
        errors.append("integrity adapter must consume validated core values directly")


def require_cli_boundary(errors: list[str]) -> None:
    manifest_path = CLI / "Cargo.toml"
    if not manifest_path.is_file():
        errors.append("CLI crate is missing")
        return
    manifest = manifest_path.read_text(encoding="utf-8")
    reject_concrete_verifier_dependency(errors, manifest, "composition CLI")


def require_digest_primitive(errors: list[str]) -> None:
    manifest_path = DIGEST / "Cargo.toml"
    if not manifest_path.is_file():
        errors.append("digest primitive crate is missing")
        return
    manifest = manifest_path.read_text(encoding="utf-8")
    for dependency in sorted(FORBIDDEN_DIGEST_DEPENDENCIES):
        if contains_dependency(manifest, dependency):
            errors.append(f"digest primitive contains forbidden dependency: {dependency}")
    for source in sorted((DIGEST / "src").rglob("*.rs")):
        text = source.read_text(encoding="utf-8")
        for token in sorted(FORBIDDEN_DIGEST_SOURCE_TOKENS):
            if token in text:
                relative = source.relative_to(ROOT)
                errors.append(f"{relative} directly accesses an outer boundary through {token}")


def require_host_facade(errors: list[str]) -> None:
    manifest_path = HOST / "Cargo.toml"
    if not manifest_path.is_file():
        errors.append("host facade crate is missing")
        return
    manifest = manifest_path.read_text(encoding="utf-8")
    for dependency in sorted(FORBIDDEN_HOST_DEPENDENCIES):
        if contains_dependency(manifest, dependency):
            errors.append(f"host facade contains forbidden transport dependency: {dependency}")
    for required in ["invokrum-core", "invokrum-integrity"]:
        if required not in manifest:
            errors.append(f"host facade must depend on {required}")
    for source in sorted((HOST / "src").rglob("*.rs")):
        text = source.read_text(encoding="utf-8")
        for token in sorted(FORBIDDEN_HOST_SOURCE_TOKENS):
            if token in text:
                relative = source.relative_to(ROOT)
                errors.append(f"{relative} directly accesses transport boundary through {token}")


def require_distribution_boundary(errors: list[str]) -> None:
    domain_manifest = DISTRIBUTION / "Cargo.toml"
    adapter_manifest = DISTRIBUTION_JSON / "Cargo.toml"
    if not domain_manifest.is_file():
        errors.append("distribution domain crate is missing")
        return
    if not adapter_manifest.is_file():
        errors.append("distribution JSON adapter crate is missing")
        return

    manifest = domain_manifest.read_text(encoding="utf-8")
    for dependency in sorted(FORBIDDEN_DISTRIBUTION_DEPENDENCIES):
        if contains_dependency(manifest, dependency):
            errors.append(f"distribution domain contains forbidden outer dependency: {dependency}")

    adapter = adapter_manifest.read_text(encoding="utf-8")
    for required in ["invokrum-digest", "invokrum-distribution"]:
        if required not in adapter:
            errors.append(f"distribution JSON adapter must depend on {required}")
    for forbidden in [
        "ed25519-dalek",
        "invokrum-acquisition",
        "invokrum-core",
        "invokrum-fs",
        "invokrum-host",
        "invokrum-integrity",
        "invokrum-schema",
        "invokrum-verifier-ed25519",
        "reqwest",
        "tokio",
    ]:
        if contains_dependency(adapter, forbidden):
            errors.append(f"distribution JSON adapter contains forbidden dependency: {forbidden}")

    for source in sorted((DISTRIBUTION / "src").rglob("*.rs")):
        text = source.read_text(encoding="utf-8")
        for token in sorted(FORBIDDEN_DISTRIBUTION_SOURCE_TOKENS):
            if token in text:
                relative = source.relative_to(ROOT)
                errors.append(f"{relative} directly accesses an outer boundary through {token}")

    for source in sorted((DISTRIBUTION_JSON / "src").rglob("*.rs")):
        text = source.read_text(encoding="utf-8")
        for token in ["ed25519_dalek::", "std::env::", "std::fs::", "std::net::", "std::process::"]:
            if token in text:
                relative = source.relative_to(ROOT)
                errors.append(f"{relative} directly accesses acquisition infrastructure through {token}")


def require_acquisition_use_case(errors: list[str]) -> None:
    manifest_path = ACQUISITION / "Cargo.toml"
    if not manifest_path.is_file():
        errors.append("acquisition application crate is missing")
        return

    manifest = manifest_path.read_text(encoding="utf-8")
    for required in ["invokrum-digest", "invokrum-distribution"]:
        if required not in manifest:
            errors.append(f"acquisition application must depend on {required}")
    for dependency in sorted(FORBIDDEN_ACQUISITION_DEPENDENCIES):
        if contains_dependency(manifest, dependency):
            errors.append(f"acquisition application contains forbidden dependency: {dependency}")

    for source in sorted((ACQUISITION / "src").rglob("*.rs")):
        text = source.read_text(encoding="utf-8")
        for token in sorted(FORBIDDEN_ACQUISITION_SOURCE_TOKENS):
            if token in text:
                relative = source.relative_to(ROOT)
                errors.append(f"{relative} directly accesses acquisition infrastructure through {token}")


def require_ed25519_verifier(errors: list[str]) -> None:
    manifest_path = VERIFIER_ED25519 / "Cargo.toml"
    if not manifest_path.is_file():
        errors.append("Ed25519 verifier adapter crate is missing")
        return

    manifest = manifest_path.read_text(encoding="utf-8")
    for required in ["ed25519-dalek", "invokrum-digest", "invokrum-distribution"]:
        if required not in manifest:
            errors.append(f"Ed25519 verifier adapter must depend on {required}")
    if 'ed25519-dalek = { version = "3.0.0", default-features = false }' not in manifest:
        errors.append("Ed25519 verifier must pin v3 verification with default features disabled")
    for dependency in sorted(FORBIDDEN_VERIFIER_DEPENDENCIES):
        if contains_dependency(manifest, dependency):
            errors.append(f"Ed25519 verifier contains forbidden outer dependency: {dependency}")

    for source in sorted((VERIFIER_ED25519 / "src").rglob("*.rs")):
        text = source.read_text(encoding="utf-8")
        for token in sorted(FORBIDDEN_VERIFIER_SOURCE_TOKENS):
            if token in text:
                relative = source.relative_to(ROOT)
                errors.append(f"{relative} directly accesses an outer boundary through {token}")


def main() -> int:
    errors: list[str] = []

    core_manifest = (CORE / "Cargo.toml").read_text(encoding="utf-8")
    for dependency in sorted(FORBIDDEN_CORE_DEPENDENCIES):
        if contains_dependency(core_manifest, dependency):
            errors.append(f"core manifest contains forbidden outer-layer dependency: {dependency}")

    require_digest_primitive(errors)
    require_inward_adapter(errors, SCHEMA, "schema")
    require_inward_adapter(errors, FILESYSTEM, "filesystem")
    require_inward_adapter(errors, INTEGRITY, "integrity")
    require_host_facade(errors)
    require_cli_boundary(errors)
    require_distribution_boundary(errors)
    require_acquisition_use_case(errors)
    require_ed25519_verifier(errors)

    for source in sorted((CORE / "src").rglob("*.rs")):
        text = source.read_text(encoding="utf-8")
        for token in sorted(FORBIDDEN_CORE_SOURCE_TOKENS):
            if token in text:
                relative = source.relative_to(ROOT)
                errors.append(f"{relative} directly accesses outer boundary through {token}")

    if errors:
        print("architecture boundary check failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print("architecture boundary check passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
