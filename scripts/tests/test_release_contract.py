from __future__ import annotations

import importlib.util
import json
import re
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

    def write_product_workflow_docs(self) -> None:
        self.write(
            "README.md",
            "RepoPilot supports developers and teams.\n"
            "## Agent and CI integrations\n"
            "The same review can check work from a human or a coding agent.\n",
        )
        self.write(
            "docs/README.md",
            "[Common workflows](commands.md)\n"
            "[First-run configuration and verification](configuration.md)\n",
        )
        self.write(
            "docs/commands.md",
            "Decision, Change Proof, and `merge_readiness` are different records. "
            "`VERIFIED` → `PASS`, `REVIEW` → `REVIEW`, `BROKEN` → `BLOCK`, "
            "and `NOT ASSESSED` → `NOT ASSESSED`. `merge_readiness` can be "
            "`ready` while Change Proof is `REVIEW` when no sufficient proof policy "
            "is selected. A failed configured gate also appears as a proof reason. "
            "The `--fail-on-priority` CI threshold is shown separately. Snapshot "
            "stores `HEAD` and `dirty`; a dirty snapshot records a baseline that "
            "keeps pre-existing changes out. It cannot establish who authored each change. "
            "configuration.md#first-review-choose-and-run-a-check\n",
        )
        self.write(
            "docs/security.md",
            "An explicitly selected `--verify` or `repopilot_review_change` "
            "`verify` check runs on the host with filesystem and network access. "
            "RepoPilot does not sandbox that process.\n",
        )
        self.write(
            "docs/roadmap/v0.24.md",
            "## Phase E — Agent Integration Workflow\n\n"
            "This optional integration extends the developer review workflow to\n"
            "coding agents.\n",
        )

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
        for invalid in ("0.17", "release/v0.17.0", "v0.17.0-beta.1"):
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

    def test_publication_recovery_requires_exact_identity_checks(self) -> None:
        self.write(".github/workflows/release.yml", "jobs: {}\n")
        self.write(".github/workflows/publish-npm.yml", "jobs: {}\n")

        with self.assertRaisesRegex(
            release_contract.ContractError, "publication recovery"
        ):
            release_contract.check_publication_recovery_contract()

    def test_release_verifier_normalizes_crlf_checksum_files(self) -> None:
        workflow = (self.original_root / "scripts/verify-publication.sh").read_text(
            encoding="utf-8"
        )

        self.assertIn(
            'tr -d \'\\r\' < "$archive.sha256" | sha256sum -c -',
            workflow,
        )

    def test_release_workflow_smokes_packaged_archives(self) -> None:
        workflow = (self.original_root / ".github/workflows/release.yml").read_text(
            encoding="utf-8"
        )

        self.assertIn('tar -xzf "$ASSET" -C packaged-smoke', workflow)
        self.assertIn('./packaged-smoke/repopilot --version', workflow)
        self.assertIn('Expand-Archive -Path $env:ASSET -DestinationPath $SmokeDir', workflow)
        self.assertIn('& ".\\$SmokeDir\\repopilot.exe" --version', workflow)

    def test_publication_recovery_rejects_mutable_latest_queries(self) -> None:
        self.write(
            ".github/workflows/release.yml",
            """
            VERSION_NUMBER="${VERSION#v}"
            python3 scripts/publication_state.py classify
            npm view "${package}@${VERSION_NUMBER}" version dist.integrity --json
            npm view repopilot version
            """,
        )
        self.write(
            ".github/workflows/publish-npm.yml",
            """
            npm view "${package_name}@${VERSION_NUMBER}" version dist.integrity --json
            published-mismatch
            """,
        )

        with self.assertRaisesRegex(
            release_contract.ContractError, "mutable npm query"
        ):
            release_contract.check_publication_recovery_contract()

    def test_publication_recovery_rejects_nested_integrity_lookup(self) -> None:
        self.write(
            ".github/workflows/release.yml",
            'jq -r \'if type == "string" then "" else (.dist.integrity // "") end\'\n',
        )
        self.write(".github/workflows/publish-npm.yml", "")

        with self.assertRaisesRegex(
            release_contract.ContractError, "literal dist.integrity"
        ):
            release_contract.check_publication_recovery_contract()

    def test_publication_recovery_requires_crates_user_agent(self) -> None:
        self.write(
            ".github/workflows/release.yml",
            'status="$(curl -sS -o out -w \'%{http_code}\' "https://crates.io/api/v1/crates/repopilot/1.0.0")"\n',
        )
        self.write(".github/workflows/publish-npm.yml", "")

        with self.assertRaisesRegex(release_contract.ContractError, "User-Agent"):
            release_contract.check_publication_recovery_contract()

    def test_release_workflow_sends_crates_user_agent(self) -> None:
        workflow = "\n".join(
            path.read_text(encoding="utf-8")
            for path in [
                *(self.original_root / ".github/workflows").glob("*.yml"),
                *(self.original_root / "scripts").glob("*.sh"),
            ]
        )
        crates_calls = [line for line in workflow.splitlines() if "crates.io/api" in line]

        self.assertTrue(crates_calls)
        for line in crates_calls:
            if "curl" in line:
                self.assertIn('-A "$CRATES_USER_AGENT"', line)

    def test_publication_verifier_reads_indented_homebrew_version(self) -> None:
        verifier = (self.original_root / "scripts/verify-publication.sh").read_text(
            encoding="utf-8"
        )
        formula = 'class Repopilot < Formula\n  version "1.2.3"\nend\n'
        pattern = re.search(r"sed -nE '([^']+)'", verifier).group(1)
        result = subprocess.run(
            ["sed", "-nE", pattern],
            input=formula,
            capture_output=True,
            text=True,
            check=True,
        )
        self.assertEqual(result.stdout.strip(), "1.2.3")

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

    def test_release_candidate_tags_are_prereleases(self) -> None:
        self.assertEqual(release_contract.version_from_tag("v0.24.0-rc.1"), "0.24.0-rc.1")
        self.assertTrue(release_contract.is_prerelease("0.24.0-rc.1"))
        self.assertFalse(release_contract.is_prerelease("0.24.0"))
        for invalid in ("v0.24.0-beta", "v0.24.0-rc", "v0.24.0-rc.1.2", "0.24"):
            with self.assertRaises(release_contract.ContractError):
                release_contract.version_from_tag(invalid)

    def test_release_candidate_uses_unreleased_changelog_and_synthesized_notes(self) -> None:
        self.write(
            "CHANGELOG.md",
            "## [Unreleased]\n\n### Changed\n\n- Calibrated priority.\n\n"
            "## [0.23.0] - 2026-09-24\n\n### Fixed\n\n- Shipped.\n",
        )
        output = release_contract.ROOT / "dist/notes.md"

        self.assertEqual(
            release_contract.release_section("0.24.0-rc.1"),
            "### Changed\n\n- Calibrated priority.",
        )
        release_contract.write_notes("v0.24.0-rc.1", output)

        notes = output.read_text(encoding="utf-8")
        self.assertIn("Pre-release rehearsal of RepoPilot 0.24.0", notes)
        self.assertIn("npm install -g repopilot@0.24.0-rc.1", notes)
        self.assertIn("/blob/v0.24.0-rc.1/CHANGELOG.md", notes)
        self.assertEqual(
            release_contract.release_title("0.24.0-rc.1"),
            "RepoPilot v0.24.0-rc.1: Release candidate for 0.24.0",
        )

    def test_release_candidate_requires_unreleased_entries(self) -> None:
        self.write(
            "CHANGELOG.md",
            "## [Unreleased]\n\n## [0.23.0] - 2026-09-24\n\n### Fixed\n\n- Shipped.\n",
        )

        with self.assertRaisesRegex(release_contract.ContractError, "Unreleased"):
            release_contract.release_section("0.24.0-rc.1")

    def test_prerelease_channels_pass_for_current_workflows(self) -> None:
        release_contract.ROOT = self.original_root
        release_contract.check_prerelease_channels()

    def test_prerelease_channels_reject_npm_publish_without_dist_tag(self) -> None:
        self.write(
            ".github/workflows/release.yml",
            "prerelease: ${{ contains(github.ref_name, '-') }}\n"
            "make_latest: ${{ !contains(github.ref_name, '-') }}\n"
            "if: ${{ !contains(github.ref_name, '-') }}\n",
        )
        self.write(
            ".github/workflows/publish-npm.yml",
            'NPM_DIST_TAG="next"\nnpm publish "$package_dir" --access public\nnpm publish --access public\n',
        )
        self.write("scripts/verify-publication.sh", "npm view repopilot dist-tags.latest\n")

        with self.assertRaisesRegex(
            release_contract.ContractError, "prerelease channel contract"
        ):
            release_contract.check_prerelease_channels()

    def test_removed_vscode_surface_fails_when_directory_returns(self) -> None:
        self.write("editors/vscode/package.json", json.dumps({"name": "preview"}))

        with self.assertRaisesRegex(
            release_contract.ContractError, "VS Code surface"
        ):
            release_contract.check_removed_vscode_surface()

    def test_roadmap_docs_require_archived_release_note(self) -> None:
        self.write("docs/roadmap/v0.20.md", "# RepoPilot 0.20\nStatus: released.\n")
        self.write("docs/engineering/v0.20-release-scorecard.md", "# Scorecard\n")
        self.write(
            "docs/engineering/README.md",
            "- [v0.20 scorecard](v0.20-release-scorecard.md)\n",
        )

        with self.assertRaisesRegex(
            release_contract.ContractError, "Missing release documentation"
        ):
            release_contract.check_roadmap_docs()

    def test_roadmap_docs_reject_detailed_internal_release_plan(self) -> None:
        self.write("docs/releases/v0.20.0.md", "# v0.20.0\n## Highlights\n")
        self.write(
            "docs/engineering/v0.20-release-scorecard.md",
            "# v0.20 release record\n\n"
            "Recorded peak memory: 41,157,112 bytes.\n"
            "The remaining checklist is not a complete release assessment.\n"
            "[Release notes](../releases/v0.20.0.md)\n",
        )
        self.write(
            "docs/engineering/README.md",
            "- [v0.20 release record](v0.20-release-scorecard.md)\n",
        )
        for version in ("0.20", "0.21", "0.22", "0.23"):
            self.write(
                f"docs/roadmap/v{version}.md",
                f"# RepoPilot {version}\nStatus: released.\n"
                f"[Release notes](../releases/v{version}.0.md)\n",
            )
            self.write(f"docs/releases/v{version}.0.md", f"# v{version}.0\n")
        self.write(
            "docs/roadmap/v0.20.md",
            "# RepoPilot 0.20\nStatus: released.\n"
            "[Release notes](../releases/v0.20.0.md)\n"
            "## Planned PR Sequence\n",
        )

        with self.assertRaisesRegex(
            release_contract.ContractError, "internal planning material"
        ):
            release_contract.check_roadmap_docs()

    def test_roadmap_docs_pass_when_public_history_is_concise(self) -> None:
        self.write(
            "docs/engineering/v0.20-release-scorecard.md",
            "# v0.20 release record\n\n"
            "Status: historical summary.\n"
            "Recorded peak memory: 41,157,112 bytes.\n"
            "The remaining checklist is not a complete release assessment.\n"
            "See [release notes](../releases/v0.20.0.md).\n",
        )
        self.write(
            "docs/engineering/README.md",
            "- [v0.20 release record](v0.20-release-scorecard.md)\n",
        )
        for version in ("0.20", "0.21", "0.22", "0.23"):
            self.write(
                f"docs/roadmap/v{version}.md",
                f"# RepoPilot {version}\nStatus: released.\n"
                f"See [release notes](../releases/v{version}.0.md).\n",
            )
            self.write(
                f"docs/releases/v{version}.0.md",
                f"# RepoPilot {version}.0\n\n## Highlights\n",
            )

        release_contract.check_roadmap_docs()

    def test_roadmap_docs_reject_unverified_v020_checklist(self) -> None:
        self.write(
            "docs/engineering/v0.20-release-scorecard.md",
            "# v0.20 Release Scorecard\n\n"
            "Use this checklist during every 0.20 PR and at release time.\n"
            "- [ ] Install workflow verified\n",
        )
        self.write(
            "docs/engineering/README.md",
            "- [v0.20 scorecard](v0.20-release-scorecard.md)\n",
        )
        for version in ("0.20", "0.21", "0.22", "0.23"):
            self.write(
                f"docs/roadmap/v{version}.md",
                f"# RepoPilot {version}\nStatus: released.\n"
                f"[Release notes](../releases/v{version}.0.md)\n",
            )
            self.write(
                f"docs/releases/v{version}.0.md",
                f"# RepoPilot {version}.0\n## Highlights\n",
            )

        with self.assertRaisesRegex(
            release_contract.ContractError, "historical release record"
        ):
            release_contract.check_roadmap_docs()

    def test_v023_docs_require_release_summary_and_report_limitations(self) -> None:
        self.write(
            "docs/releases/v0.23.0.md",
            "# RepoPilot 0.23.0\n\n## Highlights\n\nChange Proof.\n\n"
            "## Compatibility\n\nAdditive.\n\n## Upgrade\n",
        )
        self.write(
            "docs/reports.md",
            "`assessment_status` is not a safety verdict. Unsupported scope and "
            "excluded files remain disclosed.\n",
        )

        release_contract.check_v023_docs()
        self.write("docs/reports.md", "`assessment_status` is a safety verdict.\n")
        with self.assertRaisesRegex(
            release_contract.ContractError, "documentation contract"
        ):
            release_contract.check_v023_docs()

    def test_v023_docs_require_release_note(self) -> None:
        self.write(
            "docs/reports.md",
            "`assessment_status` is not a safety verdict. Unsupported scope and "
            "excluded files remain disclosed.\n",
        )
        with self.assertRaisesRegex(
            release_contract.ContractError, "Missing v0.23 documentation"
        ):
            release_contract.check_v023_docs()

    def test_docs_navigation_requires_architecture_and_release_notes(self) -> None:
        self.write("docs/architecture.md", "# Current architecture\n")
        self.write("docs/releases/v0.24.3.md", "# v0.24.3\n")
        self.write("docs/engineering/foo.md", "# Foo\n")
        self.write("docs/engineering/README.md", "- [Foo](foo.md)\n")
        self.write(
            "docs/README.md",
            "- [Engineering](engineering/README.md)\n",
        )

        with self.assertRaisesRegex(release_contract.ContractError, "architecture"):
            release_contract.check_docs_navigation()

        self.write(
            "docs/README.md",
            "- [Architecture](architecture.md)\n"
            "- [Engineering](engineering/README.md)\n",
        )
        with self.assertRaisesRegex(release_contract.ContractError, "release notes"):
            release_contract.check_docs_navigation()

        self.write(
            "docs/README.md",
            "- [Architecture](architecture.md)\n"
            "- [Release notes](releases/v0.24.3.md)\n"
            "- [Engineering](engineering/README.md)\n",
        )
        release_contract.check_docs_navigation()

    def test_docs_navigation_rejects_direct_internal_public_link(self) -> None:
        self.write("docs/architecture.md", "# Architecture\n")
        self.write("docs/releases/v0.24.3.md", "# v0.24.3\n")
        self.write("docs/engineering/foo.md", "# Foo\n")
        self.write("docs/engineering/README.md", "- [Foo](foo.md)\n")
        self.write(
            "docs/README.md",
            "- [Architecture](architecture.md)\n"
            "- [Release notes](releases/v0.24.3.md)\n"
            "- [Engineering](engineering/README.md)\n"
            "- [Foo](engineering/foo.md)\n",
        )

        with self.assertRaisesRegex(release_contract.ContractError, "public docs index"):
            release_contract.check_docs_navigation()

    def test_docs_navigation_rejects_versioned_roadmap_public_link(self) -> None:
        self.write("docs/architecture.md", "# Architecture\n")
        self.write("docs/releases/v0.24.3.md", "# v0.24.3\n")
        self.write("docs/engineering/foo.md", "# Foo\n")
        self.write("docs/engineering/README.md", "- [Foo](foo.md)\n")
        self.write(
            "docs/README.md",
            "- [Architecture](architecture.md)\n"
            "- [Release notes](releases/v0.24.3.md)\n"
            "- [Engineering](engineering/README.md)\n"
            "- [v0.22](roadmap/v0.22.md)\n",
        )

        with self.assertRaisesRegex(release_contract.ContractError, "internal/history"):
            release_contract.check_docs_navigation()

    def test_docs_navigation_rejects_orphan_engineering_file(self) -> None:
        self.write("docs/architecture.md", "# Architecture\n")
        self.write("docs/releases/v0.24.3.md", "# v0.24.3\n")
        self.write("docs/engineering/foo.md", "# Foo\n")
        self.write("docs/engineering/bar.md", "# Bar\n")
        self.write("docs/engineering/README.md", "- [Foo](foo.md)\n")
        self.write(
            "docs/README.md",
            "- [Architecture](architecture.md)\n"
            "- [Release notes](releases/v0.24.3.md)\n"
            "- [Engineering](engineering/README.md)\n",
        )

        with self.assertRaisesRegex(release_contract.ContractError, "engineering index"):
            release_contract.check_docs_navigation()

    def test_docs_navigation_rejects_stale_language_migration_checklist(self) -> None:
        self.write("docs/architecture.md", "# Architecture\n")
        self.write("docs/releases/v0.24.3.md", "# v0.24.3\n")
        self.write(
            "docs/engineering/language-surface-inventory.md",
            "# Language Surface Inventory\n"
            "This is the working checklist for the 0.21 cycle.\n"
            "- [ ] Migrate function spans in PR-5.\n",
        )
        self.write(
            "docs/engineering/README.md",
            "- [Language surface inventory](language-surface-inventory.md)\n",
        )
        self.write(
            "docs/README.md",
            "- [Architecture](architecture.md)\n"
            "- [Release notes](releases/v0.24.3.md)\n"
            "- [Engineering](engineering/README.md)\n",
        )

        with self.assertRaisesRegex(
            release_contract.ContractError, "historical language page"
        ):
            release_contract.check_docs_navigation()

        self.write(
            "docs/engineering/language-surface-inventory.md",
            "# 0.21 Language Migration Record\n\n"
            "The old migration checklist is retained locally.\n\n"
            "Current capabilities: [language support](../language-support.md).\n"
            "Contributor workflow: [add a language](add-a-language.md).\n",
        )
        release_contract.check_docs_navigation()

    def test_docs_navigation_passes_with_single_public_engineering_link(self) -> None:
        self.write("docs/architecture.md", "# Architecture\n")
        self.write("docs/releases/v0.24.3.md", "# v0.24.3\n")
        self.write("docs/engineering/foo.md", "# Foo\n")
        self.write("docs/engineering/README.md", "- [Foo](foo.md)\n")
        self.write(
            "docs/README.md",
            "- [Architecture](architecture.md)\n"
            "- [Release notes](releases/v0.24.3.md)\n"
            "- [Engineering](engineering/README.md)\n"
            "- [Roadmap](roadmap.md)\n",
        )

        release_contract.check_docs_navigation()

    def test_user_workflow_docs_explain_review_and_snapshot_boundaries(self) -> None:
        self.write_product_workflow_docs()
        release_contract.check_user_workflow_docs()

    def test_user_workflow_docs_reject_agent_only_product_positioning(self) -> None:
        self.write_product_workflow_docs()
        self.write(
            "docs/roadmap/v0.24.md",
            "## Phase E — Agent Review Workflow\n\n"
            "Serve the core use case — reviewing changes a coding agent made — end to end.\n",
        )

        with self.assertRaisesRegex(
            release_contract.ContractError, "product positioning"
        ):
            release_contract.check_user_workflow_docs()

    def test_user_workflow_docs_reject_ai_remediation_product_claim(self) -> None:
        self.write_product_workflow_docs()
        self.write(
            "docs/security.md",
            "An explicitly selected `--verify` or MCP "
            "`repopilot_review_change` `verify` check runs on the host with "
            "filesystem and network access. RepoPilot does not sandbox that process. It is a "
            "repository-level audit and AI-remediation context layer.\n",
        )

        with self.assertRaisesRegex(
            release_contract.ContractError, "product positioning"
        ):
            release_contract.check_user_workflow_docs()

    def test_user_workflow_docs_reject_legacy_status_without_proof_mapping(self) -> None:
        self.write(
            "docs/README.md",
            "[Common workflows](commands.md)\n"
            "[First-run configuration and verification](configuration.md)\n",
        )
        self.write(
            "docs/commands.md",
            "Review has ready, review, and blocked merge_readiness statuses.\n",
        )
        self.write("docs/security.md", "--verify runs on the host, with network access.\n")

        with self.assertRaisesRegex(release_contract.ContractError, "proof mapping"):
            release_contract.check_user_workflow_docs()

    def test_user_workflow_docs_require_host_verification_boundary(self) -> None:
        self.write(
            "docs/README.md",
            "[Common workflows](commands.md)\n"
            "[First-run configuration and verification](configuration.md)\n",
        )
        self.write(
            "docs/commands.md",
            "Decision Change Proof merge_readiness `VERIFIED` → `PASS`, "
            "`REVIEW` → `REVIEW`, `BROKEN` → `BLOCK`, `NOT ASSESSED` → `NOT ASSESSED`, "
            "ready while Change Proof is "
            "REVIEW when no sufficient proof policy is selected; failed configured "
            "gate also appears as a proof reason; next action --fail-on-priority "
            "snapshot `HEAD` `dirty` baseline keeps pre-existing changes out "
            "authored each change "
            "configuration.md#first-review-choose-and-run-a-check\n",
        )
        self.write(
            "docs/security.md",
            "An explicitly selected `--verify` command runs on the host.\n",
        )

        with self.assertRaisesRegex(release_contract.ContractError, "network boundary"):
            release_contract.check_user_workflow_docs()

    def test_user_workflow_docs_include_mcp_verification_boundary(self) -> None:
        self.write_product_workflow_docs()
        self.write(
            "docs/security.md",
            "A selected `--verify` command runs on the host with filesystem and "
            "network permissions. RepoPilot does not sandbox that process.\n",
        )

        with self.assertRaisesRegex(release_contract.ContractError, "network boundary"):
            release_contract.check_user_workflow_docs()

    def test_docs_parity_requires_registry_tools_in_both_guides(self) -> None:
        self.write(
            "src/commands/mcp/review.rs",
            'pub const TOOL_NAME: &str = "repopilot_review_change";\n',
        )
        self.write(
            "src/commands/mcp/scan.rs",
            'pub const TOOL_NAME: &str = "repopilot_scan";\n',
        )
        self.write("docs/mcp.md", "`repopilot_review_change`\n")
        self.write("docs/cli.md", "`repopilot_review_change`\n")

        with self.assertRaisesRegex(release_contract.ContractError, "docs parity"):
            release_contract.check_docs_parity()

    def test_mcp_docs_describe_explicit_verification_side_effects(self) -> None:
        stale_mcp = (
            "All tools are annotated as read-only, non-destructive, idempotent, "
            "and closed-world.\n"
            "| `repopilot_review_change` | Changed/full review | `base`, `head` |\n"
        )
        stale_cli = (
            "| Give an MCP client direct read-only analysis tools | "
            "`repopilot mcp --root .` |\n"
        )
        with self.assertRaisesRegex(
            release_contract.ContractError, "MCP verification boundary"
        ):
            release_contract.check_mcp_verification_docs(
                stale_mcp,
                stale_cli,
                "| agent workflow | Exposes read-only local MCP tools |",
            )

        current_mcp = (
            "The default review analysis does not run configured verification checks. "
            "Static scan and explanation tools are annotated as read-only. "
            "The review tool uses conservative annotations: "
            "`readOnlyHint=false`, `destructiveHint=true`, "
            "`idempotentHint=false`, `openWorldHint=true`.\n"
            "| `repopilot_review_change` | Review and optional verification | "
            "`base`, `head`, `verify` |\n"
            "A selected configured check runs on the host with the user's "
            "filesystem and network permissions. RepoPilot does not sandbox it.\n"
        )
        release_contract.check_mcp_verification_docs(
            current_mcp,
            "| Give an MCP client tools | `repopilot mcp --root .` |",
            "| Agent workflow | Exposes local MCP analysis tools |",
        )

    def test_docs_parity_requires_current_schema_and_binary_version(self) -> None:
        self.write(
            "Cargo.toml",
            '[package]\nname = "repopilot"\nversion = "9.9.9"\n',
        )
        self.write(
            "src/report/schema.rs",
            'pub const SCAN_REPORT_SCHEMA_VERSION: &str = "9.99";\n',
        )
        self.write("docs/mcp.md", "")
        self.write("docs/cli.md", "")
        self.write("docs/reports.md", "Binary `0.22.0` emits schema `0.26`.\n")

        with self.assertRaisesRegex(release_contract.ContractError, "docs parity"):
            release_contract.check_docs_parity()

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
        self.write("docs/engineering/README.md", "- [x](rule-scorecard.md)\n")

        with self.assertRaisesRegex(release_contract.ContractError, "stale"):
            release_contract.check_rule_scorecard()

    def test_rule_scorecard_requires_docs_index_link(self) -> None:
        self._write_zoo_scorecard_fixture()
        self.write("docs/engineering/rule-scorecard.md", self._fresh_rule_scorecard())
        self.write("docs/engineering/README.md", "- no scorecard link\n")

        with self.assertRaisesRegex(release_contract.ContractError, "does not link"):
            release_contract.check_rule_scorecard()

    def test_rule_scorecard_passes_when_fresh(self) -> None:
        self._write_zoo_scorecard_fixture()
        self.write("docs/engineering/rule-scorecard.md", self._fresh_rule_scorecard())
        self.write("docs/engineering/README.md", "- [x](rule-scorecard.md)\n")

        release_contract.check_rule_scorecard()

    def _write_review_contract_fixture(self) -> None:
        root = release_contract.ROOT / "tests/fixtures/review-zoo/boundary/access-control"
        for variant in ("safe", "unsafe"):
            (root / variant).mkdir(parents=True, exist_ok=True)
            (root / variant / "expected.json").write_text(
                json.dumps(
                    {
                        "description": variant,
                        "expect": [] if variant == "safe" else [{}],
                        "contract_expect": [] if variant == "safe" else [
                            {
                                "family": "security-boundary",
                                "change": "boundary-changed",
                                "exporter_path": "src/auth/session.ts",
                                "consumer_path": "src/auth/session.ts",
                                "confidence": "limited",
                            },
                            {
                                "family": "test-coverage",
                                "change": "test-missing",
                                "exporter_path": "src/auth/session.ts",
                                "consumer_path": "src/auth/session.ts",
                                "confidence": "limited",
                            },
                        ],
                    }
                ),
                encoding="utf-8",
            )

    def _fresh_review_contract_scorecard(self) -> str:
        fixture_dir = release_contract.ROOT / "tests/fixtures/review-zoo"
        return release_contract.rcs.render_scorecard(
            release_contract.rcs.collect_score(fixture_dir)
        )

    def test_review_contract_scorecard_requires_committed_files(self) -> None:
        with self.assertRaisesRegex(release_contract.ContractError, "Missing review-zoo"):
            release_contract.check_review_contract_scorecard()

    def test_review_contract_scorecard_fails_when_stale(self) -> None:
        self._write_review_contract_fixture()
        self.write("docs/engineering/review-contract-evidence.md", "stale\n")
        self.write("docs/engineering/README.md", "review-contract-evidence.md\n")

        with self.assertRaisesRegex(release_contract.ContractError, "stale"):
            release_contract.check_review_contract_scorecard()

    def test_review_contract_scorecard_requires_docs_index_link(self) -> None:
        self._write_review_contract_fixture()
        self.write(
            "docs/engineering/review-contract-evidence.md",
            self._fresh_review_contract_scorecard(),
        )
        self.write("docs/engineering/README.md", "no scorecard link\n")

        with self.assertRaisesRegex(release_contract.ContractError, "does not link"):
            release_contract.check_review_contract_scorecard()

    def test_review_contract_scorecard_passes_when_fresh(self) -> None:
        self._write_review_contract_fixture()
        self.write(
            "docs/engineering/review-contract-evidence.md",
            self._fresh_review_contract_scorecard(),
        )
        self.write("docs/engineering/README.md", "review-contract-evidence.md\n")

        release_contract.check_review_contract_scorecard()

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
