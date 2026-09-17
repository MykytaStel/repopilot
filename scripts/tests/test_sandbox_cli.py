from __future__ import annotations

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))


class SandboxCliTests(unittest.TestCase):
    def test_check_reports_manifest_as_json(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            manifest = Path(tmp) / "manifest.toml"
            manifest.write_text(
                """schema_version = 1
corpus = "sandbox-control-v1"
protocol = "repopilot-validation-sandbox-v1"
[resource_policy]
schema_version = 1
policy_id = "sandbox-default-v1"
max_concurrent = 1
cpus = 2.0
memory_mb = 4096
measured_timeout_seconds = 300
prepare_timeout_seconds = 1200
network = "none"
log_limit_bytes = 65536
[[project]]
id = "control"
name = "control"
url = "https://github.com/example/control.git"
sha = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
language = "python"
framework = ""
[[case]]
id = "control-case"
project = "control"
image = "python:3.12@sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
setup = ["python3", "-c", "pass"]
oracle = ["python3", "-c", "pass"]
expected_oracle = "passed"
""",
                encoding="utf-8",
            )
            result = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPTS_DIR / "sandbox.py"),
                    "check",
                    "--manifest",
                    str(manifest),
                    "--format",
                    "json",
                ],
                capture_output=True,
                text=True,
                check=False,
            )

        self.assertEqual(result.returncode, 0)
        self.assertIn('"status": "valid"', result.stdout)

    def test_report_rejects_unknown_summary_kind(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            summary = Path(tmp) / "summary.json"
            summary.write_text('{"kind": "unknown-summary"}\n', encoding="utf-8")
            result = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPTS_DIR / "sandbox.py"),
                    "report",
                    "--manifest",
                    str(Path(tmp) / "manifest.toml"),
                    "--artifact",
                    str(summary),
                    "--format",
                    "markdown",
                ],
                capture_output=True,
                text=True,
                check=False,
            )

        self.assertEqual(result.returncode, 2)
        self.assertIn("pilot-summary or mutation-summary", result.stderr)


if __name__ == "__main__":
    unittest.main()
