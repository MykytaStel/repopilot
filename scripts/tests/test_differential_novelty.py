from __future__ import annotations

import sys
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from differential_novelty import evidence_keys, novel_evidence_keys  # noqa: E402


class DifferentialNoveltyTests(unittest.TestCase):
    def test_existing_base_evidence_is_not_novel(self) -> None:
        base = [
            {
                "rule_id": "architecture.circular-dependency",
                "evidence": [
                    {
                        "path": "src/flask/__init__.py",
                        "line_start": 1,
                        "snippet": "Cycle: src/flask/__init__.py -> src/flask/app.py",
                    }
                ],
            }
        ]
        review = [*base, {"rule_id": "security.secret-candidate", "evidence": []}]

        base_keys = evidence_keys(base)
        novel = novel_evidence_keys(review, base_keys)

        self.assertEqual(len(base_keys), 1)
        self.assertEqual(novel, ["security.secret-candidate"])


if __name__ == "__main__":
    unittest.main()
