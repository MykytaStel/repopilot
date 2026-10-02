from __future__ import annotations

import contextlib
import io
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import adoption_stats as stats


def public_response(url: str) -> dict:
    if url.startswith("https://api.npmjs.org/downloads/"):
        return {"downloads": 12, "start": "2026-09-25", "end": "2026-10-01"}
    if url == "https://crates.io/api/v1/crates/repopilot":
        return {
            "crate": {"downloads": 100, "recent_downloads": 20},
            "versions": [{"num": "0.23.0", "downloads": 100}],
        }
    if url.startswith("https://registry.modelcontextprotocol.io/v0/servers?"):
        return {"servers": [], "metadata": {"count": 0}}
    raise AssertionError(f"unexpected public request: {url}")


class AdoptionStatsTests(unittest.TestCase):
    def setUp(self) -> None:
        patcher = mock.patch.object(stats, "fetch", side_effect=public_response)
        patcher.start()
        self.addCleanup(patcher.stop)

    def test_failed_github_requests_remain_unavailable_in_page_and_history(self) -> None:
        # A denied query must not become a successful measurement of zero.
        failed = subprocess.CompletedProcess(["gh"], 1, stdout="[]", stderr="denied")
        with tempfile.TemporaryDirectory() as directory:
            history = Path(directory) / "history.jsonl"
            page = io.StringIO()
            with (
                mock.patch.object(stats.subprocess, "run", return_value=failed),
                mock.patch.object(sys, "argv", ["adoption_stats.py", "--history", str(history)]),
                contextlib.redirect_stdout(page),
            ):
                stats.main()
            recorded = json.loads(history.read_text())
        for key in ("mentions", "referrers", "releases"):
            self.assertIsNone(recorded[key], key)
        self.assertIn("| Referring sites, 14 days | unavailable |", page.getvalue())
        self.assertIn("| Release binary downloads | unavailable |", page.getvalue())
        self.assertIn("| Other repositories naming `MykytaStel/repopilot` | unavailable |", page.getvalue())

    def test_successful_empty_queries_remain_measured_zero(self) -> None:
        def empty_github(args: list[str], **kwargs) -> subprocess.CompletedProcess:
            is_list = args[1] == "search" or "referrers" in args[2] or "releases?" in args[2]
            return subprocess.CompletedProcess(args, 0, stdout="[]" if is_list else "{}", stderr="")

        with mock.patch.object(stats.subprocess, "run", side_effect=empty_github):
            collected = stats.collect()
        self.assertEqual(collected["mentions"], [])
        self.assertEqual(collected["referrers"], [])
        self.assertEqual(collected["releases"], [])
        page = stats.render(collected)
        self.assertIn("| Referring sites, 14 days | none |", page)
        self.assertIn("| Other repositories naming `MykytaStel/repopilot` | 0:", page)

    def test_missing_gh_keeps_public_counters_available(self) -> None:
        with mock.patch.object(stats.subprocess, "run", side_effect=FileNotFoundError("gh")):
            collected = stats.collect()
        self.assertEqual(collected["npm_week"], 12)
        self.assertEqual(collected["crates_total"], 100)
        self.assertFalse(collected["mcp_registry"])
        self.assertIsNone(collected["mentions"])
        self.assertIn("unavailable", stats.render(collected))

    def test_successful_queries_preserve_observed_counts(self) -> None:
        def github_response(args: list[str], **kwargs) -> subprocess.CompletedProcess:
            if args[1] == "search":
                payload = [{"repository": {"nameWithOwner": "example/project"}}]
            elif "referrers" in args[2]:
                payload = [{"referrer": "example.com", "count": 5, "uniques": 2}]
            elif "releases?" in args[2]:
                payload = [{"tag_name": "v0.23.0", "assets": [{"download_count": 7}]}]
            else:
                payload = {}
            return subprocess.CompletedProcess(args, 0, stdout=json.dumps(payload), stderr="")

        with mock.patch.object(stats.subprocess, "run", side_effect=github_response):
            collected = stats.collect()
        self.assertEqual(collected["mentions"], ["example/project"])
        self.assertEqual(collected["referrers"], [("example.com", 2)])
        self.assertEqual(collected["releases"], [("v0.23.0", 7)])
        self.assertIn("| Release binary downloads | v0.23.0 7 |", stats.render(collected))


if __name__ == "__main__":
    unittest.main()
