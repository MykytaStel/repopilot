"""Append labels to tests/integrity/labels.toml: `ID kinds verdict note`.

    python3 scripts/integrity_corpus_label.py <<'LABELS'
    ic-003 | test-substituted | justified | EOL model swapped, same checks
    ic-004 | - | none |
    LABELS
"""

from __future__ import annotations

import json
import sys
import tomllib
from pathlib import Path

LABELS = Path(__file__).resolve().parent.parent / "tests" / "integrity" / "labels.toml"
HEADER = """# Integrity corpus labels (see README.md). One [[label]] per manifest id.
[meta]
labeler = "Claude (Anthropic, Opus 5.5) for the maintainer; same model wrote the detectors"
status = "single-labeler, exploratory, pending maintainer review"
method = "compact test/gate diff view (integrity_corpus.py show --compact), labeled before any RepoPilot run"
"""

KINDS = {
    "skip-added", "focus-added", "test-removed", "test-substituted", "assertion-removed",
    "assertion-trivialized", "expectation-rewritten", "suppression-added", "gate-relaxed",
}


def main() -> None:
    existing = tomllib.loads(LABELS.read_text()).get("label", []) if LABELS.exists() else []
    done = {entry["id"] for entry in existing}
    text = LABELS.read_text() if LABELS.exists() else HEADER
    for line in sys.stdin:
        if not line.strip():
            continue
        corpus_id, kinds, verdict, note = (part.strip() for part in (line.split("|") + [""])[:4])
        kind_list = [] if kinds in ("", "-") else [kind.strip() for kind in kinds.split(",")]
        unknown = set(kind_list) - KINDS
        if unknown or verdict not in ("weakened", "justified", "none") or corpus_id in done:
            sys.exit(f"bad label line: {line.strip()} ({unknown or verdict or 'duplicate'})")
        if (verdict == "none") != (not kind_list):
            sys.exit(f"verdict and kinds disagree: {line.strip()}")
        text += f"\n[[label]]\nid = {json.dumps(corpus_id)}\nkinds = {json.dumps(kind_list)}\nverdict = {json.dumps(verdict)}\n"
        if note:
            text += f"note = {json.dumps(note)}\n"
        done.add(corpus_id)
    LABELS.write_text(text)
    print(f"{len(done)} labeled")


if __name__ == "__main__":
    main()
