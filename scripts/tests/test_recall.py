from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

import recall  # noqa: E402


RULE_REFERENCE = "### `demo.rule` — Demo\n\n- **Lifecycle:** stable\n"


def write_manifest(root: Path, cases: list[str]) -> Path:
    manifest = root / "manifest.toml"
    manifest.write_text(
        "schema_version = 1\ncorpus = \"test\"\n\n" + "\n".join(cases),
        encoding="utf-8",
    )
    return manifest


def case(case_id: str, path: str, outcome: str = "must-fire", kind: str = "seeded-defect") -> str:
    return f'''[[case]]
id = "{case_id}"
rule_id = "demo.rule"
path = "{path}"
profile = "default"
outcome = "{outcome}"
kind = "{kind}"
language = "rust"
rationale = "A documented test case with explicit expected behavior."
'''


class RecallManifestTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.fixture_root = self.root / "recall"
        (self.fixture_root / "positive").mkdir(parents=True)
        (self.fixture_root / "positive" / "case.rs").write_text("fn main() {}", encoding="utf-8")
        (self.fixture_root / "negative").mkdir()
        (self.fixture_root / "negative" / "case.rs").write_text("fn main() {}", encoding="utf-8")
        self.rules = self.root / "rules-reference.md"
        self.rules.write_text(RULE_REFERENCE, encoding="utf-8")

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def test_valid_manifest_reports_seeded_and_safe_cases(self) -> None:
        manifest = write_manifest(
            self.root,
            [
                case("positive-case", "recall/positive"),
                case("negative-case", "recall/negative", "must-not-fire", "safe-guard"),
            ],
        )
        corpus, cases = recall.validate_manifest(manifest, self.rules, self.fixture_root)
        self.assertEqual(corpus, "test")
        self.assertEqual(recall.summary(corpus, cases)["safe_guards"], 1)

    def test_rejects_duplicate_case_ids(self) -> None:
        manifest = write_manifest(
            self.root,
            [case("same-case", "recall/positive"), case("same-case", "recall/negative", "must-not-fire", "safe-guard")],
        )
        with self.assertRaisesRegex(recall.RecallManifestError, "duplicate case id"):
            recall.validate_manifest(manifest, self.rules, self.fixture_root)

    def test_rejects_unknown_rule(self) -> None:
        manifest = write_manifest(self.root, [case("unknown-case", "recall/positive")])
        manifest.write_text(manifest.read_text().replace("demo.rule", "missing.rule"), encoding="utf-8")
        with self.assertRaisesRegex(recall.RecallManifestError, "unknown rule_id"):
            recall.validate_manifest(manifest, self.rules, self.fixture_root)

    def test_rejects_path_outside_recall_root(self) -> None:
        outside = self.root / "outside"
        outside.mkdir()
        manifest = write_manifest(self.root, [case("escape-case", "outside")])
        with self.assertRaisesRegex(recall.RecallManifestError, "escapes recall fixture root"):
            recall.validate_manifest(manifest, self.rules, self.fixture_root)

    def test_rejects_empty_fixture_directory(self) -> None:
        (self.fixture_root / "empty").mkdir()
        manifest = write_manifest(
            self.root,
            [
                case("empty-case", "recall/empty", "must-not-fire", "safe-guard"),
                case("positive-case", "recall/positive"),
            ],
        )
        with self.assertRaisesRegex(recall.RecallManifestError, "fixture directory is empty"):
            recall.validate_manifest(manifest, self.rules, self.fixture_root)

    def test_kind_and_outcome_must_agree(self) -> None:
        manifest = write_manifest(self.root, [case("mismatch-case", "recall/positive", "must-not-fire")])
        with self.assertRaisesRegex(recall.RecallManifestError, "requires outcome must-fire"):
            recall.validate_manifest(manifest, self.rules, self.fixture_root)

    def test_rejects_unknown_case_fields(self) -> None:
        manifest = write_manifest(self.root, [case("unknown-field-case", "recall/positive")])
        manifest.write_text(manifest.read_text().replace('rationale = "', 'unexpected = "x"\nrationale = "'), encoding="utf-8")
        with self.assertRaisesRegex(recall.RecallManifestError, "unknown fields: unexpected"):
            recall.validate_manifest(manifest, self.rules, self.fixture_root)

    def test_evaluates_target_rule_without_confusing_other_findings(self) -> None:
        manifest = write_manifest(
            self.root,
            [
                case("positive-case", "recall/positive"),
                case("negative-case", "recall/negative", "must-not-fire", "safe-guard"),
            ],
        )
        _, cases = recall.validate_manifest(manifest, self.rules, self.fixture_root)
        positive, negative = cases
        report = {"findings": [{"rule_id": "other.rule"}, {"rule_id": "demo.rule"}]}
        self.assertEqual(recall.evaluate_case_report(positive, report)["status"], "pass")
        self.assertEqual(recall.evaluate_case_report(negative, report)["status"], "fail")

    def test_missing_target_rule_is_a_false_negative(self) -> None:
        manifest = write_manifest(
            self.root,
            [
                case("positive-case", "recall/positive"),
                case("negative-case", "recall/negative", "must-not-fire", "safe-guard"),
            ],
        )
        _, cases = recall.validate_manifest(manifest, self.rules, self.fixture_root)
        result = recall.evaluate_case_report(cases[0], {"findings": []})
        self.assertEqual(result["status"], "fail")
        self.assertEqual(result["kind"], "seeded-defect")


if __name__ == "__main__":
    unittest.main()
