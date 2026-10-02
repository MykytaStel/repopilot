"""Prevent a different local compressor from looking like a changed release."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/verify-publication.sh"


class PublicationRuntimeTests(unittest.TestCase):
    def run_verifier(self, node_version, npm_version="11.19.0", tag="v0.24.0"):
        with tempfile.TemporaryDirectory() as directory:
            fixture = Path(directory)
            marker = fixture / "download-called"
            for name, body in {
                "node": f"printf '%s\\n' '{node_version}'",
                "npm": f"printf '%s\\n' '{npm_version}'",
                "gh": f"touch '{marker}'; exit 73",
            }.items():
                executable = fixture / name
                executable.write_text(f"#!/bin/sh\n{body}\n")
                executable.chmod(0o755)
            result = subprocess.run(
                ["/bin/bash", str(SCRIPT)], cwd=ROOT,
                env={**os.environ, "PATH": f"{fixture}:{os.environ['PATH']}",
                     "VERSION": tag, "SOURCE_DIR": str(ROOT)},
                text=True, capture_output=True, check=False,
            )
            return result, marker.exists()

    def test_different_runtime_fails_before_observing_public_channels(self):
        for node, npm in [("v26.8.1", "11.19.0"), ("v24.21.0", "12.0.0")]:
            with self.subTest(node=node, npm=npm):
                result, download_called = self.run_verifier(node, npm)
                self.assertEqual(result.returncode, 1)
                self.assertFalse(download_called)
                self.assertIn("unsupported verifier runtime", result.stderr)
                self.assertIn("24.21.0", result.stderr)
                self.assertNotIn("published-mismatch", result.stdout + result.stderr)

    def test_publisher_runtime_continues_to_artifact_verification(self):
        result, download_called = self.run_verifier("v24.21.0")
        self.assertTrue(download_called)
        self.assertEqual(result.returncode, 73)

    def test_historical_tag_accepts_its_publisher_runtime(self):
        result, called = self.run_verifier("v24.19.0", "11.17.0", "v0.22.0")
        self.assertTrue(called)
        self.assertEqual(result.returncode, 73)
        result, called = self.run_verifier("v24.21.0", tag="v0.22.0")
        self.assertFalse(called)
        self.assertEqual(result.returncode, 1)
        self.assertIn("24.19.0", result.stderr)

    def test_unknown_tag_does_not_assume_current_publisher_runtime(self):
        result, called = self.run_verifier("v24.21.0", tag="v9.9.9")
        self.assertFalse(called)
        self.assertEqual(result.returncode, 1)
        self.assertIn("unsupported publication tag", result.stderr)


if __name__ == "__main__":
    unittest.main()
