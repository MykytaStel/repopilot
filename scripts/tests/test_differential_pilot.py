from __future__ import annotations

import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from differential_artifact import validate_artifact  # noqa: E402
from differential_pilot import load_pilot, render_pilot_template, validate_pilot  # noqa: E402
from differential_pilot_metrics import build_pilot_metrics  # noqa: E402
from differential_telemetry import build_command_telemetry  # noqa: E402
from real_history_runner import BASELINE_COMMANDS  # noqa: E402


class DifferentialPilotTests(unittest.TestCase):
    def test_template_is_blinded_and_completed_pilot_validates(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            inputs = self._write_inputs(root)
            template = root / "pilot.toml"
            template.write_text(
                render_pilot_template(inputs["artifact"], inputs["manifest"], inputs["differential"], inputs["rules"], inputs["zoo"], "expert"),
                encoding="utf-8",
            )
            text = template.read_text(encoding="utf-8").replace('label = ""', 'label = "no-defect"').replace('rationale = ""', 'rationale = "reviewed exact diff"')
            template.write_text(text, encoding="utf-8")
            result = validate_pilot(template, inputs["artifact"], inputs["manifest"], inputs["differential"], inputs["rules"], inputs["zoo"])
            rendered = template.read_text(encoding="utf-8")
        self.assertEqual(result["status"], "valid")
        self.assertIn("blinded = true", rendered)
        self.assertNotIn("novel_in_diff_evidence_keys", rendered)

    def test_pilot_rejects_unknown_rule(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            inputs = self._write_inputs(root)
            template = root / "pilot.toml"
            template.write_text(
                render_pilot_template(inputs["artifact"], inputs["manifest"], inputs["differential"], inputs["rules"], inputs["zoo"], "expert").replace(
                    'label = ""', 'label = "defect-present"', 1
                ).replace('expected_rule_ids = []', 'expected_rule_ids = ["unknown.rule"]', 1).replace(
                    'rationale = ""', 'rationale = "unsupported rule"', 1
                ),
                encoding="utf-8",
            )
            with self.assertRaisesRegex(ValueError, "unknown rule"):
                validate_pilot(template, inputs["artifact"], inputs["manifest"], inputs["differential"], inputs["rules"], inputs["zoo"])

    def test_scores_exploratory_outcomes(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            inputs = self._write_inputs(root, include_novel=True)
            pilot = root / "pilot.toml"
            pilot.write_text(
                render_pilot_template(inputs["artifact"], inputs["manifest"], inputs["differential"], inputs["rules"], inputs["zoo"], "expert")
                .replace('label = ""', 'label = "no-defect"', 1)
                .replace('rationale = ""', 'rationale = "reviewed first case"', 1)
                .replace('label = ""', 'label = "uncertain"', 1)
                .replace('rationale = ""', 'rationale = "insufficient context"', 1),
                encoding="utf-8",
            )
            output = build_pilot_metrics(
                inputs["artifact"], pilot, inputs["manifest"], inputs["differential"], inputs["rules"], inputs["zoo"]
            )
        self.assertEqual(output["scope"], "single-expert exploratory pilot")
        self.assertEqual(output["counts"], {"tp": 0, "fn": 0, "tn": 0, "fp": 1, "excluded": 1})
        self.assertIsNone(output["metrics"]["recall"]["value"])
        self.assertEqual(output["measurements"]["deterministic_cases"], 2)

    def test_metrics_use_recorded_telemetry_and_baseline_overlap(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            inputs = self._write_inputs(root, include_novel=True)
            data = json.loads(inputs["artifact"].read_text(encoding="utf-8"))
            for case in data["cases"]:
                for runs in case["baselines"].values():
                    for run in runs:
                        run["telemetry"] = build_command_telemetry(run["wall_ms"])
                        run["evidence"] = {
                            "status": "measured",
                            "keys": ["security.secret-candidate"],
                            "comparison": {
                                "status": "measured",
                                "keys": ["security.secret-candidate"],
                                "scheme": "review-exact-v1",
                            },
                        }
                for review in case["reviews"]:
                    review["telemetry"] = {
                        "schema_version": 1,
                        "events": [
                            {"name": "process_started", "elapsed_ms": 0.0},
                            {"name": "evidence_ready", "elapsed_ms": 5.0},
                            {"name": "decision_ready", "elapsed_ms": 10.0},
                            {"name": "process_finished", "elapsed_ms": review["wall_ms"]},
                        ],
                    }
            inputs["artifact"].write_text(json.dumps(data), encoding="utf-8")
            pilot = root / "pilot.toml"
            pilot.write_text(
                render_pilot_template(
                    inputs["artifact"], inputs["manifest"], inputs["differential"], inputs["rules"], inputs["zoo"], "expert"
                )
                .replace('label = ""', 'label = "no-defect"', 1)
                .replace('rationale = ""', 'rationale = "reviewed first case"', 1)
                .replace('label = ""', 'label = "uncertain"', 1)
                .replace('rationale = ""', 'rationale = "insufficient context"', 1),
                encoding="utf-8",
            )
            output = build_pilot_metrics(
                inputs["artifact"], pilot, inputs["manifest"], inputs["differential"], inputs["rules"], inputs["zoo"]
            )
        self.assertEqual(output["measurements"]["time_to_first_useful_evidence"]["status"], "measured")
        self.assertEqual(output["measurements"]["time_to_first_useful_evidence"]["median_ms"], 5.0)
        self.assertEqual(output["measurements"]["decision_latency"]["median_ms"], 10.0)
        self.assertEqual(output["measurements"]["duplicate_work"]["overlap_count"], 1)

    def test_metrics_do_not_infer_overlap_from_unmapped_baseline_keys(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            inputs = self._write_inputs(root, include_novel=True)
            data = json.loads(inputs["artifact"].read_text(encoding="utf-8"))
            for case in data["cases"]:
                for runs in case["baselines"].values():
                    for run in runs:
                        run["evidence"] = {
                            "status": "measured",
                            "keys": ["python.tests:tests/test_api.py::test_create:failed"],
                            "comparison": {
                                "status": "unavailable",
                                "reason": "baseline evidence has no review-comparable identity mapping",
                                "scheme": "review-exact-v1",
                            },
                        }
            inputs["artifact"].write_text(json.dumps(data), encoding="utf-8")
            pilot = root / "pilot.toml"
            pilot.write_text(
                render_pilot_template(
                    inputs["artifact"], inputs["manifest"], inputs["differential"], inputs["rules"], inputs["zoo"], "expert"
                )
                .replace('label = ""', 'label = "no-defect"', 1)
                .replace('rationale = ""', 'rationale = "reviewed first case"', 1)
                .replace('label = ""', 'label = "uncertain"', 1)
                .replace('rationale = ""', 'rationale = "insufficient context"', 1),
                encoding="utf-8",
            )
            output = build_pilot_metrics(
                inputs["artifact"], pilot, inputs["manifest"], inputs["differential"], inputs["rules"], inputs["zoo"]
            )
        self.assertEqual(output["measurements"]["duplicate_work"]["status"], "unavailable")
        self.assertEqual(
            output["measurements"]["duplicate_work"]["reason"],
            "baseline evidence has no review-comparable identity mapping",
        )

    @staticmethod
    def _write_inputs(root: Path, include_novel: bool = False) -> dict[str, Path]:
        rules = root / "rules.md"
        zoo = root / "zoo.toml"
        manifest = root / "holdout.toml"
        differential = root / "differential.toml"
        artifact = root / "differential.json"
        rules.write_text("### `demo.rule` — Demo\n### `security.secret-candidate` — Secret\n", encoding="utf-8")
        zoo.write_text("", encoding="utf-8")
        manifest.write_text(
            """schema_version = 1
corpus = "test"
protocol = "dual-independent-adjudication-v1"

[[case]]
id = "case-one"
repo = "owner/one"
url = "https://github.com/owner/one.git"
pull_request = 1
base_sha = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
head_sha = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
merge_sha = "cccccccccccccccccccccccccccccccccccccccc"
language = "python"
source_kind = "merged-pull-request"
baseline_ids = ["python.compile"]
label_state = "pending"

[[case]]
id = "case-two"
repo = "owner/two"
url = "https://github.com/owner/two.git"
pull_request = 2
base_sha = "dddddddddddddddddddddddddddddddddddddddd"
head_sha = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
merge_sha = "ffffffffffffffffffffffffffffffffffffffff"
language = "python"
source_kind = "merged-pull-request"
baseline_ids = ["python.tests"]
label_state = "pending"
""",
            encoding="utf-8",
        )
        differential.write_text(
            """schema_version = 1
corpus = "test"
protocol = "differential-utility-v1"
repetitions = 3
measurements = ["novel-actionable-evidence", "duplicate-work", "time-to-first-useful-evidence", "decision-latency", "determinism", "resource-cost"]

[[case]]
id = "case-one"
baseline_ids = ["python.compile"]

[[case]]
id = "case-two"
baseline_ids = ["python.tests"]
""",
            encoding="utf-8",
        )
        cases = []
        for case_id, repo, base_sha, head_sha, merge_sha, baseline_id in (
            ("case-one", "owner/one", "a" * 40, "b" * 40, "c" * 40, "python.compile"),
            ("case-two", "owner/two", "d" * 40, "e" * 40, "f" * 40, "python.tests"),
        ):
            novel = ["security.secret-candidate"] if include_novel and case_id == "case-one" else []
            cases.append(
                {
                    "id": case_id,
                    "repo": repo,
                    "base_sha": base_sha,
                    "head_sha": head_sha,
                    "merge_sha": merge_sha,
                    "base_scan": {"status": "collected", "wall_ms": 2.0, "evidence_keys": []},
                    "baselines": {
                        baseline_id: [
                            {
                                "command": list(BASELINE_COMMANDS[baseline_id]),
                                "status": "passed",
                                "wall_ms": 10.0,
                            }
                            for _ in range(3)
                        ]
                    },
                    "reviews": [
                        {
                            "status": "collected",
                            "wall_ms": 20.0,
                            "stable_evidence_sha256": "1" * 64,
                            "in_diff_evidence_keys": novel,
                            "novel_in_diff_evidence_keys": novel,
                        }
                        for _ in range(3)
                    ],
                    "determinism": {"stable_evidence_deterministic": True},
                }
            )
        data = {
            "schema_version": 1,
            "corpus": "test",
            "protocol": "differential-utility-v1",
            "manifest_sha256": hashlib.sha256(manifest.read_bytes()).hexdigest(),
            "differential_manifest_sha256": hashlib.sha256(differential.read_bytes()).hexdigest(),
            "scanner": {"mode": "explicit", "version": "0.22.0", "report_schema_version": "0.26", "workspace_version": "0.22.0"},
            "repetitions": 3,
            "cases": cases,
            "label_state": "pending",
        }
        artifact.write_text(json.dumps(data), encoding="utf-8")
        validate_artifact(artifact, manifest, differential, rules, zoo)
        return {"artifact": artifact, "manifest": manifest, "differential": differential, "rules": rules, "zoo": zoo}


if __name__ == "__main__":
    unittest.main()
