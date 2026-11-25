import argparse
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import zoo


class ZooReleaseInputsTests(unittest.TestCase):
    def test_missing_clone_is_not_a_successful_gate(self):
        with tempfile.TemporaryDirectory() as tmp, patch.object(zoo, "ZOO_DIR", Path(tmp)):
            errors = zoo.release_input_errors([{"name": "example", "sha": "a"}])
            self.assertEqual(errors, ["example: missing clone or incorrect pin"])

    def test_pin_and_clean_checkout_do_not_replace_required_labels(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "example").mkdir()
            with (
                patch.object(zoo, "ZOO_DIR", root),
                patch.object(zoo, "SNAPSHOT_DIR", root),
                patch.object(zoo, "EXPECTATION_DIR", root),
                patch.object(zoo, "head_sha", return_value="a"),
                patch.object(zoo, "run", return_value=subprocess.CompletedProcess([], 0, "", "")),
            ):
                errors = zoo.release_input_errors([{"name": "example", "sha": "a"}])
                self.assertEqual(len(errors), 2)
                (root / "example.json").write_text("{}")
                (root / "example.toml").write_text("")
                self.assertEqual(zoo.release_input_errors([{"name": "example", "sha": "a"}]), [])
                with patch.object(zoo, "run", return_value=subprocess.CompletedProcess([], 0, " M file\n", "")):
                    self.assertEqual(zoo.release_input_errors([{"name": "example", "sha": "a"}]),
                                     ["example: checkout has local changes"])

    def test_release_gate_cannot_bless_or_use_a_subset(self):
        for options in ({"bless": True}, {"only": "example"}, {"allow_version_mismatch": True}):
            args = argparse.Namespace(**({"only": None, "bless": False,
                                         "allow_version_mismatch": False, "release_gate": True} | options))
            with patch.object(zoo, "load_manifest", return_value=[{"name": "example"}]):
                self.assertEqual(zoo.cmd_scan(args), 2)


if __name__ == "__main__":
    unittest.main()
