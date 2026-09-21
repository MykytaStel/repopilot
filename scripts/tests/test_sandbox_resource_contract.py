from __future__ import annotations

import hashlib
import json
import math
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from sandbox_contract import SandboxManifestError, validate_artifact  # noqa: E402
from sandbox_metrics import _rss_kb  # noqa: E402


def manifest_text() -> str:
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
sha = "{"a" * 40}"
language = "python"
framework = ""

[[case]]
id = "control-case"
project = "control"
image = "python:3.12@sha256:{"b" * 64}"
setup = ["python3", "-m", "compileall", "-q", "."]
oracle = ["python3", "-c", "print('ok')"]
expected_oracle = "passed"
'''


def artifact(manifest_sha: str, resource: dict[str, object]) -> dict[str, object]:
    return {
        "schema_version": 1,
        "protocol": "repopilot-validation-sandbox-v1",
        "corpus": "sandbox-control-v1",
        "run_id": "run",
        "case_id": "control-case",
        "project_id": "control",
        "manifest_sha256": manifest_sha,
        "inputs": {
            "project_sha": "a" * 40,
            "image": "python:3.12@sha256:" + "b" * 64,
            "patch_sha256": None,
            "source_sha": None,
        },
        "phases": [
            {
                "name": "analyze",
                "status": "passed",
                "result": {"resource": resource},
            }
        ],
        "status": "passed",
        "cleanup": {"status": "complete"},
    }


class SandboxResourceContractTests(unittest.TestCase):
    def _validate(self, resource: dict[str, object]) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            manifest_path = root / "manifest.toml"
            manifest_path.write_text(manifest_text(), encoding="utf-8")
            artifact_path = root / "result.json"
            manifest_sha = hashlib.sha256(manifest_path.read_bytes()).hexdigest()
            artifact_path.write_text(
                json.dumps(artifact(manifest_sha, resource)), encoding="utf-8"
            )
            validate_artifact(artifact_path, manifest_path)

    def test_accepts_positive_available_sample(self) -> None:
        self._validate(
            {"status": "available", "peak_rss_kb": 2048, "source": "posix-time-v1"}
        )

    def test_accepts_legacy_unavailable_sample_without_source(self) -> None:
        self._validate({"status": "unavailable", "reason": "unsupported"})

    def test_rejects_invalid_available_samples(self) -> None:
        for value in (0, -1, True, "2048", math.inf, math.nan):
            with self.subTest(value=value):
                with self.assertRaisesRegex(SandboxManifestError, "resource"):
                    self._validate(
                        {
                            "status": "available",
                            "peak_rss_kb": value,
                            "source": "posix-time-v1",
                        }
                    )

    def test_rejects_unknown_status_source_and_unavailable_sample(self) -> None:
        for resource in (
            {"status": "measured", "peak_rss_kb": 1, "source": "posix-time-v1"},
            {"status": [], "peak_rss_kb": 1, "source": "posix-time-v1"},
            {"status": "available", "peak_rss_kb": 1, "source": "other"},
            {"status": "available", "peak_rss_kb": 1, "source": []},
            {
                "status": "unavailable",
                "peak_rss_kb": 1,
                "source": "posix-time-v1",
            },
        ):
            with self.subTest(resource=resource):
                with self.assertRaisesRegex(SandboxManifestError, "resource"):
                    self._validate(resource)

    def test_metrics_ignore_non_positive_or_non_finite_samples(self) -> None:
        for value in (0, -1, 10**400, math.inf, math.nan, True, "2048"):
            with self.subTest(value=value):
                self.assertIsNone(
                    _rss_kb({"resource": {"status": "available", "peak_rss_kb": value}})
                )


if __name__ == "__main__":
    unittest.main()
