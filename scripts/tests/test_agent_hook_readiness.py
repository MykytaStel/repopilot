from __future__ import annotations

import json
import os
import shutil
import stat
import sys
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
HOOKS = {
    "claude-start": "integrations/claude-code/repopilot/scripts/snapshot.sh",
    "claude-stop": "integrations/claude-code/repopilot/scripts/guard.sh",
    "cursor-start": "integrations/cursor/hooks/repopilot-snapshot.sh",
    "cursor-stop": "integrations/cursor/hooks/repopilot-guard.sh",
}
FAKE_CLI = """#!/bin/sh
case "$1" in
  --version) echo "${TEST_CLI_VERSION:-repopilot 0.24.0}" ;;
  snapshot)
    [ "${TEST_SNAPSHOT_FAILURE:-0}" = 1 ] && exit 7
    mkdir -p .repopilot
    echo ready > .repopilot/snapshot.json ;;
  review)
    [ "${TEST_REVIEW_FAILURE:-0}" = 1 ] && exit 7
    echo 'RepoPilot Review' ;;
  *) exit 8 ;;
esac
"""


@unittest.skipUnless(shutil.which("sh"), "POSIX shell hook recipe")
class AgentHookReadinessTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.repo = Path(self.temp.name)
        self.bin = self.repo / "bin"
        self.bin.mkdir()
        for name in ("cat", "awk", "tr", "sed", "mkdir", "rm"):
            executable = shutil.which(name)
            if executable is None:
                self.skipTest(f"{name} is required by POSIX hooks")
            (self.bin / name).symlink_to(executable)
        self.write_executable("git", "#!/bin/sh\ncase \"$*\" in\n"
                              "  'rev-parse --is-inside-work-tree') echo true ;;\n"
                              "  'rev-parse --show-toplevel') pwd ;;\n"
                              "  *) exit 1 ;;\nesac\n")
        (self.repo / ".repopilot").mkdir()
        self.snapshot = self.repo / ".repopilot/snapshot.json"
        self.snapshot.write_text("prior baseline")

    def write_executable(self, name: str, text: str) -> None:
        path = self.bin / name
        path.write_text(text)
        path.chmod(0o755)

    def run_hook(self, name: str, *, shell: str = "sh",
                 **variables: str) -> subprocess.CompletedProcess:
        env = dict(os.environ, PATH=str(self.bin), **variables)
        payload = {"source": "startup", "stop_hook_active": False,
                   "status": "completed", "loop_count": 0}
        result = subprocess.run([shutil.which(shell), str(ROOT / HOOKS[name])],
                                cwd=self.repo, env=env, input=json.dumps(payload),
                                text=True, capture_output=True, check=True)
        if name.startswith("cursor"):
            self.assertEqual(json.loads(result.stdout), {})
        else:
            self.assertEqual(result.stdout, "")
        return result

    def test_missing_cli_is_visible_without_blocking_the_agent(self) -> None:
        for name in HOOKS:
            with self.subTest(hook=name):
                self.snapshot.write_text("prior baseline")
                result = self.run_hook(name)
                self.assertIn("unavailable", result.stderr)
                self.assertIn("repopilot", result.stderr)
                if name.endswith("start"):
                    self.assertFalse(self.snapshot.exists())

    def test_old_cli_does_not_create_or_review_a_session_baseline(self) -> None:
        self.write_executable("repopilot", FAKE_CLI)
        for name in HOOKS:
            with self.subTest(hook=name):
                self.snapshot.write_text("prior baseline")
                result = self.run_hook(name, TEST_CLI_VERSION="repopilot 0.22.0")
                self.assertIn("0.24", result.stderr)
                self.assertIn("unavailable", result.stderr)
                if name.endswith("start"):
                    self.assertFalse(self.snapshot.exists())
                else:
                    self.assertEqual(self.snapshot.read_text(), "prior baseline")

    def test_cli_repaired_mid_session_cannot_review_an_old_baseline(self) -> None:
        for start, stop in (("claude-start", "claude-stop"),
                            ("cursor-start", "cursor-stop")):
            for failure in ("missing", "old"):
                with self.subTest(hook=start, failure=failure):
                    (self.bin / "repopilot").unlink(missing_ok=True)
                    self.snapshot.write_text("prior baseline")
                    if failure == "old":
                        self.write_executable("repopilot", FAKE_CLI)
                    self.run_hook(start, TEST_CLI_VERSION="repopilot 0.22.0")
                    self.write_executable("repopilot", FAKE_CLI)
                    stopped = self.run_hook(stop)
                    self.assertIn("snapshot is missing", stopped.stderr)

    def test_failed_snapshot_invalidates_the_prior_session_baseline(self) -> None:
        self.write_executable("repopilot", FAKE_CLI)
        for name in ("claude-start", "cursor-start"):
            with self.subTest(hook=name):
                self.snapshot.write_text("prior baseline")
                result = self.run_hook(name, TEST_SNAPSHOT_FAILURE="1")
                self.assertIn("snapshot", result.stderr)
                self.assertIn("unavailable", result.stderr)
                self.assertFalse(self.snapshot.exists())

    @unittest.skipIf(hasattr(os, "geteuid") and os.geteuid() == 0,
                     "Root bypasses the permission failure under test")
    def test_unwritable_state_cannot_silently_reuse_a_prior_baseline(self) -> None:
        self.write_executable("repopilot", FAKE_CLI)
        state = self.snapshot.parent
        for start, stop in (("claude-start", "claude-stop"),
                            ("cursor-start", "cursor-stop")):
            with self.subTest(hook=start):
                self.snapshot.write_text("prior baseline")
                self.snapshot.chmod(0o444)
                state.chmod(0o555)
                try:
                    started = self.run_hook(start, TEST_SNAPSHOT_FAILURE="1")
                    self.assertIn("write permission", started.stderr)
                    stopped = self.run_hook(stop)
                    self.assertIn("write permission", stopped.stderr)
                    self.assertIn("unavailable", stopped.stderr)
                finally:
                    state.chmod(0o755)
                    self.snapshot.chmod(0o644)

    def test_failed_review_and_missing_snapshot_are_visible(self) -> None:
        self.write_executable("repopilot", FAKE_CLI)
        for name in ("claude-stop", "cursor-stop"):
            with self.subTest(hook=name):
                self.snapshot.write_text("prior baseline")
                failed = self.run_hook(name, TEST_REVIEW_FAILURE="1")
                self.assertIn("review", failed.stderr)
                self.assertIn("unavailable", failed.stderr)
                self.snapshot.unlink()
                missing = self.run_hook(name)
                self.assertIn("snapshot", missing.stderr)
                self.assertIn("unavailable", missing.stderr)

    @unittest.skipUnless(hasattr(os, "chflags") and hasattr(stat, "UF_IMMUTABLE"),
                         "Immutable metadata regression requires BSD file flags")
    def test_immutable_metadata_cannot_be_reused_after_failed_start(self) -> None:
        self.write_executable("repopilot", FAKE_CLI)
        for start, stop in (("claude-start", "claude-stop"),
                            ("cursor-start", "cursor-stop")):
            with self.subTest(hook=start):
                self.snapshot.write_text("prior baseline")
                os.chflags(self.snapshot, stat.UF_IMMUTABLE)
                try:
                    shells = ("sh", "dash") if shutil.which("dash") else ("sh",)
                    for shell in shells:
                        with self.subTest(shell=shell):
                            started = self.run_hook(start, shell=shell)
                            self.assertIn("could not be invalidated", started.stderr)
                            stopped = self.run_hook(stop, shell=shell)
                            self.assertIn("write permission", stopped.stderr)
                            self.assertIn("unavailable", stopped.stderr)
                finally:
                    os.chflags(self.snapshot, 0)

    @unittest.skipUnless(sys.platform == "darwin", "macOS deletion ACL regression")
    def test_delete_denied_acl_invalidates_writable_metadata(self) -> None:
        self.write_executable("repopilot", FAKE_CLI)
        for start in ("claude-start", "cursor-start"):
            with self.subTest(hook=start):
                self.snapshot.write_text("prior baseline")
                subprocess.run([shutil.which("chmod"), "+a", "everyone deny delete",
                                str(self.snapshot)], check=True)
                try:
                    result = self.run_hook(start, TEST_SNAPSHOT_FAILURE="1")
                    self.assertIn("unavailable", result.stderr)
                    self.assertNotEqual(self.snapshot.read_text(), "prior baseline")
                finally:
                    subprocess.run([shutil.which("chmod"), "-N", str(self.snapshot)],
                                   check=True)

    def test_current_prerelease_and_new_major_versions_keep_healthy_hooks_quiet(self) -> None:
        self.write_executable("repopilot", FAKE_CLI)
        for version in ("0.24.0", "0.24.0-rc.1", "0.25.0", "1.0.0"):
            for name in HOOKS:
                with self.subTest(version=version, hook=name):
                    result = self.run_hook(name, TEST_CLI_VERSION=f"repopilot {version}")
                    self.assertEqual(result.stderr, "")
                    self.assertTrue(self.snapshot.exists())


if __name__ == "__main__":
    unittest.main()
