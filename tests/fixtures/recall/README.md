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
python3 scripts/recall.py run --output recall-run.json
```

`run` builds the workspace scanner (or accepts `--scanner PATH`), materializes
each case outside the fixture tree, and writes a deterministic JSON artifact
with manifest/rules/config hashes, scanner version/schema/revision metadata,
and per-case target-rule results plus corpus-only TP/FN/TN/FP, recall, and
specificity metrics. It does not claim general recall: the current corpus is
synthetic and small, so its result is a regression checkpoint until an
independent real-history holdout is added.
