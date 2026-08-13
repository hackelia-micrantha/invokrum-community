from __future__ import annotations

import importlib.util
from pathlib import Path
import sys
import tempfile
import textwrap
import unittest


ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = ROOT / "examples" / "reference-host" / "reference_host.py"
spec = importlib.util.spec_from_file_location("reference_host", MODULE_PATH)
assert spec and spec.loader
reference_host = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = reference_host
spec.loader.exec_module(reference_host)


class ReferenceHostTests(unittest.TestCase):
    def fake_invokrum(self, body: str) -> Path:
        directory = Path(tempfile.mkdtemp(prefix="invokrum-reference-host-"))
        path = directory / "invokrum"
        path.write_text(
            "#!/usr/bin/env python3\n" + textwrap.dedent(body),
            encoding="utf-8",
        )
        path.chmod(0o755)
        self.addCleanup(lambda: __import__("shutil").rmtree(directory, ignore_errors=True))
        return path

    def test_capabilities_negotiate_required_read_only_contract(self) -> None:
        executable = self.fake_invokrum(
            '''
            import json, sys
            request = json.load(sys.stdin)
            print(json.dumps({
                "format": "invokrum.host/v1",
                "ok": True,
                "operation": request["operation"],
                "request_id": request["request_id"],
                "result": {
                    "capabilities": ["capabilities", "resolve", "verify"],
                    "network_access": False,
                    "persistent_writes": False,
                    "runtime_invocation": False,
                },
            }))
            '''
        )
        result = reference_host.InvokrumProcess(executable).capabilities()
        self.assertEqual(result["network_access"], False)

    def test_missing_capability_fails_closed(self) -> None:
        executable = self.fake_invokrum(
            '''
            import json, sys
            request = json.load(sys.stdin)
            print(json.dumps({
                "format": "invokrum.host/v1",
                "ok": True,
                "operation": request["operation"],
                "request_id": request["request_id"],
                "result": {
                    "capabilities": ["capabilities", "resolve"],
                    "network_access": False,
                    "persistent_writes": False,
                    "runtime_invocation": False,
                },
            }))
            '''
        )
        with self.assertRaisesRegex(reference_host.HostError, "missing required capabilities: verify"):
            reference_host.InvokrumProcess(executable).capabilities()

    def test_protocol_stderr_is_not_mixed_with_stdout(self) -> None:
        executable = self.fake_invokrum(
            '''
            import sys
            print("diagnostic", file=sys.stderr)
            print("{}")
            '''
        )
        with self.assertRaisesRegex(reference_host.HostError, "unexpected RPC stderr"):
            reference_host.InvokrumProcess(executable).call("capabilities")

    def test_wrong_protocol_and_request_identity_fail_closed(self) -> None:
        executable = self.fake_invokrum(
            '''
            import json, sys
            request = json.load(sys.stdin)
            print(json.dumps({
                "format": "invokrum.host/v2",
                "ok": True,
                "operation": request["operation"],
                "request_id": "different",
                "result": {},
            }))
            '''
        )
        with self.assertRaisesRegex(reference_host.HostError, "unsupported Invokrum response format"):
            reference_host.InvokrumProcess(executable).call("capabilities")

    def test_oversized_response_is_rejected_before_parsing(self) -> None:
        executable = self.fake_invokrum(
            '''
            print("x" * 512)
            '''
        )
        process = reference_host.InvokrumProcess(executable, maximum_response_bytes=128)
        with self.assertRaisesRegex(reference_host.HostError, "response exceeded"):
            process.call("capabilities")

    def test_timeout_is_bounded(self) -> None:
        executable = self.fake_invokrum(
            '''
            import time
            time.sleep(1)
            '''
        )
        process = reference_host.InvokrumProcess(executable, timeout_seconds=0.05)
        with self.assertRaisesRegex(reference_host.HostError, "timed out"):
            process.call("capabilities")

    def test_canonical_base64_is_required(self) -> None:
        with self.assertRaisesRegex(reference_host.HostError, "noncanonical context_base64"):
            reference_host._decode_canonical_base64({"context_base64": "AB=="}, "context_base64")


if __name__ == "__main__":
    unittest.main()
