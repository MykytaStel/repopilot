from __future__ import annotations

import stat
import tempfile
import unittest
from pathlib import Path

import sys

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from sandbox_contract import ResourcePolicy  # noqa: E402
from sandbox_process import SubprocessDockerAdapter  # noqa: E402


class SandboxDockerTests(unittest.TestCase):
    def test_missing_pinned_image_is_unavailable_before_run(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            fake = root / "docker"
            fake.write_text(
                "#!/bin/sh\n"
                'if [ "$1" = info ]; then exit 0; fi\n'
                'if [ "$1" = image ]; then exit 1; fi\n'
                "exit 99\n",
                encoding="utf-8",
            )
            fake.chmod(fake.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
            result = SubprocessDockerAdapter(str(fake)).run(
                ("python3", "-c", "pass"),
                root,
                "python:3.12@sha256:" + "b" * 64,
                5,
                ResourcePolicy(1, "test", 1, 2.0, 4096, 300, 1200, "none", 65536),
                "test-run",
            )

        self.assertEqual(result.status, "unavailable")
        self.assertIn("image is unavailable", result.reason or "")


if __name__ == "__main__":
    unittest.main()
