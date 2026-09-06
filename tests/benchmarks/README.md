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
```

Both entries are intentionally `pending`. This PR does not invent defect labels
or claim real-history recall. The next benchmark slice will clone the pinned
revisions, collect baseline outcomes, run RepoPilot, and publish the dual-label
adjudication artifact.
