from __future__ import annotations

import sys
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from differential_resource import (  # noqa: E402
    execute_timed_command,
    parse_peak_rss_kb,
    resource_command,
    resource_sample_from_file,
)


class DifferentialResourceTests(unittest.TestCase):
    def test_parses_linux_peak_rss_in_kib(self) -> None:
        self.assertEqual(
            parse_peak_rss_kb("Maximum resident set size (kbytes): 12345\n", "linux"),
            12345,
        )

    def test_parses_macos_peak_rss_bytes_as_kib(self) -> None:
        self.assertEqual(
            parse_peak_rss_kb("  12582912  maximum resident set size\n", "darwin"),
            12288,
        )

    def test_unsupported_or_malformed_time_output_is_unavailable(self) -> None:
        self.assertIsNone(parse_peak_rss_kb("time failed\n", "linux"))
        self.assertEqual(resource_sample_from_file(Path("/does/not/exist"))["status"], "unavailable")

    def test_resource_command_keeps_the_original_command_as_the_suffix(self) -> None:
        command = (sys.executable, "-c", "print('ok')")
        wrapped = resource_command(command, Path("/tmp/repopilot-time.txt"), "linux")
        if wrapped is None:
            self.skipTest("portable POSIX time sampler is unavailable")
        self.assertEqual(tuple(wrapped[-len(command) :]), command)

    def test_timeout_keeps_resource_measurement_unavailable(self) -> None:
        result = execute_timed_command(
            (sys.executable, "-c", "import time; time.sleep(0.2)"),
            Path("."),
            0.01,
        )
        self.assertEqual(result["status"], "timeout")
        self.assertEqual(result["resource"]["status"], "unavailable")
        self.assertIn("timed out", result["resource"]["reason"])


if __name__ == "__main__":
    unittest.main()
