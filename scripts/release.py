#!/usr/bin/env python3
"""Build deterministic Invokrum release metadata and archives."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import json
import os
import re
import subprocess
import sys
import tarfile
import tomllib
import zipfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any
from urllib.parse import quote

TAG_PATTERN = re.compile(r"^v(?P<version>\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)$")
SPDX_VERSION = "SPDX-2.3"
SPDX_DATA_LICENSE = "CC0-1.0"


def workspace_version(root: Path) -> str:
    with (root / "Cargo.toml").open("rb") as handle:
        manifest = tomllib.load(handle)
    return str(manifest["workspace"]["package"]["version"])


def validate_tag(tag: str, version: str) -> None:
    match = TAG_PATTERN.fullmatch(tag)
    if not match:
        raise ValueError("release tag must use vMAJOR.MINOR.PATCH or a semver prerelease")
    if match.group("version") != version:
        raise ValueError(f"release tag {tag!r} does not match workspace version {version!r}")


def stable_identifier(prefix: str, value: str) -> str:
    digest = hashlib.sha256(value.encode("utf-8")).hexdigest()[:20]
    return f"SPDXRef-{prefix}-{digest}"


def package_download_location(package: dict[str, Any]) -> str:
    source = package.get("source")
    if isinstance(source, str) and source.startswith("registry+"):
        return source.removeprefix("registry+")
    if isinstance(source, str) and source.startswith("git+"):
        return source.removeprefix("git+")
    return "NOASSERTION"


def cargo_purl(name: str, version: str) -> str:
    return f"pkg:cargo/{quote(name, safe='')}@{quote(version, safe='')}"


def build_spdx(
    metadata: dict[str, Any],
    *,
    version: str,
    target: str,
    created: str,
    source_revision: str,
) -> dict[str, Any]:
    packages = sorted(metadata.get("packages", []), key=lambda item: str(item["id"]))
    package_ids = {
        str(package["id"]): stable_identifier("Package", str(package["id"]))
        for package in packages
    }
    spdx_packages: list[dict[str, Any]] = []
    for package in packages:
        name = str(package["name"])
        package_version = str(package["version"])
        declared = package.get("license") or "NOASSERTION"
        spdx_packages.append(
            {
                "SPDXID": package_ids[str(package["id"])],
                "name": name,
                "versionInfo": package_version,
                "downloadLocation": package_download_location(package),
                "filesAnalyzed": False,
                "licenseConcluded": "NOASSERTION",
                "licenseDeclared": declared,
                "copyrightText": "NOASSERTION",
                "externalRefs": [
                    {
                        "referenceCategory": "PACKAGE-MANAGER",
                        "referenceType": "purl",
                        "referenceLocator": cargo_purl(name, package_version),
                    }
                ],
            }
        )

    relationships: set[tuple[str, str, str]] = set()
    resolve = metadata.get("resolve") or {}
    for node in resolve.get("nodes", []):
        source_id = package_ids.get(str(node.get("id")))
        if source_id is None:
            continue
        for dependency in node.get("deps", []):
            target_id = package_ids.get(str(dependency.get("pkg")))
            if target_id is not None:
                relationships.add((source_id, "DEPENDS_ON", target_id))

    cli_package = next(
        (package for package in packages if package.get("name") == "invokrum-cli"),
        None,
    )
    if cli_package is None:
        raise ValueError("Cargo metadata does not contain invokrum-cli")
    relationships.add(("SPDXRef-DOCUMENT", "DESCRIBES", package_ids[str(cli_package["id"])]))

    document_seed = f"{version}:{target}:{source_revision}"
    return {
        "spdxVersion": SPDX_VERSION,
        "dataLicense": SPDX_DATA_LICENSE,
        "SPDXID": "SPDXRef-DOCUMENT",
        "name": f"invokrum-v{version}-{target}",
        "documentNamespace": (
            "https://github.com/hackelia-micrantha/invokrum-community/sbom/"
            f"v{version}/{target}/{hashlib.sha256(document_seed.encode()).hexdigest()}"
        ),
        "creationInfo": {
            "created": created,
            "creators": ["Tool: invokrum/scripts/release.py"],
        },
        "packages": spdx_packages,
        "relationships": [
            {
                "spdxElementId": source,
                "relationshipType": relation,
                "relatedSpdxElement": target_id,
            }
            for source, relation, target_id in sorted(relationships)
        ],
    }


def cargo_metadata(root: Path) -> dict[str, Any]:
    result = subprocess.run(
        ["cargo", "metadata", "--locked", "--format-version", "1"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(result.stdout)


def iso8601_from_epoch(epoch: int) -> str:
    return datetime.fromtimestamp(epoch, timezone.utc).replace(microsecond=0).isoformat().replace(
        "+00:00", "Z"
    )


def read_required(path: Path) -> bytes:
    if not path.is_file():
        raise ValueError(f"required release input is missing: {path}")
    return path.read_bytes()


def archive_entries(root: Path, binary: Path, sbom: Path) -> list[tuple[str, bytes, int]]:
    binary_name = "invokrum.exe" if binary.suffix.lower() == ".exe" else "invokrum"
    return [
        (binary_name, read_required(binary), 0o755),
        ("README.md", read_required(root / "README.md"), 0o644),
        ("LICENSE", read_required(root / "LICENSE"), 0o644),
        ("SBOM.spdx.json", read_required(sbom), 0o644),
    ]


def deterministic_tar_gz(path: Path, entries: list[tuple[str, bytes, int]], epoch: int) -> None:
    with path.open("wb") as output:
        with gzip.GzipFile(filename="", mode="wb", fileobj=output, mtime=epoch, compresslevel=9) as zipped:
            with tarfile.open(fileobj=zipped, mode="w", format=tarfile.PAX_FORMAT) as archive:
                for name, content, mode in sorted(entries):
                    info = tarfile.TarInfo(name)
                    info.size = len(content)
                    info.mode = mode
                    info.mtime = epoch
                    info.uid = 0
                    info.gid = 0
                    info.uname = "root"
                    info.gname = "root"
                    archive.addfile(info, io.BytesIO(content))


def zip_timestamp(epoch: int) -> tuple[int, int, int, int, int, int]:
    value = datetime.fromtimestamp(epoch, timezone.utc)
    if value.year < 1980:
        value = value.replace(year=1980, month=1, day=1, hour=0, minute=0, second=0)
    return value.year, value.month, value.day, value.hour, value.minute, value.second


def deterministic_zip(path: Path, entries: list[tuple[str, bytes, int]], epoch: int) -> None:
    timestamp = zip_timestamp(epoch)
    with zipfile.ZipFile(path, mode="w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for name, content, mode in sorted(entries):
            info = zipfile.ZipInfo(name, timestamp)
            info.compress_type = zipfile.ZIP_DEFLATED
            info.create_system = 3
            info.external_attr = (mode & 0xFFFF) << 16
            archive.writestr(info, content)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def package_release(
    *,
    root: Path,
    target: str,
    binary: Path,
    sbom: Path,
    dist: Path,
    epoch: int,
) -> tuple[Path, Path]:
    version = workspace_version(root)
    dist.mkdir(parents=True, exist_ok=True)
    stem = f"invokrum-v{version}-{target}"
    windows = target.endswith("windows-msvc")
    archive = dist / f"{stem}.zip" if windows else dist / f"{stem}.tar.gz"
    entries = archive_entries(root, binary, sbom)
    if windows:
        deterministic_zip(archive, entries, epoch)
    else:
        deterministic_tar_gz(archive, entries, epoch)
    checksum = archive.with_name(f"{archive.name}.sha256")
    checksum.write_text(f"{sha256(archive)}  {archive.name}\n", encoding="utf-8", newline="\n")
    return archive, checksum


def write_github_output(values: dict[str, Path]) -> None:
    output_path = os.environ.get("GITHUB_OUTPUT")
    if not output_path:
        return
    with Path(output_path).open("a", encoding="utf-8", newline="\n") as handle:
        for key, value in values.items():
            handle.write(f"{key}={value.resolve()}\n")


def command_validate_tag(arguments: argparse.Namespace) -> None:
    version = workspace_version(arguments.root)
    validate_tag(arguments.tag, version)
    print(f"validated {arguments.tag} against workspace version {version}")


def command_sbom(arguments: argparse.Namespace) -> None:
    version = workspace_version(arguments.root)
    metadata = cargo_metadata(arguments.root)
    created = iso8601_from_epoch(arguments.epoch)
    document = build_spdx(
        metadata,
        version=version,
        target=arguments.target,
        created=created,
        source_revision=arguments.revision,
    )
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    arguments.output.write_text(
        json.dumps(document, sort_keys=True, separators=(",", ":")) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(arguments.output.resolve())


def command_package(arguments: argparse.Namespace) -> None:
    archive, checksum = package_release(
        root=arguments.root,
        target=arguments.target,
        binary=arguments.binary,
        sbom=arguments.sbom,
        dist=arguments.dist,
        epoch=arguments.epoch,
    )
    outputs = {"archive": archive, "checksum": checksum, "sbom": arguments.sbom}
    write_github_output(outputs)
    print(json.dumps({key: str(value.resolve()) for key, value in outputs.items()}, sort_keys=True))


def build_parser() -> argparse.ArgumentParser:
    root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=root)
    subcommands = parser.add_subparsers(dest="command", required=True)

    validate = subcommands.add_parser("validate-tag")
    validate.add_argument("--tag", required=True)
    validate.set_defaults(handler=command_validate_tag)

    sbom = subcommands.add_parser("sbom")
    sbom.add_argument("--target", required=True)
    sbom.add_argument("--output", type=Path, required=True)
    sbom.add_argument("--epoch", type=int, required=True)
    sbom.add_argument("--revision", required=True)
    sbom.set_defaults(handler=command_sbom)

    package = subcommands.add_parser("package")
    package.add_argument("--target", required=True)
    package.add_argument("--binary", type=Path, required=True)
    package.add_argument("--sbom", type=Path, required=True)
    package.add_argument("--dist", type=Path, required=True)
    package.add_argument("--epoch", type=int, required=True)
    package.set_defaults(handler=command_package)
    return parser


def main() -> int:
    parser = build_parser()
    arguments = parser.parse_args()
    arguments.root = arguments.root.resolve()
    try:
        arguments.handler(arguments)
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as error:
        print(f"release error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
