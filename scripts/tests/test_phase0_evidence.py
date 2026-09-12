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
        self.assertEqual(
            [track["id"] for track in report["tracks"]],
            ["real-history", "differential-utility"],
        )
        self.assertTrue(all(track["protocol_status"] == "valid" for track in report["tracks"]))
        self.assertTrue(all(track["evidence_status"] == "artifact-missing" for track in report["tracks"]))
        self.assertTrue(all(track["label_state"] == "not-supplied" for track in report["tracks"]))

    def test_audit_mode_is_open_but_require_complete_fails_closed(self) -> None:
        report = phase0_evidence.build_report(self.paths)

        self.assertEqual(phase0_evidence.exit_code(report, require_complete=False), 0)
        self.assertEqual(phase0_evidence.exit_code(report, require_complete=True), 1)

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
        self.assertTrue(all(track["evidence_status"] == "protocol-invalid" for track in report["tracks"]))
        self.assertEqual(phase0_evidence.exit_code(report, require_complete=False), 1)

    def test_renderers_are_deterministic_and_json_is_machine_readable(self) -> None:
        report = phase0_evidence.build_report(self.paths)

        self.assertEqual(phase0_evidence.render_markdown(report), phase0_evidence.render_markdown(report))
        self.assertEqual(phase0_evidence.render_text(report), phase0_evidence.render_text(report))
        decoded = json.loads(phase0_evidence.render_json(report))
        self.assertEqual(decoded, report)
        self.assertIn("no precision, recall, or utility claim", phase0_evidence.render_markdown(report).lower())


if __name__ == "__main__":
    unittest.main()
