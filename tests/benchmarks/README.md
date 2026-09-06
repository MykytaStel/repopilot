# Independent real-history holdout

`manifest.toml` defines a future evidence corpus from immutable merged pull
requests. Its repositories are outside the precision zoo, and each case pins
base, head, and merge SHAs rather than a mutable branch.

The protocol requires two independent labels before a case is admissible for a
recall result. A disagreement must have an explicit adjudication record. Each
case also declares the existing compiler/test baselines that the future runner
will execute and compare against RepoPilot's actionable evidence.

Validate the protocol contract without cloning repositories:

```bash
python3 scripts/real_history.py check
python3 scripts/real_history.py check --format json
python3 scripts/real_history.py collect --scanner target/release/repopilot \
  --timeout 60 --output real-history-run.json
python3 scripts/real_history.py validate-result \
  --artifact real-history-run.json
python3 scripts/real_history.py template --artifact real-history-run.json \
  --reviewer a --output annotation-a.toml
python3 scripts/real_history.py template --artifact real-history-run.json \
  --reviewer b --output annotation-b.toml
python3 scripts/real_history.py validate-annotation --artifact real-history-run.json \
  --annotation annotation-a.toml --reviewer a
python3 scripts/real_history.py adjudication-template \
  --artifact real-history-run.json --annotation-a annotation-a.toml \
  --annotation-b annotation-b.toml --output adjudication.toml
python3 scripts/real_history.py validate-adjudication \
  --artifact real-history-run.json --annotation-a annotation-a.toml \
  --annotation-b annotation-b.toml --annotation adjudication.toml
python3 scripts/real_history.py metrics --artifact real-history-run.json \
  --annotation-a annotation-a.toml --annotation-b annotation-b.toml \
  --annotation adjudication.toml --output real-history-metrics.json
```

Both entries are intentionally `pending`. This PR does not invent defect labels
or claim real-history recall. `collect` clones the pinned revisions into a
temporary workspace, runs only the allowlisted baselines and a base-to-head
RepoPilot review, and records statuses, output hashes, and stable evidence
hashes. Baseline failures are retained as observations; they do not become
RepoPilot findings. The collected artifact still needs dual-label adjudication.

`validate-result` checks the artifact against the current manifest, including
the manifest hash, immutable PR revisions, scanner provenance, baseline command
allowlist, and one review observation per case.

`template` creates one deterministic worksheet per independent reviewer. It
copies only pinned case identity, baseline statuses, and observed in-diff rule
IDs from the collection artifact; labels and rationales stay empty. Complete
both worksheets from the diff and repository evidence, then run
`validate-annotation` before creating the `adjudication-template`. The latter
copies both independent labels and leaves the adjudicated label and rationale
empty for an explicit third decision. These commands create evidence packets;
`validate-adjudication` requires that decision and its rationale to be filled
and verifies that the copied independent labels still match both worksheets.
`metrics` is then allowed to calculate corpus-only case-level TP/FN/TN/FP,
recall, specificity, and precision with Wilson 95% intervals. Its output pins
all three input hashes and states that the result is descriptive evidence for
this holdout, not a production or language-wide estimate.
