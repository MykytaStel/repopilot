from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from changeproof_benchmark_runner import collect_benchmark  # noqa: E402


def write_fixture(root: Path) -> None:
    before = root / "sample" / "safe" / "before"
    after = root / "sample" / "safe" / "after"
    rationale = root / "sample" / "safe" / "patch"
    before.mkdir(parents=True)
    after.mkdir()
    rationale.mkdir()
    (before / "app.py").write_text("value = 1\n", encoding="utf-8")
    (after / "app.py").write_text("value = 2\n", encoding="utf-8")
    (rationale / "rationale.md").write_text("The controlled edit is harmless.", encoding="utf-8")


def write_manifest(root: Path) -> Path:
    manifest = root / "changeproof.toml"
    manifest.write_text(
        '''schema_version = 1
corpus = "runner-test"
protocol = "changeproof-benchmark-v1"

[[case]]
id = "sample"
fixture = "sample"
variant = "safe"
split = "evaluation"
profile = "default"
mutation_kind = "negative-control"
expected_verdict = "REVIEW"
claims_state = "unknown"
reason_codes_state = "unknown"
contracts_state = "unknown"
capabilities_state = "unknown"
oracle = "mutation-rationale-v1"
oracle_state = "known"
coverage_state = "unknown"
obligations_state = "unknown"
''',
        encoding="utf-8",
    )
    return manifest


def run(proof_hash: str) -> dict[str, object]:
    return {
        "status": "collected",
        "returncode": 0,
        "wall_ms": 1.5,
        "resource": {"status": "available", "peak_rss_kb": 12, "source": "posix-time-v1"},
        "proof_semantic_sha256": proof_hash,
        "evaluation": {"decision": {"status": "pass", "expected": "REVIEW", "observed": "REVIEW"}},
    }


class ChangeProofBenchmarkRunnerTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.fixture_root = self.root / "review-zoo"
        write_fixture(self.fixture_root)
        self.manifest = write_manifest(self.root)
        self.scanner = SimpleNamespace(
            mode="explicit",
            command=("repopilot",),
            version="0.22.0",
            report_schema_version="0.26",
            workspace_version="0.22.0",
            workspace_commit=None,
            workspace_dirty=None,
            version_mismatch_allowed=False,
        )

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def test_collect_runs_one_cold_and_two_warm_reviews(self) -> None:
        with patch("changeproof_benchmark_runner.prepare_scanner", return_value=self.scanner), patch(
            "changeproof_benchmark_runner.run_review", return_value=run("a" * 64)
        ) as mocked:
            artifact = collect_benchmark(self.root, self.manifest, self.fixture_root, "repopilot", timeout_seconds=30)

        self.assertEqual([item["phase"] for item in artifact["cases"][0]["runs"]], ["cold", "warm", "warm"])
        self.assertEqual(mocked.call_count, 3)
        self.assertTrue(artifact["cases"][0]["deterministic"])
        self.assertEqual(artifact["manifest_sha256"], __import__("hashlib").sha256(self.manifest.read_bytes()).hexdigest())

    def test_collect_reports_hash_disagreement_as_nondeterministic(self) -> None:
        with patch("changeproof_benchmark_runner.prepare_scanner", return_value=self.scanner), patch(
            "changeproof_benchmark_runner.run_review", side_effect=[run("a" * 64), run("b" * 64), run("b" * 64)]
        ):
            artifact = collect_benchmark(self.root, self.manifest, self.fixture_root, "repopilot", timeout_seconds=30)

        self.assertFalse(artifact["cases"][0]["deterministic"])
        self.assertEqual(artifact["cases"][0]["proof_semantic_hashes"], ["a" * 64, "b" * 64])

    def test_collect_requires_an_explicit_scanner_to_stay_offline(self) -> None:
        with patch("changeproof_benchmark_runner.prepare_scanner") as prepare:
            with self.assertRaisesRegex(Exception, "--scanner is required"):
                collect_benchmark(self.root, self.manifest, self.fixture_root, "", timeout_seconds=30)
        prepare.assert_not_called()


if __name__ == "__main__":
    unittest.main()
