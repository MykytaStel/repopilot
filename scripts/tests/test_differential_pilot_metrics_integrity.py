from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
TESTS_DIR = Path(__file__).resolve().parent
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))
if str(TESTS_DIR) not in sys.path:
    sys.path.insert(0, str(TESTS_DIR))

from differential_pilot import render_pilot_template  # noqa: E402
from differential_pilot_metrics import (  # noqa: E402
    build_pilot_metrics,
    validate_pilot_metrics,
    write_pilot_metrics,
)
from differential_metrics_report import render_differential_metrics_report  # noqa: E402
from test_differential_pilot import DifferentialPilotTests  # noqa: E402


class DifferentialPilotMetricsIntegrityTests(unittest.TestCase):
    def _inputs(self, root: Path) -> tuple[dict[str, Path], Path]:
        inputs = DifferentialPilotTests._write_inputs(root, include_novel=True)
        pilot = root / "pilot.toml"
        pilot.write_text(
            render_pilot_template(
                inputs["artifact"],
                inputs["manifest"],
                inputs["differential"],
                inputs["rules"],
                inputs["zoo"],
                "expert",
            )
            .replace('label = ""', 'label = "no-defect"', 1)
            .replace('rationale = ""', 'rationale = "reviewed first case"', 1)
            .replace('label = ""', 'label = "uncertain"', 1)
            .replace('rationale = ""', 'rationale = "insufficient context"', 1),
            encoding="utf-8",
        )
        return inputs, pilot

    def test_validator_recomputes_and_rejects_tampered_metrics(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            inputs, pilot = self._inputs(Path(tmp))
            metrics = Path(tmp) / "metrics.json"
            write_pilot_metrics(
                metrics,
                inputs["artifact"],
                pilot,
                inputs["manifest"],
                inputs["differential"],
                inputs["rules"],
                inputs["zoo"],
            )
            validated = validate_pilot_metrics(
                metrics,
                inputs["artifact"],
                pilot,
                inputs["manifest"],
                inputs["differential"],
                inputs["rules"],
                inputs["zoo"],
            )
            self.assertEqual(validated["status"], "valid")
            data = json.loads(metrics.read_text(encoding="utf-8"))
            data["counts"]["fp"] += 1
            metrics.write_text(json.dumps(data), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "does not match recomputed score"):
                validate_pilot_metrics(
                    metrics,
                    inputs["artifact"],
                    pilot,
                    inputs["manifest"],
                    inputs["differential"],
                    inputs["rules"],
                    inputs["zoo"],
                )

    def test_report_is_deterministic_and_keeps_unavailable_measurements_explicit(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            inputs, pilot = self._inputs(Path(tmp))
            data = build_pilot_metrics(
                inputs["artifact"],
                pilot,
                inputs["manifest"],
                inputs["differential"],
                inputs["rules"],
                inputs["zoo"],
            )
        report = render_differential_metrics_report(data)
        self.assertEqual(report, render_differential_metrics_report(data))
        self.assertIn("single-expert exploratory pilot", report)
        self.assertIn("decision latency", report.lower())
        self.assertIn("unavailable", report.lower())
        self.assertIn("security.secret-candidate", report)


if __name__ == "__main__":
    unittest.main()
