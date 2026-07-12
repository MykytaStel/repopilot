import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "review_delta", ROOT / "scripts" / "review_delta.py"
)
assert SPEC is not None and SPEC.loader is not None
review_delta = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(review_delta)


def finding(stable_id: str, occurrence: str, path: str, priority: str = "P2"):
    return {
        "id": stable_id,
        "occurrence_key": occurrence,
        "title": stable_id,
        "risk": {"priority": priority},
        "evidence": [{"path": path, "line_start": 3}],
    }


class ReviewDeltaTests(unittest.TestCase):
    def test_classifies_new_changed_resolved_and_unchanged_occurrences(self):
        base = [
            finding("same", "occ-same", "src/same.rs"),
            finding("changed", "occ-before", "src/changed.rs"),
            finding("resolved", "occ-resolved", "src/resolved.rs"),
        ]
        head = [
            finding("same", "occ-same", "src/same.rs"),
            finding("changed", "occ-after", "src/changed.rs"),
            finding("new", "occ-new", "src/new.rs", "P1"),
        ]

        delta = review_delta.build_delta(base, head, "base-sha", "head-sha")

        self.assertEqual(
            delta["summary"],
            {
                "new_findings": 1,
                "changed_findings": 1,
                "resolved_findings": 1,
                "unchanged_findings": 1,
            },
        )
        self.assertEqual(delta["new_findings"][0]["id"], "new")
        self.assertEqual(delta["changed_findings"][0]["id"], "changed")
        self.assertEqual(delta["resolved_findings"][0]["id"], "resolved")
        self.assertEqual(delta["base_revision"], "base-sha")
        self.assertEqual(delta["head_revision"], "head-sha")

    def test_falls_back_to_exact_evidence_when_occurrence_key_is_missing(self):
        base = [finding("same", "ignored", "src/a.rs")]
        head = [finding("same", "ignored", "src/a.rs")]
        base[0].pop("occurrence_key")
        head[0].pop("occurrence_key")

        delta = review_delta.build_delta(base, head)

        self.assertEqual(delta["summary"]["unchanged_findings"], 1)

    def test_cli_writes_stable_sorted_json(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            base_path = root / "base.json"
            head_path = root / "head.json"
            output_path = root / "delta.json"
            base_path.write_text('{"findings": []}', encoding="utf-8")
            head_path.write_text(
                json.dumps(
                    {
                        "findings": [
                            finding("later", "occ-2", "src/z.rs", "P2"),
                            finding("first", "occ-1", "src/a.rs", "P1"),
                        ]
                    }
                ),
                encoding="utf-8",
            )

            original_parse_args = review_delta.parse_args
            review_delta.parse_args = lambda: type(
                "Args",
                (),
                {
                    "base": base_path,
                    "head": head_path,
                    "output": output_path,
                    "base_revision": "base",
                    "head_revision": "head",
                },
            )()
            try:
                review_delta.main()
            finally:
                review_delta.parse_args = original_parse_args

            rendered = json.loads(output_path.read_text(encoding="utf-8"))
            self.assertEqual(
                [item["id"] for item in rendered["new_findings"]],
                ["first", "later"],
            )
            self.assertTrue(output_path.read_text(encoding="utf-8").endswith("\n"))


if __name__ == "__main__":
    unittest.main()
