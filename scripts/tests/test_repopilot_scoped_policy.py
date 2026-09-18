from __future__ import annotations

import json
import subprocess
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "repopilot-scoped-policy.py"
WORKFLOW = Path(__file__).resolve().parents[2] / ".github/workflows/ci.yaml"
POLICY = Path(__file__).resolve().parents[2] / ".github/repopilot-scoped-policy.toml"


class ScopedPolicyTests(unittest.TestCase):
    def write_policy(
        self, root: Path, *, rules: str = '"architecture.circular-dependency"'
    ) -> Path:
        path = root / "policy.toml"
        path.write_text(
            "\n".join(
                [
                    "schema_version = 1",
                    'policy_id = "test-stable-v1"',
                    'owner = "test-owner"',
                    'profile = "default"',
                    'max_priority = "p1"',
                    f"rules = [{rules}]",
                    'rollback = "run with --mode advisory"',
                ]
            )
            + "\n",
            encoding="utf-8",
        )
        return path

    def write_report(
        self, root: Path, *, findings: list[dict[str, object]], profile: str = "default"
    ) -> Path:
        path = root / "report.json"
        path.write_text(
            json.dumps(
                {
                    "visibility_profile": profile,
                    "repopilot_version": "0.22.0",
                    "findings": findings,
                }
            ),
            encoding="utf-8",
        )
        return path

    def run_policy(
        self,
        root: Path,
        report: Path,
        policy: Path,
        *,
        mode: str = "advisory",
    ) -> tuple[subprocess.CompletedProcess[str], dict[str, object]]:
        output = root / "artifact.json"
        result = subprocess.run(
            [
                "python3",
                str(SCRIPT),
                "evaluate",
                "--report",
                str(report),
                "--policy",
                str(policy),
                "--output",
                str(output),
                "--mode",
                mode,
                "--repo",
                str(root),
            ],
            check=False,
            capture_output=True,
            text=True,
        )
        return result, json.loads(output.read_text(encoding="utf-8"))

    def finding(
        self, *, priority: str = "P1", lifecycle: str = "stable"
    ) -> dict[str, object]:
        return {
            "id": "architecture.circular-dependency:src/lib.rs:abc123",
            "rule_id": "architecture.circular-dependency",
            "risk": {"priority": priority},
            "provenance": {"rule_lifecycle": lifecycle},
            "evidence": [{"path": "src/lib.rs", "line_start": 12}],
        }

    def test_advisory_failure_never_changes_exit_code(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            report = self.write_report(root, findings=[self.finding()])
            policy = self.write_policy(root)

            result, artifact = self.run_policy(root, report, policy)

            self.assertEqual(result.returncode, 0)
            self.assertEqual(artifact["status"], "failed")
            self.assertEqual(artifact["mode"], "advisory")
            self.assertEqual(artifact["matched_count"], 1)
            self.assertIn("--mode advisory", str(artifact["rollback"]))

    def test_blocking_failure_returns_one_for_allowlisted_stable_rule(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            report = self.write_report(root, findings=[self.finding()])
            policy = self.write_policy(root)

            result, artifact = self.run_policy(root, report, policy, mode="blocking")

            self.assertEqual(result.returncode, 1)
            self.assertEqual(artifact["status"], "failed")
            self.assertEqual(artifact["matched"][0]["path"], "src/lib.rs")

    def test_blocking_ignores_findings_outside_scoped_policy(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            report = self.write_report(
                root,
                findings=[
                    self.finding(),
                    {
                        **self.finding(),
                        "id": "security.secret-candidate:config.py:def456",
                        "rule_id": "security.secret-candidate",
                    },
                ],
            )
            policy = self.write_policy(root)

            result, artifact = self.run_policy(root, report, policy, mode="blocking")

            self.assertEqual(result.returncode, 1)
            self.assertEqual(artifact["matched_count"], 1)

    def test_clean_scope_passes_in_blocking_mode(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            report = self.write_report(root, findings=[])
            policy = self.write_policy(root)

            result, artifact = self.run_policy(root, report, policy, mode="blocking")

            self.assertEqual(result.returncode, 0)
            self.assertEqual(artifact["status"], "passed")
            self.assertEqual(artifact["matched_count"], 0)

    def test_invalid_policy_is_recorded_without_advisory_failure(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            report = self.write_report(root, findings=[])
            policy = root / "policy.toml"
            policy.write_text("schema_version = 99\n", encoding="utf-8")

            result, artifact = self.run_policy(root, report, policy)

            self.assertEqual(result.returncode, 0)
            self.assertEqual(artifact["status"], "invalid")
            self.assertIn("schema_version", str(artifact["reason"]))

    def test_non_stable_match_is_unavailable_and_cannot_block(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            report = self.write_report(
                root, findings=[self.finding(lifecycle="preview")]
            )
            policy = self.write_policy(root)

            result, artifact = self.run_policy(root, report, policy, mode="blocking")

            self.assertEqual(result.returncode, 2)
            self.assertEqual(artifact["status"], "unavailable")
            self.assertIn("stable", str(artifact["reason"]))

    def test_profile_mismatch_is_unavailable(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            report = self.write_report(root, findings=[], profile="strict")
            policy = self.write_policy(root)

            result, artifact = self.run_policy(root, report, policy, mode="blocking")

            self.assertEqual(result.returncode, 2)
            self.assertEqual(artifact["status"], "unavailable")
            self.assertIn("profile", str(artifact["reason"]))

    def test_workflow_keeps_opt_in_blocking_disabled_by_default(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")
        self.assertIn("name: RepoPilot CI shadow policy", workflow)
        self.assertIn("vars.REPOPILOT_SCOPED_BLOCKING == 'true'", workflow)
        self.assertIn("--mode blocking", workflow)
        self.assertIn(".github/repopilot-scoped-policy.toml", workflow)
        self.assertIn("continue-on-error: true", workflow)

    def test_tracked_policy_declares_stable_owner_and_rollback(self) -> None:
        policy = POLICY.read_text(encoding="utf-8")
        self.assertIn('policy_id = "stable-architecture-v1"', policy)
        self.assertIn('owner = "repopilot-maintainers"', policy)
        self.assertIn('rules = ["architecture.circular-dependency"]', policy)
        self.assertIn("rollback", policy)


if __name__ == "__main__":
    unittest.main()
