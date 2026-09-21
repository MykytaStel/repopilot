from __future__ import annotations

import json
import sys
import tempfile
import unittest
from dataclasses import replace
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
PROJECT_ROOT = SCRIPTS_DIR.parent
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

import phase0_evidence  # noqa: E402


class Phase0EvidenceReportTests(unittest.TestCase):
    def setUp(self) -> None:
        self.paths = phase0_evidence.Phase0Paths.defaults(PROJECT_ROOT)

    def test_committed_protocols_are_valid_but_external_evidence_is_missing(self) -> None:
        report = phase0_evidence.build_report(self.paths)

        self.assertEqual(report["status"], "open")
        self.assertEqual(report["closure"], "open")
        self.assertEqual(report["protocol"], "phase0-evidence-closure-v2")
        self.assertEqual(report["decision"], "blocked")
        self.assertEqual(report["coverage"]["tracks_total"], 3)
        self.assertEqual(report["coverage"]["protocols_valid"], 3)
        self.assertEqual(report["coverage"]["evidence_valid"], 0)
        self.assertEqual(len(report["blocking_reasons"]), 3)
        self.assertEqual(len(report["next_actions"]), 3)
        self.assertEqual(
            [track["id"] for track in report["tracks"]],
            ["real-history", "differential-utility", "rule-quality"],
        )
        self.assertTrue(all(track["protocol_status"] == "valid" for track in report["tracks"]))
        self.assertEqual(
            [track["evidence_status"] for track in report["tracks"]],
            ["artifact-missing", "artifact-missing", "pending-labels"],
        )
        self.assertEqual(
            [track["label_state"] for track in report["tracks"]],
            ["not-supplied", "not-supplied", "committed-reviewed-labels"],
        )

        rule_quality = report["tracks"][2]
        self.assertEqual(rule_quality["observation"]["rules_total"], 54)
        self.assertEqual(rule_quality["observation"]["default_rules_measured"], 6)
        self.assertEqual(rule_quality["observation"]["default_rules_unmeasured"], 48)
        self.assertEqual(rule_quality["observation"]["labeled_default_findings"], 25)

    def test_audit_mode_is_open_but_require_complete_fails_closed(self) -> None:
        report = phase0_evidence.build_report(self.paths)

        self.assertEqual(phase0_evidence.exit_code(report, require_complete=False), 0)
        self.assertEqual(phase0_evidence.exit_code(report, require_complete=True), 1)

    def test_evidence_dir_discovers_only_current_packets(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            evidence_dir = Path(directory)
            current = evidence_dir / "real-history-run-current.json"
            current.write_text("{}\n", encoding="utf-8")
            (evidence_dir / "real-history-run.json").write_text("{}\n", encoding="utf-8")

            paths = self.paths.with_evidence_dir(evidence_dir)

        self.assertEqual(paths.real_history_artifact, current.resolve())
        self.assertIsNone(paths.differential_artifact)

    def test_renderers_show_current_artifact_observation_counts(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            artifact = Path(directory) / "current.json"
            artifact.write_text("{}\n", encoding="utf-8")
            report = phase0_evidence.build_report(
                replace(self.paths, real_history_artifact=artifact)
            )
            report["tracks"][0]["observation"] = {
                "cases": 2,
                "baseline_observations": 4,
                "review_observations": 2,
            }

        text = phase0_evidence.render_text(report)
        markdown = phase0_evidence.render_markdown(report)
        self.assertIn("artifact:", text)
        self.assertIn("current.json", text)
        self.assertIn("cases=2", text)
        self.assertIn("artifact", markdown)
        self.assertIn("cases=2", markdown)

    def test_renderers_show_rule_quality_coverage(self) -> None:
        report = phase0_evidence.build_report(self.paths)

        text = phase0_evidence.render_text(report)
        markdown = phase0_evidence.render_markdown(report)
        self.assertIn("rules_total=54", text)
        self.assertIn("default_rules_unmeasured=48", text)
        self.assertIn("labeled_default_findings=25", markdown)

    def test_tampered_real_history_artifact_is_invalid(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            artifact = Path(directory) / "real-history.json"
            artifact.write_text("{}\n", encoding="utf-8")
            report = phase0_evidence.build_report(replace(self.paths, real_history_artifact=artifact))

        track = report["tracks"][0]
        self.assertEqual(track["protocol_status"], "valid")
        self.assertEqual(track["evidence_status"], "invalid")
        self.assertIn("collection artifact", track["reason"])

    def test_invalid_protocol_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            manifest = Path(directory) / "manifest.toml"
            manifest.write_text("schema_version = 0\n", encoding="utf-8")
            report = phase0_evidence.build_report(replace(self.paths, manifest=manifest))

        self.assertEqual(report["status"], "invalid")
        self.assertEqual(report["decision"], "invalid")
        self.assertEqual(report["coverage"]["protocols_invalid"], 2)
        self.assertEqual(
            [track["evidence_status"] for track in report["tracks"]],
            ["protocol-invalid", "protocol-invalid", "pending-labels"],
        )
        self.assertEqual(phase0_evidence.exit_code(report, require_complete=False), 1)

    def test_complete_independent_tracks_are_verified(self) -> None:
        tracks = [
            {
                "id": "real-history",
                "protocol_status": "valid",
                "evidence_status": "valid",
                "scope": "independent-adjudication",
                "next_action": "retain the bounded result",
            },
            {
                "id": "differential-utility",
                "protocol_status": "valid",
                "evidence_status": "valid",
                "scope": "independent-adjudication",
                "next_action": "retain the bounded result",
            },
        ]

        decision = phase0_evidence.decision_summary(tracks)

        self.assertEqual(decision["decision"], "verified")
        self.assertEqual(decision["coverage"]["evidence_valid"], 2)
        self.assertEqual(decision["blocking_reasons"], [])
        self.assertEqual(len(decision["next_actions"]), 2)

    def test_renderers_are_deterministic_and_json_is_machine_readable(self) -> None:
        report = phase0_evidence.build_report(self.paths)

        self.assertEqual(phase0_evidence.render_markdown(report), phase0_evidence.render_markdown(report))
        self.assertEqual(phase0_evidence.render_text(report), phase0_evidence.render_text(report))
        decoded = json.loads(phase0_evidence.render_json(report))
        self.assertEqual(decoded, report)
        self.assertIn("no precision, recall, or utility claim", phase0_evidence.render_markdown(report).lower())
        self.assertIn("decision", decoded)
        self.assertIn("Decision", phase0_evidence.render_markdown(report))
        self.assertIn("Blocking reasons", phase0_evidence.render_markdown(report))
        self.assertIn("Next actions", phase0_evidence.render_text(report))


if __name__ == "__main__":
    unittest.main()
