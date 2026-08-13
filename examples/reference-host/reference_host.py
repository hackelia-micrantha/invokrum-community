#!/usr/bin/env python3
"""Minimal independent Invokrum v0.1 subprocess reference host."""

from __future__ import annotations

import argparse
import base64
import json
import subprocess
import sys
import uuid
from dataclasses import dataclass
from pathlib import Path
from typing import Any

PROTOCOL = "invokrum.host/v1"
REQUIRED_CAPABILITIES = {"capabilities", "resolve", "verify"}
MAX_RESPONSE_BYTES = 1_048_576
DEFAULT_TIMEOUT_SECONDS = 5.0


class HostError(RuntimeError):
    pass


@dataclass(frozen=True)
class RpcResponse:
    operation: str
    result: dict[str, Any]


class InvokrumProcess:
    def __init__(
        self,
        executable: Path,
        *,
        timeout_seconds: float = DEFAULT_TIMEOUT_SECONDS,
        maximum_response_bytes: int = MAX_RESPONSE_BYTES,
    ) -> None:
        self._executable = executable
        self._timeout_seconds = timeout_seconds
        self._maximum_response_bytes = maximum_response_bytes

    def call(self, operation: str, **fields: Any) -> RpcResponse:
        request_id = f"reference-host-{uuid.uuid4().hex}"
        request = {
            "protocol": PROTOCOL,
            "request_id": request_id,
            "operation": operation,
            **fields,
        }
        encoded = json.dumps(request, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
        try:
            completed = subprocess.run(
                [str(self._executable), "rpc"],
                input=encoded,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                timeout=self._timeout_seconds,
                check=False,
            )
        except subprocess.TimeoutExpired as error:
            raise HostError(f"Invokrum RPC timed out after {self._timeout_seconds:g}s") from error
        except OSError as error:
            raise HostError(f"failed to start Invokrum: {error}") from error

        if len(completed.stdout) > self._maximum_response_bytes:
            raise HostError("Invokrum response exceeded the host byte limit")
        if completed.stderr:
            diagnostic = completed.stderr.decode("utf-8", errors="replace").strip()
            raise HostError(f"Invokrum wrote unexpected RPC stderr: {diagnostic}")

        try:
            response = json.loads(completed.stdout)
        except (UnicodeDecodeError, json.JSONDecodeError) as error:
            raise HostError("Invokrum stdout was not one valid UTF-8 JSON response") from error
        if not isinstance(response, dict):
            raise HostError("Invokrum response must be a JSON object")
        if response.get("format") != PROTOCOL:
            raise HostError(f"unsupported Invokrum response format: {response.get('format')!r}")
        if response.get("request_id") != request_id:
            raise HostError("Invokrum response request_id did not match the request")

        if response.get("ok") is not True:
            error = response.get("error")
            if not isinstance(error, dict):
                raise HostError("Invokrum returned an invalid error response")
            code = error.get("code", "unknown")
            message = error.get("message", "unknown error")
            raise HostError(f"Invokrum RPC failed [{code}]: {message}")

        if completed.returncode != 0:
            raise HostError(f"Invokrum returned success JSON with exit code {completed.returncode}")
        if response.get("operation") != operation:
            raise HostError("Invokrum response operation did not match the request")
        result = response.get("result")
        if not isinstance(result, dict):
            raise HostError("Invokrum success response did not contain an object result")
        return RpcResponse(operation=operation, result=result)

    def capabilities(self) -> dict[str, Any]:
        result = self.call("capabilities").result
        capabilities = result.get("capabilities")
        if not isinstance(capabilities, list) or not all(isinstance(item, str) for item in capabilities):
            raise HostError("Invokrum capability response is malformed")
        missing = REQUIRED_CAPABILITIES.difference(capabilities)
        if missing:
            raise HostError(f"Invokrum is missing required capabilities: {', '.join(sorted(missing))}")
        for negative in ("network_access", "persistent_writes", "runtime_invocation"):
            if result.get(negative) is not False:
                raise HostError(f"reference host requires {negative}=false")
        return result

    def resolve(self, pack: Path, profile: str) -> dict[str, Any]:
        self.capabilities()
        result = self.call("resolve", pack=str(pack), profile=profile).result
        _decode_canonical_base64(result, "context_base64")
        _decode_canonical_base64(result, "lock_base64")
        _require_digest(result)
        return result

    def verify(self, pack: Path, profile: str, expected_lock: bytes) -> dict[str, Any]:
        self.capabilities()
        result = self.call(
            "verify",
            pack=str(pack),
            profile=profile,
            expected_lock_base64=base64.b64encode(expected_lock).decode("ascii"),
        ).result
        if not isinstance(result.get("verified"), bool):
            raise HostError("verify response is missing boolean verified")
        if not isinstance(result.get("drifts"), list):
            raise HostError("verify response is missing drift list")
        bundle = result.get("bundle")
        if not isinstance(bundle, dict):
            raise HostError("verify response is missing current bundle")
        _decode_canonical_base64(bundle, "context_base64")
        _decode_canonical_base64(bundle, "lock_base64")
        _require_digest(bundle)
        return result


def _decode_canonical_base64(result: dict[str, Any], field: str) -> bytes:
    value = result.get(field)
    if not isinstance(value, str):
        raise HostError(f"Invokrum result is missing {field}")
    try:
        decoded = base64.b64decode(value, validate=True)
    except (ValueError, base64.binascii.Error) as error:
        raise HostError(f"Invokrum returned malformed {field}") from error
    if base64.b64encode(decoded).decode("ascii") != value:
        raise HostError(f"Invokrum returned noncanonical {field}")
    return decoded


def _require_digest(result: dict[str, Any]) -> str:
    digest = result.get("output_digest")
    if not isinstance(digest, str) or len(digest) != 64 or any(ch not in "0123456789abcdef" for ch in digest):
        raise HostError("Invokrum result has an invalid output_digest")
    return digest


def _write_private(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(content)
    try:
        path.chmod(0o600)
    except OSError:
        pass


def command_resolve(process: InvokrumProcess, args: argparse.Namespace) -> int:
    result = process.resolve(args.pack.resolve(), args.profile)
    context = _decode_canonical_base64(result, "context_base64")
    lock = _decode_canonical_base64(result, "lock_base64")
    _write_private(args.context_out, context)
    _write_private(args.lock_out, lock)
    print(json.dumps({
        "output_digest": result["output_digest"],
        "context": str(args.context_out),
        "lock": str(args.lock_out),
        "manifest": result.get("manifest"),
    }, sort_keys=True))
    return 0


def command_verify(process: InvokrumProcess, args: argparse.Namespace) -> int:
    result = process.verify(args.pack.resolve(), args.profile, args.lock.read_bytes())
    print(json.dumps({"verified": result["verified"], "drifts": result["drifts"]}, sort_keys=True))
    return 0 if result["verified"] else 2


def command_capabilities(process: InvokrumProcess, _: argparse.Namespace) -> int:
    print(json.dumps(process.capabilities(), sort_keys=True))
    return 0


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    result.add_argument("--invokrum", type=Path, required=True, help="Path to released invokrum executable")
    result.add_argument("--timeout", type=float, default=DEFAULT_TIMEOUT_SECONDS)
    commands = result.add_subparsers(dest="command", required=True)

    commands.add_parser("capabilities")

    resolve = commands.add_parser("resolve")
    resolve.add_argument("--pack", type=Path, required=True)
    resolve.add_argument("--profile", required=True)
    resolve.add_argument("--context-out", type=Path, default=Path("context.txt"))
    resolve.add_argument("--lock-out", type=Path, default=Path("invokrum.lock.json"))

    verify = commands.add_parser("verify")
    verify.add_argument("--pack", type=Path, required=True)
    verify.add_argument("--profile", required=True)
    verify.add_argument("--lock", type=Path, required=True)
    return result


def main() -> int:
    args = parser().parse_args()
    process = InvokrumProcess(args.invokrum, timeout_seconds=args.timeout)
    try:
        if args.command == "capabilities":
            return command_capabilities(process, args)
        if args.command == "resolve":
            return command_resolve(process, args)
        return command_verify(process, args)
    except HostError as error:
        print(f"reference host error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
