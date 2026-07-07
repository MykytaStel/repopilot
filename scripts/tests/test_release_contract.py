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

    def release_note(self, version: str = "0.17.0") -> str:
        return (
            "---\n"
            "title: Explainable review output\n"
            "description: Agents can understand why findings were emitted.\n"
            "---\n\n"
            "## Highlights\n\n"
            "- First highlight.\n"
            "- Second highlight.\n"
            "- Third highlight.\n\n"
            "## Compatibility\n\n"
            "No breaking changes.\n\n"
            "## Upgrade\n\n"
            "```bash\n"
            f"cargo install repopilot --version {version} --force\n"
            f"npm install -g repopilot@{version}\n"
            "```\n"
        )

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
            release_contract.release_section("0.19.0")

    def test_release_notes_require_curated_version_file(self) -> None:
        with self.assertRaisesRegex(
            release_contract.ContractError, "Missing curated GitHub release notes"
        ):
            release_contract.release_notes("0.17.0")

    def test_release_notes_reject_empty_curated_file(self) -> None:
        self.write("docs/releases/v0.17.0.md", "\n\n")

        with self.assertRaisesRegex(
            release_contract.ContractError, "Release notes for 0.17.0 are empty"
        ):
            release_contract.release_notes("0.17.0")

    def test_release_notes_read_curated_version_file(self) -> None:
        self.write("docs/releases/v0.17.0.md", self.release_note())

        self.assertEqual(
            release_contract.release_notes("0.17.0"),
            "Agents can understand why findings were emitted.\n\n"
            "## Highlights\n\n"
            "- First highlight.\n"
            "- Second highlight.\n"
            "- Third highlight.\n\n"
            "## Compatibility\n\n"
            "No breaking changes.\n\n"
            "## Upgrade\n\n"
            "```bash\n"
            "cargo install repopilot --version 0.17.0 --force\n"
            "npm install -g repopilot@0.17.0\n"
            "```",
        )

    def test_release_title_uses_curated_front_matter(self) -> None:
        self.write("docs/releases/v0.17.0.md", self.release_note())

        self.assertEqual(
            release_contract.release_title("0.17.0"),
            "RepoPilot v0.17.0: Explainable review output",
        )

    def test_release_notes_reject_h1_body(self) -> None:
        self.write(
            "docs/releases/v0.17.0.md",
            self.release_note().replace("## Highlights", "# RepoPilot\n\n## Highlights"),
        )

        with self.assertRaisesRegex(release_contract.ContractError, "must not include an H1"):
            release_contract.release_notes("0.17.0")

    def test_release_notes_require_expected_sections(self) -> None:
        self.write(
            "docs/releases/v0.17.0.md",
            self.release_note().replace("## Compatibility", "## Details"),
        )

        with self.assertRaisesRegex(release_contract.ContractError, "must start with"):
            release_contract.release_notes("0.17.0")

    def test_release_notes_require_three_to_five_highlights(self) -> None:
        self.write(
            "docs/releases/v0.17.0.md",
            self.release_note().replace("- Third highlight.\n", ""),
        )

        with self.assertRaisesRegex(release_contract.ContractError, "3-5 highlight"):
            release_contract.release_notes("0.17.0")

    def test_release_notes_require_pinned_upgrade_commands(self) -> None:
        self.write(
            "docs/releases/v0.17.0.md",
            self.release_note().replace(
                "npm install -g repopilot@0.17.0",
                "npm update -g repopilot",
            ),
        )

        with self.assertRaisesRegex(release_contract.ContractError, "pinned upgrade"):
            release_contract.release_notes("0.17.0")

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

    def test_write_notes_uses_curated_body_and_tagged_changelog_link(self) -> None:
        self.write(
            "CHANGELOG.md",
            "## [0.17.0] - 2026-07-01\n\n### Fixed\n\n- Precise review.\n",
        )
        self.write(
            "docs/releases/v0.17.0.md",
            self.release_note(),
        )
        output = release_contract.ROOT / "dist/notes.md"

        release_contract.write_notes("v0.17.0", output)

        notes = output.read_text(encoding="utf-8")
        self.assertIn("Agents can understand why findings were emitted.", notes)
        self.assertNotIn("Precise review.", notes)
        self.assertNotIn("title: Explainable review output", notes)
        self.assertNotIn("# RepoPilot", notes)
        self.assertIn("Full technical changelog:", notes)
        self.assertIn("/blob/v0.17.0/CHANGELOG.md", notes)
        self.assertNotIn("0.16.0", notes)

    def test_removed_vscode_surface_fails_when_directory_returns(self) -> None:
        self.write("editors/vscode/package.json", json.dumps({"name": "preview"}))

        with self.assertRaisesRegex(
            release_contract.ContractError, "VS Code surface"
        ):
            release_contract.check_removed_vscode_surface()

    def test_roadmap_docs_require_v020_roadmap_file(self) -> None:
        self.write("docs/engineering/v0.20-release-scorecard.md", "# Scorecard\n")
        self.write("docs/README.md", "- [r](roadmap/v0.20.md)\n- [s](engineering/v0.20-release-scorecard.md)\n")

        with self.assertRaisesRegex(
            release_contract.ContractError, "Missing release documentation"
        ):
            release_contract.check_roadmap_docs()

    def test_roadmap_docs_require_v020_scorecard_file(self) -> None:
        self.write(
            "docs/roadmap/v0.20.md",
            "# v0.20\n## Problem Statement\n## Product Promise\n"
            "## Non-Goals\n## Compatibility Contract\n## Planned PR Sequence\n"
            "## Release Gates\n## Definition of Done\n",
        )
        self.write("docs/README.md", "- [r](roadmap/v0.20.md)\n- [s](engineering/v0.20-release-scorecard.md)\n")

        with self.assertRaisesRegex(
            release_contract.ContractError, "Missing release documentation"
        ):
            release_contract.check_roadmap_docs()

    def test_roadmap_docs_require_headings(self) -> None:
        self.write("docs/roadmap/v0.20.md", "# v0.20\n## Problem Statement\n")
        self.write("docs/engineering/v0.20-release-scorecard.md", "# Scorecard\n")
        self.write("docs/README.md", "- [r](roadmap/v0.20.md)\n- [s](engineering/v0.20-release-scorecard.md)\n")

        with self.assertRaisesRegex(
            release_contract.ContractError, "missing required headings"
        ):
            release_contract.check_roadmap_docs()

    def test_roadmap_docs_require_index_links(self) -> None:
        self.write(
            "docs/roadmap/v0.20.md",
            "# v0.20\n## Problem Statement\n## Product Promise\n"
            "## Non-Goals\n## Compatibility Contract\n## Planned PR Sequence\n"
            "## Release Gates\n## Definition of Done\n",
        )
        self.write("docs/engineering/v0.20-release-scorecard.md", "# Scorecard\n")
        self.write("docs/README.md", "- no links here\n")

        with self.assertRaisesRegex(
            release_contract.ContractError, "does not link"
        ):
            release_contract.check_roadmap_docs()

    def test_roadmap_docs_pass_when_complete(self) -> None:
        self.write(
            "docs/roadmap/v0.20.md",
            "# v0.20\n## Problem Statement\n## Product Promise\n"
            "## Non-Goals\n## Compatibility Contract\n## Planned PR Sequence\n"
            "## Release Gates\n## Definition of Done\n",
        )
        self.write("docs/engineering/v0.20-release-scorecard.md", "# Scorecard\n")
        self.write("docs/README.md", "- [r](roadmap/v0.20.md)\n- [s](engineering/v0.20-release-scorecard.md)\n")

        release_contract.check_roadmap_docs()

    def _write_zoo_scorecard_fixture(self) -> None:
        self.write("tests/zoo/manifest.toml", '[[repo]]\nname = "repo-a"\n')
        self.write("docs/rules-reference.md", "### `a.rule` — Title\n\n- **Lifecycle:** stable\n")
        self.write(
            "tests/zoo/expectations/repo-a.toml",
            "schema_version = 1\n"
            'repo = "repo-a"\n\n'
            "[[finding]]\n"
            'profile = "default"\n'
            'finding_id = "a.rule:x.py:deadbeef"\n'
            'rule_id = "a.rule"\n'
            'path = "x.py"\n'
            'disposition = "actionable"\n'
            'reason = "fixture"\n',
        )

    def _fresh_rule_scorecard(self) -> str:
        return release_contract.zs.generate_scorecard(
            [{"name": "repo-a"}],
            release_contract.ROOT / "tests/zoo/expectations",
            release_contract.ROOT / "docs/rules-reference.md",
        )

    def test_rule_scorecard_requires_committed_files(self) -> None:
        with self.assertRaisesRegex(release_contract.ContractError, "Missing"):
            release_contract.check_rule_scorecard()

    def test_rule_scorecard_fails_when_stale(self) -> None:
        self._write_zoo_scorecard_fixture()
        self.write("docs/engineering/rule-scorecard.md", "stale content\n")
        self.write("docs/README.md", "- [x](engineering/rule-scorecard.md)\n")

        with self.assertRaisesRegex(release_contract.ContractError, "stale"):
            release_contract.check_rule_scorecard()

    def test_rule_scorecard_requires_docs_index_link(self) -> None:
        self._write_zoo_scorecard_fixture()
        self.write("docs/engineering/rule-scorecard.md", self._fresh_rule_scorecard())
        self.write("docs/README.md", "- no links here\n")

        with self.assertRaisesRegex(release_contract.ContractError, "does not link"):
            release_contract.check_rule_scorecard()

    def test_rule_scorecard_passes_when_fresh(self) -> None:
        self._write_zoo_scorecard_fixture()
        self.write("docs/engineering/rule-scorecard.md", self._fresh_rule_scorecard())
        self.write("docs/README.md", "- [x](engineering/rule-scorecard.md)\n")

        release_contract.check_rule_scorecard()

    def test_zoo_gate_skips_when_not_cloned(self) -> None:
        self.write("tests/zoo/manifest.toml", '[[repo]]\nname = "repo-a"\n')

        release_contract.check_zoo_gate()

    def test_zoo_gate_fails_on_nonzero_scan_exit(self) -> None:
        self.write("tests/zoo/manifest.toml", '[[repo]]\nname = "repo-a"\n')
        (release_contract.ROOT / ".zoo" / "repo-a").mkdir(parents=True)
        result = subprocess.CompletedProcess(
            args=["zoo.py", "scan"], returncode=1, stdout="oops\n", stderr=""
        )

        with mock.patch.object(release_contract.subprocess, "run", return_value=result) as run_mock:
            with self.assertRaisesRegex(release_contract.ContractError, "zoo scan gate failed"):
                release_contract.check_zoo_gate()
        run_mock.assert_called_once()

    def test_zoo_gate_passes_when_scan_succeeds(self) -> None:
        self.write("tests/zoo/manifest.toml", '[[repo]]\nname = "repo-a"\n')
        (release_contract.ROOT / ".zoo" / "repo-a").mkdir(parents=True)
        result = subprocess.CompletedProcess(
            args=["zoo.py", "scan"], returncode=0, stdout="ok\n", stderr=""
        )

        with mock.patch.object(release_contract.subprocess, "run", return_value=result):
            release_contract.check_zoo_gate()

    def test_zoo_gate_requires_manifest_file(self) -> None:
        with self.assertRaisesRegex(release_contract.ContractError, "Missing zoo manifest"):
            release_contract.check_zoo_gate()


if __name__ == "__main__":
    unittest.main()
