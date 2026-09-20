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

from sandbox_contract import (  # noqa: E402
    SandboxManifestError,
    load_manifest,
    validate_artifact,
)
from sandbox_case import _normalize_findings  # noqa: E402
from sandbox_runner import DockerResult, SubprocessDockerAdapter, run_case, run_command  # noqa: E402


def manifest_text(sha: str) -> str:
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
sha = "{sha}"
language = "python"
framework = ""

[[case]]
id = "control-case"
project = "control"
image = "python:3.12@sha256:{"b" * 64}"
setup = ["python3", "-c", "print('setup')"]
oracle = ["python3", "-c", "print('oracle')"]
expected_oracle = "passed"
'''


class FakeDocker:
    def __init__(self, status: str = "passed") -> None:
        self.status = status

    def run(
        self,
        command: tuple[str, ...],
        cwd: Path,
        image: str,
        timeout_seconds: int,
        policy: object,
        run_id: str,
    ) -> DockerResult:
        return DockerResult(
            status=self.status,
            returncode=0 if self.status == "passed" else None,
            stdout_sha256="1" * 64,
            stderr_sha256="2" * 64,
            wall_ms=1.0,
        )


class SandboxRunnerTests(unittest.TestCase):
    def make_repo(self, root: Path) -> tuple[Path, str]:
        source = root / "source"
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
        return source, sha

    def test_normalization_keeps_evidence_location_without_snippet(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            checkout = Path(tmp) / "checkout"
            checkout.mkdir()
            payload = json.dumps(
                {
                    "findings": [
                        {
                            "rule_id": "demo.rule",
                            "evidence": [
                                {
                                    "path": str(checkout / "src/main.rs"),
                                    "line_start": 12,
                                    "snippet": "secret-value",
                                }
                            ],
                        }
                    ]
                }
            ).encode()

            normalized = _normalize_findings(payload, checkout)

        self.assertEqual(
            normalized["findings"],
            [
                {
                    "rule_id": "demo.rule",
                    "path": "src/main.rs",
                    "line": 12,
                    "in_diff": None,
                    "evidence_sha256": "31160254d1297393d2ad00e1c01851aec834361e02c524b89fe06aff2879ce6a",
                }
            ],
        )

    def test_dry_run_writes_reproducible_artifact_and_cleanup_receipt(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source, sha = self.make_repo(root)
            manifest_path = root / "manifest.toml"
            manifest_path.write_text(manifest_text(sha), encoding="utf-8")
            output = root / "out" / "result.json"

            result = run_case(
                manifest_path,
                "control-case",
                output,
                source_override=source,
                dry_run=True,
            )

            self.assertEqual(result["status"], "dry-run")
            self.assertTrue(output.is_file())
            self.assertEqual(result["cleanup"]["status"], "complete")
            self.assertEqual(result["inputs"]["source_sha"], sha)
            self.assertEqual(
                validate_artifact(output, manifest_path)["status"], "valid"
            )

    def test_docker_command_has_measured_network_and_only_workspace_mount(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            manifest_path = root / "manifest.toml"
            manifest_path.write_text(manifest_text("a" * 40), encoding="utf-8")
            manifest = load_manifest(manifest_path)
            command = SubprocessDockerAdapter.build_command(
                ("python3", "-c", "pass"),
                root,
                manifest.cases[0].image,
                manifest.policy,
                "repopilot-sandbox-test",
            )

        rendered = " ".join(command)
        self.assertIn("--network none", rendered)
        self.assertIn("--pull=never", rendered)
        self.assertIn("--cpus 2.0", rendered)
        self.assertIn("--memory 4096m", rendered)
        self.assertIn("dst=/workspace", rendered)
        self.assertNotIn("/home", rendered)
        self.assertNotIn("docker.sock", rendered)
        self.assertNotIn("SSH_AUTH_SOCK", rendered)

    def test_docker_adapter_reports_missing_cli_without_running_host_oracle(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            manifest_path = root / "manifest.toml"
            manifest_path.write_text(manifest_text("a" * 40), encoding="utf-8")
            manifest = load_manifest(manifest_path)
            result = SubprocessDockerAdapter("sandbox-docker-does-not-exist").run(
                ("python3", "-c", "pass"),
                root,
                manifest.cases[0].image,
                5,
                manifest.policy,
                "test-run",
            )

        self.assertEqual(result.status, "unavailable")
        self.assertIn("CLI", result.reason or "")

    def test_static_analysis_persists_only_normalized_findings(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source, sha = self.make_repo(root)
            manifest_path = root / "manifest.toml"
            manifest_path.write_text(manifest_text(sha), encoding="utf-8")
            output = root / "result.json"
            scanner = (
                sys.executable,
                "-c",
                "import json; print(json.dumps({'findings': [{'rule_id': 'demo.rule', 'path': 'main.py', 'line': 1, 'in_diff': True, 'snippet': 'secret'}]}))",
            )

            result = run_case(
                manifest_path,
                "control-case",
                output,
                source_override=source,
                scanner=scanner,
                docker=FakeDocker(),
            )

        self.assertEqual(result["status"], "passed")
        analysis = next(
            phase for phase in result["phases"] if phase["name"] == "analyze"
        )
        self.assertEqual(
            analysis["result"]["normalized_findings"]["status"], "measured"
        )
        self.assertEqual(
            analysis["result"]["normalized_findings"]["findings"],
            [{"rule_id": "demo.rule", "path": "main.py", "line": 1, "in_diff": True}],
        )
        self.assertNotIn("secret", json.dumps(result))

    def test_large_scanner_json_is_normalized_without_stdout_truncation(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source, sha = self.make_repo(root)
            manifest_path = root / "manifest.toml"
            manifest_path.write_text(manifest_text(sha), encoding="utf-8")
            output = root / "result.json"
            scanner = (
                sys.executable,
                "-c",
                "import json,sys; path=sys.argv[sys.argv.index('--output')+1]; "
                "open(path, 'w').write(json.dumps({'findings': ["
                "{'rule_id': 'demo.rule', 'path': 'main.py', 'line': 1, "
                "'in_diff': True, 'padding': 'x' * 300} for _ in range(400)]}))",
            )

            result = run_case(
                manifest_path,
                "control-case",
                output,
                source_override=source,
                scanner=scanner,
                docker=FakeDocker(),
            )

        self.assertEqual(result["status"], "passed")
        analysis = next(
            phase for phase in result["phases"] if phase["name"] == "analyze"
        )
        normalized = analysis["result"]["normalized_findings"]
        self.assertEqual(normalized["status"], "measured")
        self.assertEqual(normalized["count"], 400)
        self.assertEqual(len(analysis["result"]["report_sha256"]), 64)

    def test_changed_analysis_mode_passes_flag_and_records_mode(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source, sha = self.make_repo(root)
            manifest_path = root / "manifest.toml"
            manifest_path.write_text(
                manifest_text(sha).replace(
                    'expected_oracle = "passed"',
                    'expected_oracle = "passed"\nanalysis_mode = "changed"',
                ),
                encoding="utf-8",
            )
            output = root / "result.json"
            scanner = (
                sys.executable,
                "-c",
                "import json,sys; print(json.dumps({'findings': [{'rule_id': 'demo.rule', 'path': 'main.py', 'line': 1, 'in_diff': '--changed' in sys.argv}]} if '--changed' in sys.argv else {'findings': []}))",
            )

            result = run_case(
                manifest_path,
                "control-case",
                output,
                source_override=source,
                scanner=scanner,
                docker=FakeDocker(),
            )

        self.assertEqual(result["provenance"]["analysis_mode"], "changed")
        analysis = next(
            phase for phase in result["phases"] if phase["name"] == "analyze"
        )
        self.assertEqual(analysis["result"]["normalized_findings"]["count"], 1)

    def test_artifact_validator_rejects_analysis_mode_drift(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source, sha = self.make_repo(root)
            manifest_path = root / "manifest.toml"
            manifest_path.write_text(
                manifest_text(sha).replace(
                    'expected_oracle = "passed"',
                    'expected_oracle = "passed"\nanalysis_mode = "changed"',
                ),
                encoding="utf-8",
            )
            output = root / "result.json"
            run_case(
                manifest_path,
                "control-case",
                output,
                source_override=source,
                dry_run=True,
            )
            data = json.loads(output.read_text(encoding="utf-8"))
            data["provenance"]["analysis_mode"] = "default"
            output.write_text(json.dumps(data), encoding="utf-8")

            with self.assertRaisesRegex(SandboxManifestError, "does not match"):
                validate_artifact(output, manifest_path)

    def test_mutation_patch_is_applied_and_reverted_in_run_owned_copy(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source, sha = self.make_repo(root)
            original = (source / "main.py").read_text(encoding="utf-8")
            (source / "main.py").write_text(
                original + "print('mutated')\n", encoding="utf-8"
            )
            patch = root / "mutation.patch"
            patch.write_bytes(
                subprocess.check_output(["git", "diff", "--", "main.py"], cwd=source)
            )
            (source / "main.py").write_text(original, encoding="utf-8")
            manifest_path = root / "manifest.toml"
            manifest_path.write_text(
                manifest_text(sha).replace(
                    'expected_oracle = "passed"',
                    'patch = "mutation.patch"\nexpected_oracle = "passed"',
                ),
                encoding="utf-8",
            )
            output = root / "result.json"
            result = run_case(
                manifest_path,
                "control-case",
                output,
                source_override=source,
                docker=FakeDocker(),
                work_root=root / "work",
            )

            case_dirs = list((root / "work").glob("control-case-*"))
            reverted = (case_dirs[0] / "after" / "main.py").read_text(encoding="utf-8")

        self.assertEqual(result["status"], "unavailable")
        self.assertEqual(
            [phase["name"] for phase in result["phases"]],
            ["prepare", "baseline", "mutate", "analyze", "oracle", "revert", "collect"],
        )
        self.assertEqual(result["phases"][2]["status"], "passed")
        self.assertEqual(result["phases"][4]["status"], "passed")
        self.assertEqual(result["phases"][5]["status"], "passed")
        self.assertEqual(reverted, original)

    def test_source_sha_mismatch_leaves_failure_artifact(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source, _sha = self.make_repo(root)
            manifest_path = root / "manifest.toml"
            manifest_path.write_text(manifest_text("a" * 40), encoding="utf-8")
            output = root / "result.json"

            result = run_case(
                manifest_path, "control-case", output, source_override=source
            )

            self.assertEqual(result["status"], "failed")
            self.assertIn("SHA", result["reason"])
            self.assertTrue(output.is_file())
            persisted = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(persisted["cleanup"]["status"], "complete")

    def test_timed_command_terminates_on_timeout(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            result = run_command(
                (sys.executable, "-c", "import time; time.sleep(1)"),
                Path(tmp),
                0.01,
                "run-test",
            )

        self.assertEqual(result.status, "timeout")
        self.assertEqual(result.returncode, None)
        self.assertGreaterEqual(result.wall_ms, 0)
        self.assertEqual(len(result.stdout_sha256), 64)

    def test_timed_command_marks_bounded_output(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            result = run_command(
                (sys.executable, "-c", "print('x' * 100)"), Path(tmp), 5, "run-test", 16
            )

        self.assertTrue(result.stdout_truncated)
        self.assertFalse(result.stderr_truncated)


if __name__ == "__main__":
    unittest.main()
