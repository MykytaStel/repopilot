from __future__ import annotations

import hashlib
import importlib
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

integrity_corpus_eval = importlib.import_module("integrity_corpus_eval")
integrity_corpus_report = importlib.import_module("integrity_corpus_report")
evaluate = integrity_corpus_eval.evaluate
load_cached_results = integrity_corpus_eval.load_cached_results
catches = integrity_corpus_report.catches


class IntegrityCorpusEvaluationCacheTests(unittest.TestCase):
    def test_results_are_separate_for_different_binary_contents(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            binary_a = self._fake_binary(root / "repopilot-a", "repopilot test-a", "integrity.test-removed")
            binary_b = self._fake_binary(root / "repopilot-b", "repopilot test-b", "integrity.assertions-removed")
            entry = {"id": "ih-cache", "repo": "example/repo", "number": 1, "base_sha": "base", "head_sha": "head"}
            with (
                patch("integrity_corpus_eval.changed_files", return_value=[]),
                patch("integrity_corpus_eval.merge_base", return_value="merge-base"),
                patch("integrity_corpus_eval.build_repo", return_value=("base", "head")),
            ):
                evaluate([entry], root / "cache", binary_a)
                evaluate([entry], root / "cache", binary_b)

            sha_a = hashlib.sha256(binary_a.read_bytes()).hexdigest()
            sha_b = hashlib.sha256(binary_b.read_bytes()).hexdigest()
            result_a = json.loads((root / "cache" / "results" / sha_a / "ih-cache.json").read_text())
            result_b = json.loads((root / "cache" / "results" / sha_b / "ih-cache.json").read_text())
            self.assertNotEqual(sha_a, sha_b)
            self.assertEqual(result_a["binary"], "repopilot test-a")
            self.assertEqual(result_b["binary"], "repopilot test-b")
            self.assertEqual(result_a["binary_sha256"], sha_a)
            self.assertEqual(result_b["signals"][0]["kind"], "integrity.assertions-removed")

    def test_report_requires_binary_when_multiple_evaluations_exist(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            binary_a = self._fake_binary(root / "repopilot-a", "repopilot test-a", "integrity.test-removed")
            binary_b = self._fake_binary(root / "repopilot-b", "repopilot test-b", "integrity.assertions-removed")
            results = root / "cache" / "results"
            for binary, kind in ((binary_a, "integrity.test-removed"), (binary_b, "integrity.assertions-removed")):
                run = results / hashlib.sha256(binary.read_bytes()).hexdigest()
                run.mkdir(parents=True)
                (run / "ih-cache.json").write_text(json.dumps({"signals": [{"kind": kind}]}))

            with self.assertRaisesRegex(ValueError, "multiple binary evaluations"):
                load_cached_results(root / "cache")
            selected = load_cached_results(root / "cache", binary_b)
            self.assertEqual(selected["ih-cache"]["signals"][0]["kind"], "integrity.assertions-removed")

    def test_report_displays_executable_fingerprint(self) -> None:
        manifest = [{"id": "ih-cache", "group": "agent"}]
        labels = {"ih-cache": {"kinds": ["test-removed"]}}
        results = {
            "ih-cache": {
                "binary": "repopilot test",
                "binary_sha256": "a" * 64,
                "signals": [{"kind": "integrity.test-removed"}],
            }
        }
        rendered = "\n".join(catches(manifest, labels, results, "blind labels"))
        self.assertIn("Binary: repopilot test (aaaaaaaaaaaa).", rendered)

    def test_legacy_cache_requires_explicit_opt_in(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            cache = Path(temp_dir) / "cache"
            results = cache / "results"
            results.mkdir(parents=True)
            (results / "ih-cache.json").write_text(json.dumps({"binary": "unknown", "signals": []}))

            self.assertEqual(load_cached_results(cache), {})
            legacy = load_cached_results(cache, allow_legacy_cache=True)
            self.assertEqual(legacy["ih-cache"]["binary"], "unknown")

    def test_empty_binary_run_is_not_reported_as_committed_baseline(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            binary = self._fake_binary(root / "repopilot", "repopilot test", "integrity.test-removed")
            run = root / "cache" / "results" / hashlib.sha256(binary.read_bytes()).hexdigest()
            run.mkdir(parents=True)
            with self.assertRaisesRegex(ValueError, "no PR results"):
                load_cached_results(root / "cache", binary)

    @staticmethod
    def _fake_binary(path: Path, version: str, signal_kind: str) -> Path:
        report = {
            "tiered_signals": {
                "definitely": [{"kind": signal_kind, "path": "tests/test.py", "line": 1, "detail": version}],
                "maybe": [],
                "noise": [],
            }
        }
        code = (
            "#!/usr/bin/env python3\n"
            "import json, sys\n"
            f"version = {version!r}\n"
            f"report = {json.dumps(report)!r}\n"
            "if sys.argv[1] == '--version':\n"
            "    print(version)\n"
            "else:\n"
            "    print(report)\n"
        )
        path.write_text(code, encoding="utf-8")
        path.chmod(0o755)
        return path


if __name__ == "__main__":
    unittest.main()
