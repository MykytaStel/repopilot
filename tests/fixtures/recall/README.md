# Held-out recall corpus

`tests/fixtures/recall` contains synthetic cases reserved for recall and
regression measurement. They are deliberately separate from rule fixtures and
zoo expectation labels, so a rule cannot pass by replaying its precision test
set.

The manifest at `tests/recall/manifest.toml` describes each case. A
`seeded-defect` must emit its declared rule; a `safe-guard` must stay silent.
Each case has a stable ID, declared profile, language, rationale, and fixture
path. Run the deterministic contract check from the repository root:

```bash
python3 scripts/recall.py
python3 scripts/recall.py --format json
```

This contract validates corpus structure only. It does not claim recall until a
frozen scanner runner executes the cases and records true-positive, false-
negative, and specificity counts with tool and revision metadata.
