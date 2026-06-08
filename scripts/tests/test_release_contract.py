from __future__ import annotations

import importlib.util
import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock


SCRIPT = Path(__file__).resolve().parents[1] / "release-contract.py"
SPEC = importlib.util.spec_from_file_location("release_contract", SCRIPT)
assert SPEC and SPEC.loader
release_contract = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(release_contract)


class ReleaseContractTests(unittest.TestCase):
    def setUp(self) -> None:
        self.original_root = release_contract.ROOT
        self.original_changelog = release_contract.CHANGELOG
        self.temp = tempfile.TemporaryDirectory()
        release_contract.ROOT = Path(self.temp.name)
        release_contract.CHANGELOG = release_contract.ROOT / "CHANGELOG.md"

    def tearDown(self) -> None:
        release_contract.ROOT = self.original_root
        release_contract.CHANGELOG = self.original_changelog
        self.temp.cleanup()

    def write(self, relative: str, content: str) -> Path:
        path = release_contract.ROOT / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")
        return path

    def test_release_section_extracts_only_requested_version(self) -> None:
        self.write(
            "CHANGELOG.md",
            "# Changelog\n\n"
            "## [0.17.0] - 2026-07-01\n\n### Fixed\n\n- Current.\n\n"
            "## [0.16.0] - 2026-06-08\n\n### Fixed\n\n- Previous.\n",
        )

        self.assertEqual(
            release_contract.release_section("0.17.0"),
            "### Fixed\n\n- Current.",
        )
        with self.assertRaises(release_contract.ContractError):
            release_contract.release_section("0.18.0")

    def test_version_from_tag_rejects_non_release_refs(self) -> None:
        self.assertEqual(release_contract.version_from_tag("v0.17.0"), "0.17.0")
        for invalid in ("0.17", "release/v0.17.0", "v0.17.0-rc.1"):
            with self.subTest(invalid=invalid):
                with self.assertRaises(release_contract.ContractError):
                    release_contract.version_from_tag(invalid)

    def test_action_pin_check_rejects_mutable_refs(self) -> None:
        self.write("action.yml", "runs:\n  using: composite\n  steps: []\n")
        self.write(
            ".github/workflows/ci.yml",
            "jobs:\n  test:\n    steps:\n      - uses: actions/checkout@v6\n",
        )

        with self.assertRaisesRegex(
            release_contract.ContractError, "commit SHAs"
        ):
            release_contract.check_action_pins()

    def test_release_orchestration_requires_reusable_npm_workflow(self) -> None:
        self.write(".github/workflows/release.yml", "jobs: {}\n")
        self.write(
            ".github/workflows/publish-npm.yml",
            "on:\n  workflow_dispatch:\n",
        )

        with self.assertRaisesRegex(release_contract.ContractError, "does not call"):
            release_contract.check_release_orchestration()

    def test_cargo_package_rejects_files_outside_allowlist(self) -> None:
        result = subprocess.CompletedProcess(
            args=["cargo", "package"],
            returncode=0,
            stdout="Cargo.toml\nREADME.md\ndocs/internal.md\nsrc/lib.rs\n",
            stderr="",
        )
        with mock.patch.object(release_contract.subprocess, "run", return_value=result):
            with self.assertRaisesRegex(
                release_contract.ContractError, "outside the allowlist"
            ):
                release_contract.check_cargo_package()

    def test_write_notes_uses_changelog_body_and_tagged_link(self) -> None:
        self.write(
            "CHANGELOG.md",
            "## [0.17.0] - 2026-07-01\n\n### Fixed\n\n- Precise review.\n",
        )
        output = release_contract.ROOT / "dist/notes.md"

        release_contract.write_notes("v0.17.0", output)

        notes = output.read_text(encoding="utf-8")
        self.assertIn("### Fixed", notes)
        self.assertIn("/blob/v0.17.0/CHANGELOG.md", notes)
        self.assertNotIn("0.16.0", notes)

    def test_removed_vscode_surface_fails_when_directory_returns(self) -> None:
        self.write("editors/vscode/package.json", json.dumps({"name": "preview"}))

        with self.assertRaisesRegex(
            release_contract.ContractError, "VS Code surface"
        ):
            release_contract.check_removed_vscode_surface()


if __name__ == "__main__":
    unittest.main()
