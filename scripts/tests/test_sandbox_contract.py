from __future__ import annotations

import json
import hashlib
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


def manifest_text(*, command: str = "python3") -> str:
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
setup = ["{command}", "-m", "compileall", "-q", "."]
oracle = ["{command}", "-c", "print('ok')"]
expected_oracle = "passed"
'''


class SandboxContractTests(unittest.TestCase):
    def test_manifest_loads_pinned_project_and_policy(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "manifest.toml"
            path.write_text(manifest_text(), encoding="utf-8")

            manifest = load_manifest(path)

        self.assertEqual(manifest.protocol, "repopilot-validation-sandbox-v1")
        self.assertEqual(manifest.projects[0].sha, "a" * 40)
        self.assertEqual(manifest.cases[0].oracle[0], "python3")
        self.assertEqual(manifest.policy.memory_mb, 4096)

    def test_manifest_loads_changed_analysis_mode(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "manifest.toml"
            path.write_text(
                manifest_text() + 'analysis_mode = "changed"\n', encoding="utf-8"
            )

            manifest = load_manifest(path)

        self.assertEqual(manifest.cases[0].analysis_mode, "changed")

    def test_manifest_loads_expected_rule_ids(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "manifest.toml"
            path.write_text(
                manifest_text() + 'expected_rule_ids = ["demo.rule"]\n',
                encoding="utf-8",
            )

            manifest = load_manifest(path)

        self.assertEqual(manifest.cases[0].expected_rule_ids, ("demo.rule",))

    def test_manifest_loads_analysis_profile(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "manifest.toml"
            path.write_text(manifest_text() + 'profile = "strict"\n', encoding="utf-8")

            manifest = load_manifest(path)

        self.assertEqual(manifest.cases[0].profile, "strict")

    def test_manifest_rejects_unsupported_analysis_mode(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "manifest.toml"
            path.write_text(
                manifest_text() + 'analysis_mode = "invalid"\n', encoding="utf-8"
            )

            with self.assertRaisesRegex(SandboxManifestError, "analysis_mode"):
                load_manifest(path)

    def test_manifest_rejects_shell_metacharacters_in_commands(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "manifest.toml"
            path.write_text(manifest_text(command="python3;rm"), encoding="utf-8")

            with self.assertRaisesRegex(SandboxManifestError, "allowlisted"):
                load_manifest(path)

    def test_manifest_rejects_unpinned_image(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "manifest.toml"
            path.write_text(
                manifest_text().replace("@sha256:" + "b" * 64, ""), encoding="utf-8"
            )

            with self.assertRaisesRegex(SandboxManifestError, "digest"):
                load_manifest(path)

    def test_manifest_requires_patch_for_mutation_case(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "manifest.toml"
            path.write_text(
                manifest_text() + 'mutation_kind = "violation"\n', encoding="utf-8"
            )

            with self.assertRaisesRegex(SandboxManifestError, "require a patch"):
                load_manifest(path)

    def test_artifact_validator_requires_cleanup_receipt(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            manifest_path = root / "manifest.toml"
            manifest_path.write_text(manifest_text(), encoding="utf-8")
            artifact_path = root / "result.json"
            manifest_sha = hashlib.sha256(manifest_path.read_bytes()).hexdigest()
            artifact_path.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "protocol": "repopilot-validation-sandbox-v1",
                        "corpus": "sandbox-control-v1",
                        "manifest_sha256": manifest_sha,
                        "case_id": "control-case",
                        "project_id": "control",
                        "status": "failed",
                        "phases": [],
                        "inputs": {
                            "project_sha": "a" * 40,
                            "image": "python:3.12@sha256:" + "b" * 64,
                            "source_sha": None,
                            "patch_sha256": None,
                        },
                    }
                ),
                encoding="utf-8",
            )

            with self.assertRaisesRegex(SandboxManifestError, "cleanup"):
                validate_artifact(artifact_path, manifest_path)


if __name__ == "__main__":
    unittest.main()
