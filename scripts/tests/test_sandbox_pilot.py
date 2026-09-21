from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from sandbox_pilot import (  # noqa: E402
    _analyze_record,
    _case_comparison,
    _case_status,
    run_pilot,
)
from sandbox_contract import validate_pilot_summary  # noqa: E402
from sandbox_runner import DockerResult  # noqa: E402


def manifest_text(sha: str) -> str:
    return f'''schema_version = 1
corpus = "technical-pilot-v1"
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
oracle = ["python3", "-c", "pass"]
expected_oracle = "passed"
'''


class FakeDocker:
    def run(self, command, cwd, image, timeout_seconds, policy, run_id):
        return DockerResult("passed", 0, "1" * 64, "2" * 64, 1.0)


class SandboxPilotTests(unittest.TestCase):
    def test_analyze_record_preserves_resource_receipt(self) -> None:
        resource = {
            "status": "available",
            "peak_rss_kb": 2048,
            "source": "posix-time-v1",
        }
        record = _analyze_record(
            {
                "phases": [
                    {
                        "name": "analyze",
                        "result": {
                            "normalized_findings": {
                                "status": "measured",
                                "count": 1,
                                "sha256": "a" * 64,
                            },
                            "resource": resource,
                        },
                    }
                ]
            }
        )

        self.assertEqual(record["resource"], resource)

    def test_repeated_case_is_stable_and_summary_validates(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = root / "control"
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
            manifest = root / "manifest.toml"
            manifest.write_text(manifest_text(sha), encoding="utf-8")
            output = root / "pilot.json"
            scanner = (
                sys.executable,
                "-c",
                "import json; print(json.dumps({'findings': []}))",
            )
            summary = run_pilot(
                manifest,
                output,
                source_root=root,
                scanner=scanner,
                repeats=2,
                docker=FakeDocker(),
                work_root=root / "work",
            )

            self.assertEqual(summary["status"], "passed")
            self.assertEqual(summary["cases"][0]["comparison"]["status"], "stable")
            self.assertTrue(output.is_file())
            data = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(len(data["cases"][0]["runs"]), 2)
            self.assertEqual(
                validate_pilot_summary(output, manifest)["status"], "valid"
            )

    def test_comparison_preserves_drift_and_unavailable(self) -> None:
        base = {"status": "measured", "sha256": "a" * 64, "count": 1}
        self.assertEqual(
            _case_comparison(
                [{"normalized": base}, {"normalized": {**base, "sha256": "b" * 64}}]
            )["status"],
            "drift",
        )
        self.assertEqual(
            _case_comparison(
                [{"normalized": {"status": "unavailable", "reason": "no scanner"}}]
            )["status"],
            "unavailable",
        )
        self.assertEqual(
            _case_status({"status": "stable"}, [{"status": "unavailable"}]),
            "unavailable",
        )


if __name__ == "__main__":
    unittest.main()
