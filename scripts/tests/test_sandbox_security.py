from __future__ import annotations

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from sandbox_runner import run_case  # noqa: E402


def manifest_text(sha: str) -> str:
    return f'''schema_version = 1
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
sha = "{sha}"
language = "python"
framework = ""

[[case]]
id = "control-case"
project = "control"
image = "python:3.12@sha256:{"b" * 64}"
oracle = ["python3", "-c", "print('ok')"]
expected_oracle = "passed"
'''


class SandboxSecurityTests(unittest.TestCase):
    def test_source_symlink_escape_is_rejected_before_execution(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = root / "source"
            source.mkdir()
            (source / "main.py").write_text("print('ok')\n", encoding="utf-8")
            subprocess.run(["git", "init", "-q"], cwd=source, check=True)
            subprocess.run(["git", "add", "."], cwd=source, check=True)
            subprocess.run(
                [
                    "git",
                    "-c",
                    "user.name=Test",
                    "-c",
                    "user.email=test@example.com",
                    "commit",
                    "-qm",
                    "initial",
                ],
                cwd=source,
                check=True,
            )
            sha = subprocess.check_output(
                ["git", "rev-parse", "HEAD"], cwd=source, text=True
            ).strip()
            (source / "escape").symlink_to(root / "outside.txt")
            manifest = root / "manifest.toml"
            manifest.write_text(manifest_text(sha), encoding="utf-8")
            result = run_case(
                manifest,
                "control-case",
                root / "result.json",
                source_override=source,
            )

        self.assertEqual(result["status"], "failed")
        self.assertIn("escapes checkout", result["reason"])


if __name__ == "__main__":
    unittest.main()
