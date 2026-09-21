from __future__ import annotations

import json
import os
import stat
import subprocess
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "repopilot-shadow.sh"
WORKFLOW = Path(__file__).resolve().parents[2] / ".github/workflows/ci.yaml"


class RepoPilotShadowTests(unittest.TestCase):
    def make_analyzer(self, root: Path, mode: str = "pass") -> Path:
        analyzer = root / "fake-repopilot"
        analyzer.write_text(
            """#!/bin/sh
if [ "${1:-}" = "--version" ]; then
  echo "fake-repopilot 9.9.0"
  exit 0
fi
output=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "--output" ]; then
    output="$2"
    shift 2
  else
    shift
  fi
done
if [ -n "$output" ]; then
  case "${FAKE_MODE:-pass}" in
    invalid) printf '%s\\n' 'not-json' > "$output" ;;
    *) printf '%s\\n' '{"findings": []}' > "$output" ;;
  esac
fi
if [ "${FAKE_MODE:-pass}" = "sleep" ]; then
  sleep 5
fi
exit "${FAKE_EXIT:-0}"
""",
            encoding="utf-8",
        )
        analyzer.chmod(analyzer.stat().st_mode | stat.S_IXUSR)
        return analyzer

    def run_shadow(
        self,
        output_dir: Path,
        analyzer: Path,
        *,
        env: dict[str, str] | None = None,
        timeout: int = 10,
    ) -> subprocess.CompletedProcess[str]:
        command = [
            "bash",
            str(SCRIPT),
            "--output-dir",
            str(output_dir),
            "--repo",
            str(output_dir),
            "--binary",
            str(analyzer),
            "--timeout-seconds",
            str(timeout),
        ]
        process_env = os.environ.copy()
        if env:
            process_env.update(env)
        return subprocess.run(command, check=False, capture_output=True, text=True, env=process_env)

    def read_metadata(self, output_dir: Path) -> dict[str, object]:
        return json.loads((output_dir / "shadow.json").read_text(encoding="utf-8"))

    def test_passed_report_records_policy_and_hash(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            analyzer = self.make_analyzer(root)
            output_dir = root / "shadow"
            result = self.run_shadow(output_dir, analyzer)

            self.assertEqual(result.returncode, 0)
            metadata = self.read_metadata(output_dir)
            self.assertEqual(metadata["schema_version"], "repopilot-shadow-1")
            self.assertEqual(metadata["status"], "passed")
            self.assertEqual(metadata["policy"], {"profile": "strict", "fail_on_priority": "p1"})
            self.assertEqual(metadata["analyzer_version"], "fake-repopilot 9.9.0")
            self.assertRegex(str(metadata["report_sha256"]), r"^[0-9a-f]{64}$")

    def test_policy_failure_is_advisory_and_preserves_valid_report(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            analyzer = self.make_analyzer(root)
            output_dir = root / "shadow"
            result = self.run_shadow(output_dir, analyzer, env={"FAKE_EXIT": "1"})

            self.assertEqual(result.returncode, 0)
            metadata = self.read_metadata(output_dir)
            self.assertEqual(metadata["status"], "failed")
            self.assertEqual(metadata["exit_code"], 1)
            self.assertTrue((output_dir / "report.json").is_file())

    def test_invalid_report_is_never_classified_as_passed(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            analyzer = self.make_analyzer(root)
            output_dir = root / "shadow"
            result = self.run_shadow(output_dir, analyzer, env={"FAKE_MODE": "invalid"})

            self.assertEqual(result.returncode, 0)
            metadata = self.read_metadata(output_dir)
            self.assertEqual(metadata["status"], "invalid")
            self.assertIn("JSON", str(metadata["reason"]))

    def test_missing_analyzer_is_recorded_as_unavailable(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            output_dir = root / "shadow"
            result = self.run_shadow(output_dir, root / "does-not-exist")

            self.assertEqual(result.returncode, 0)
            metadata = self.read_metadata(output_dir)
            self.assertEqual(metadata["status"], "unavailable")
            self.assertIn("executable", str(metadata["reason"]))

    def test_timeout_is_recorded_as_unavailable(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            analyzer = self.make_analyzer(root)
            output_dir = root / "shadow"
            result = self.run_shadow(
                output_dir,
                analyzer,
                env={"FAKE_MODE": "sleep"},
                timeout=1,
            )

            self.assertEqual(result.returncode, 0)
            metadata = self.read_metadata(output_dir)
            self.assertEqual(metadata["status"], "unavailable")
            self.assertIn("timed out", str(metadata["reason"]))

    def test_ci_keeps_blocking_gate_and_adds_non_blocking_shadow_artifact(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")
        self.assertIn("name: RepoPilot CI gate", workflow)
        self.assertIn("cargo run -- scan . --fail-on-priority p1", workflow)
        self.assertIn("name: RepoPilot CI shadow policy", workflow)
        self.assertIn("continue-on-error: true", workflow)
        self.assertIn("name: repopilot-shadow", workflow)
        self.assertIn("/tmp/repopilot-shadow", workflow)


if __name__ == "__main__":
    unittest.main()
