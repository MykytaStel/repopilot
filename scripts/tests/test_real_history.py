from __future__ import annotations

import sys
import tempfile
import textwrap
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

import real_history  # noqa: E402

RULES = "### `demo.rule` — Demo\n"
ZOO = '[[repo]]\nname = "zoo/repo"\nurl = "https://github.com/zoo/repo.git"\nsha = "' + "a" * 40 + '"\n'


def manifest(case_blocks: str) -> str:
    return textwrap.dedent(
        f'''\
        schema_version = 1
        corpus = "test"
        protocol = "dual-v1"

        {case_blocks}
        '''
    )


def case(case_id: str = "case-one", repo: str = "owner/repo", **overrides: str) -> str:
    values = {
        "id": case_id,
        "repo": repo,
        "url": f"https://github.com/{repo}.git",
        "pull_request": "1",
        "base_sha": "a" * 40,
        "head_sha": "b" * 40,
        "merge_sha": "c" * 40,
        "language": "python",
        "source_kind": "merged-pull-request",
        "baseline_ids": '["python.compile"]',
        "label_state": "pending",
    }
    values.update(overrides)
    lines = ["[[case]]"]
    for key, value in values.items():
        if key in {"pull_request", "baseline_ids"} or value.startswith('"'):
            rendered = value
        else:
            rendered = f'"{value}"'
        lines.append(f"{key} = {rendered}")
    return "\n".join(lines) + "\n"


class HoldoutManifestTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        root = Path(self.tmp.name)
        self.manifest = root / "manifest.toml"
        self.rules = root / "rules.md"
        self.zoo = root / "zoo.toml"
        self.rules.write_text(RULES, encoding="utf-8")
        self.zoo.write_text(ZOO, encoding="utf-8")
        self.root = root

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def write(self, blocks: str) -> None:
        self.manifest.write_text(manifest(blocks), encoding="utf-8")

    def test_valid_pending_cases_require_independent_repositories(self) -> None:
        self.write(case() + "\n" + case("case-two", "other/repo"))
        corpus, protocol, cases = real_history.validate_manifest(self.manifest, self.rules, self.zoo)
        self.assertEqual((corpus, protocol), ("test", "dual-v1"))
        self.assertEqual(real_history.summary(corpus, protocol, cases)["cases"], 2)

    def test_rejects_zoo_overlap(self) -> None:
        self.write(case(repo="zoo/repo") + "\n" + case("case-two", "other/repo"))
        with self.assertRaisesRegex(real_history.HoldoutManifestError, "overlaps precision zoo"):
            real_history.validate_manifest(self.manifest, self.rules, self.zoo)

    def test_rejects_mutable_or_short_revision(self) -> None:
        self.write(case(base_sha="main") + "\n" + case("case-two", "other/repo"))
        with self.assertRaisesRegex(real_history.HoldoutManifestError, "full lowercase SHA-1"):
            real_history.validate_manifest(self.manifest, self.rules, self.zoo)

    def test_pending_case_cannot_smuggle_labels(self) -> None:
        self.write(case(annotator_a='"defect-present"') + "\n" + case("case-two", "other/repo"))
        with self.assertRaisesRegex(real_history.HoldoutManifestError, "pending case cannot contain labels"):
            real_history.validate_manifest(self.manifest, self.rules, self.zoo)

    def test_adjudicated_disagreement_requires_rationale(self) -> None:
        block = case(
            label_state='"adjudicated"',
            annotator_a='"defect-present"',
            annotator_b='"no-defect"',
            adjudicated='"uncertain"',
        )
        self.write(block + "\n" + case("case-two", "other/repo"))
        with self.assertRaisesRegex(real_history.HoldoutManifestError, "disagreement needs"):
            real_history.validate_manifest(self.manifest, self.rules, self.zoo)

    def test_excluded_case_requires_reason(self) -> None:
        self.write(case(label_state='"excluded"') + "\n" + case("case-two", "other/repo"))
        with self.assertRaisesRegex(real_history.HoldoutManifestError, "excluded case needs"):
            real_history.validate_manifest(self.manifest, self.rules, self.zoo)


if __name__ == "__main__":
    unittest.main()
