from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from sandbox_contract import SandboxManifestError, load_manifest  # noqa: E402
from sandbox_metrics import build_metrics  # noqa: E402
from sandbox_metrics_io import validate_metrics, write_metrics  # noqa: E402
from sandbox_metrics_report import render_metrics_report  # noqa: E402


def manifest_text() -> str:
    digest = "b" * 64
    return f'''schema_version = 1
corpus = "mutation-test-v1"
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
language = "javascript"
framework = "express"
[[case]]
id = "broken-export"
project = "control"
image = "node:24@sha256:{digest}"
setup = ["node", "--version"]
oracle = ["node", "--version"]
expected_oracle = "failed"
patch = "patch.diff"
mutation_kind = "violation"
split = "evaluation"
[[case]]
id = "clean-control"
project = "control"
image = "node:24@sha256:{digest}"
setup = ["node", "--version"]
oracle = ["node", "--version"]
expected_oracle = "passed"
patch = "patch.diff"
mutation_kind = "negative-control"
split = "evaluation"
'''


def artifact(
    manifest_sha: str, case_id: str, oracle: str, count: int, wall_ms: float
) -> dict:
    phases = []
    for name, status in (
        ("prepare", "passed"),
        ("baseline", "passed"),
        ("mutate", "passed"),
    ):
        phases.append({"name": name, "status": status})
    phases.append(
        {
            "name": "analyze",
            "status": "passed",
            "result": {
                "normalized_findings": {
                    "status": "measured",
                    "count": count,
                    "sha256": "c" * 64,
                },
                "wall_ms": wall_ms,
                "resource": {"status": "unavailable"},
            },
        }
    )
    phases.extend(
        [
            {"name": "oracle", "status": oracle},
            {"name": "revert", "status": "passed"},
            {"name": "collect", "status": "passed"},
        ]
    )
    return {
        "schema_version": 1,
        "protocol": "repopilot-validation-sandbox-v1",
        "corpus": "mutation-test-v1",
        "run_id": "run",
        "case_id": case_id,
        "project_id": "control",
        "manifest_sha256": manifest_sha,
        "inputs": {
            "project_sha": "a" * 40,
            "image": "node:24@sha256:" + "b" * 64,
            "patch_sha256": None,
            "source_sha": None,
        },
        "phases": phases,
        "status": "passed",
        "cleanup": {"status": "complete"},
    }


def summary_data(manifest_sha: str) -> dict:
    def case(case_id: str, kind: str, expected: str, oracle: str, count: int) -> dict:
        return {
            "case_id": case_id,
            "project_id": "control",
            "mutation_kind": kind,
            "split": "evaluation",
            "expected_oracle": expected,
            "status": "passed",
            "artifact": f"runs/{case_id}.json",
            "artifact_status": "passed",
            "oracle_status": oracle,
            "analysis": {"status": "measured", "count": count, "sha256": "c" * 64},
            "phases": {
                "baseline": {"status": "passed"},
                "mutate": {"status": "passed"},
                "oracle": {"status": oracle},
                "revert": {"status": "passed"},
            },
        }

    return {
        "schema_version": 1,
        "kind": "mutation-summary",
        "protocol": "repopilot-validation-sandbox-v1",
        "corpus": "mutation-test-v1",
        "manifest_sha256": manifest_sha,
        "status": "passed",
        "cases": [
            case("broken-export", "violation", "failed", "failed", 1),
            case("clean-control", "negative-control", "passed", "passed", 0),
        ],
    }


class SandboxMetricsTests(unittest.TestCase):
    def test_mutation_metrics_keep_denominators_and_unavailable_quality_claims(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            manifest_path = root / "manifest.toml"
            manifest_path.write_text(manifest_text(), encoding="utf-8")
            (root / "patch.diff").write_text("diff --git a/a b/a\n", encoding="utf-8")
            manifest = load_manifest(manifest_path)
            summary_path = root / "summary.json"
            summary_path.write_text(
                json.dumps(summary_data(manifest.sha256)), encoding="utf-8"
            )
            runs = root / "runs"
            runs.mkdir()
            (runs / "broken-export.json").write_text(
                json.dumps(
                    artifact(manifest.sha256, "broken-export", "failed", 1, 20.0)
                ),
                encoding="utf-8",
            )
            (runs / "clean-control.json").write_text(
                json.dumps(
                    artifact(manifest.sha256, "clean-control", "passed", 0, 10.0)
                ),
                encoding="utf-8",
            )

            metrics = build_metrics(summary_path, manifest_path)

        self.assertEqual(metrics["metrics"]["case_coverage"]["numerator"], 2)
        self.assertEqual(metrics["metrics"]["case_coverage"]["denominator"], 2)
        self.assertEqual(metrics["metrics"]["violation_signal_rate"]["numerator"], 1)
        self.assertEqual(
            metrics["metrics"]["negative_control_clean_rate"]["numerator"], 1
        )
        self.assertEqual(metrics["metrics"]["unresolved_rate"]["status"], "unavailable")
        self.assertEqual(metrics["metrics"]["tp"]["status"], "unavailable")
        self.assertEqual(metrics["performance"]["analyze_wall_ms"]["median_ms"], 15.0)
        self.assertEqual(metrics["performance"]["analyze_wall_ms"]["samples"], 2)
        self.assertEqual(len(metrics["metrics"]["case_coverage"]["wilson_95"]), 2)

    def test_metrics_validator_rejects_edited_artifact_and_report_explains_scope(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            manifest_path = root / "manifest.toml"
            manifest_path.write_text(manifest_text(), encoding="utf-8")
            (root / "patch.diff").write_text("diff --git a/a b/a\n", encoding="utf-8")
            manifest = load_manifest(manifest_path)
            summary_path = root / "summary.json"
            summary_path.write_text(
                json.dumps(summary_data(manifest.sha256)), encoding="utf-8"
            )
            runs = root / "runs"
            runs.mkdir()
            for case_id, oracle, count, wall in (
                ("broken-export", "failed", 1, 20.0),
                ("clean-control", "passed", 0, 10.0),
            ):
                (runs / f"{case_id}.json").write_text(
                    json.dumps(artifact(manifest.sha256, case_id, oracle, count, wall)),
                    encoding="utf-8",
                )
            metrics_path = root / "metrics.json"
            write_metrics(metrics_path, summary_path, manifest_path)
            self.assertEqual(
                validate_metrics(metrics_path, summary_path, manifest_path)["status"],
                "valid",
            )
            child_path = runs / "broken-export.json"
            child = json.loads(child_path.read_text(encoding="utf-8"))
            child["phases"][3]["result"]["normalized_findings"]["count"] = 2
            child_path.write_text(json.dumps(child), encoding="utf-8")
            with self.assertRaisesRegex(SandboxManifestError, "does not match summary"):
                build_metrics(summary_path, manifest_path)
            child["phases"][3]["result"]["normalized_findings"]["count"] = 1
            child_path.write_text(json.dumps(child), encoding="utf-8")
            data = json.loads(metrics_path.read_text(encoding="utf-8"))
            data["metrics"]["case_coverage"]["numerator"] = 1
            metrics_path.write_text(json.dumps(data), encoding="utf-8")

            with self.assertRaisesRegex(SandboxManifestError, "does not match"):
                validate_metrics(metrics_path, summary_path, manifest_path)

            report = render_metrics_report(build_metrics(summary_path, manifest_path))

        self.assertIn("TP", report)
        self.assertIn("unavailable", report)
        self.assertIn("Wilson", report)
        self.assertIn("| 1 normalized finding |", report)


if __name__ == "__main__":
    unittest.main()
