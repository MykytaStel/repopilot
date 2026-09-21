from __future__ import annotations

import json
import shutil
import sys
import tempfile
import unittest
from dataclasses import replace
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
PROJECT_ROOT = SCRIPTS_DIR.parent
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

import phase0_rule_quality as rule_quality  # noqa: E402
from phase0_evidence_model import Phase0Paths  # noqa: E402


class Phase0RuleQualityTests(unittest.TestCase):
    def setUp(self) -> None:
        self.paths = Phase0Paths.defaults(PROJECT_ROOT)

    def test_committed_scorecard_reports_measured_and_unmeasured_rules(self) -> None:
        first = rule_quality.inspect_rule_quality(self.paths)
        second = rule_quality.inspect_rule_quality(self.paths)

        self.assertEqual(first["observation"]["rules_total"], 54)
        self.assertEqual(first["observation"]["default_rules_measured"], 6)
        self.assertEqual(first["observation"]["default_rules_unmeasured"], 48)
        self.assertEqual(first["observation"]["labeled_default_findings"], 25)
        self.assertEqual(first["observation"]["strict_sampled_rules"], 10)
        self.assertEqual(first["observation"]["strict_sampled_findings"], 32)
        self.assertEqual(first["hashes"], second["hashes"])
        self.assertEqual(first["evidence_status"], "pending-labels")
        self.assertIn("production precision or recall", first["claim_boundary"])

    def test_stale_scorecard_is_a_protocol_failure(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            scorecard = Path(directory) / "rule-scorecard.md"
            scorecard.write_text("stale\n", encoding="utf-8")
            paths = replace(self.paths, rule_scorecard=scorecard)

            track = rule_quality.rule_quality_track(paths)

        self.assertEqual(track["protocol_status"], "invalid")
        self.assertEqual(track["evidence_status"], "invalid")
        self.assertEqual(track["scope"], "none")
        self.assertIn("stale", track["reason"])

    def test_missing_snapshot_stays_artifact_missing(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            paths = replace(self.paths, zoo_snapshot_dir=Path(directory))

            track = rule_quality.rule_quality_track(paths)

        self.assertEqual(track["protocol_status"], "valid")
        self.assertEqual(track["evidence_status"], "artifact-missing")
        self.assertEqual(track["scope"], "protocol-only")
        self.assertIn("missing snapshot", track["reason"])

    def test_snapshot_denominator_mismatch_is_invalid(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            snapshot_dir = Path(directory) / "snapshots"
            shutil.copytree(self.paths.zoo_snapshot_dir, snapshot_dir)
            snapshot = snapshot_dir / "ripgrep.json"
            payload = json.loads(snapshot.read_text(encoding="utf-8"))
            payload["default"]["visible_total"] += 1
            snapshot.write_text(json.dumps(payload), encoding="utf-8")
            paths = replace(self.paths, zoo_snapshot_dir=snapshot_dir)

            track = rule_quality.rule_quality_track(paths)

        self.assertEqual(track["protocol_status"], "valid")
        self.assertEqual(track["evidence_status"], "invalid")
        self.assertEqual(track["scope"], "none")
        self.assertIn("do not match", track["reason"])


if __name__ == "__main__":
    unittest.main()
