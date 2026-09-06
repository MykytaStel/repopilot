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
