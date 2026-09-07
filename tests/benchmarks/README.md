# Independent real-history holdout

`manifest.toml` defines a future evidence corpus from immutable merged pull
requests. Its repositories are outside the precision zoo, and each case pins
base, head, and merge SHAs rather than a mutable branch.

The protocol requires two independent labels before a case is admissible for a
recall result. A disagreement must have an explicit adjudication record. Each
case also declares the existing compiler/test baselines that the future runner
will execute and compare against RepoPilot's actionable evidence.

The differential utility protocol is preregistered separately:

```bash
python3 scripts/differential.py check
python3 scripts/differential.py check --format json
python3 scripts/differential.py collect --repo-root . \
  --scanner target/release/repopilot --timeout 300 \
  --output differential-run.json
python3 scripts/differential.py validate-result \
  --artifact differential-run.json
python3 scripts/differential.py pilot-template \
  --artifact differential-run.json --reviewer expert \
  --output pilot.toml
python3 scripts/differential.py pilot-validate \
  --artifact differential-run.json --pilot pilot.toml
python3 scripts/differential.py pilot-score \
  --artifact differential-run.json --pilot pilot.toml \
  --output pilot-metrics.json
python3 scripts/differential.py pilot-validate-metrics \
  --artifact differential-run.json --pilot pilot.toml \
  --metrics pilot-metrics.json
python3 scripts/differential.py pilot-metrics-report \
  --metrics pilot-metrics.json --output pilot-metrics.md
```

It freezes the six utility measurements, the baseline set, the six-case
holdout set, and three repetitions before a benchmark run. `collect` executes the
allowlisted checks and repeated RepoPilot reviews on exact-SHA worktrees and
records timings, output hashes, determinism, scanner provenance, best-effort
child-resource samples, and the base scan's exact evidence identities. Review
evidence is marked novel only when its rule/path/line/snippet identity is absent
from that base scan. The artifact remains unlabeled and makes no utility claim
until the independent labeling and scoring step exists.

When one reviewer is available, `pilot-template` creates an exploratory
worksheet over the same pinned differential artifact. `pilot-score` reports
only captured novel-evidence, timing, determinism, and resource fields;
decision latency, time to first useful evidence, and duplicate work remain
unavailable until the collector records the required events. Validate the
metrics artifact before rendering or circulating its report.

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
python3 scripts/real_history.py validate-metrics --artifact real-history-run.json \
  --annotation-a annotation-a.toml --annotation-b annotation-b.toml \
  --annotation adjudication.toml --metrics real-history-metrics.json
python3 scripts/real_history.py metrics-report \
  --metrics real-history-metrics.json --output real-history-metrics.md
python3 scripts/real_history.py label-coverage \
  --artifact real-history-run.json --output label-coverage.json
python3 scripts/real_history.py validate-label-coverage \
  --artifact real-history-run.json --coverage label-coverage.json
python3 scripts/real_history.py coverage-report \
  --coverage label-coverage.json --output label-coverage.md
```

All six entries are intentionally `pending`. The corpus was expanded before
any labels were collected; the earlier two-case packets remain historical and
are not silently merged into the expanded corpus. This PR does not invent
defect labels or claim real-history recall. `collect` clones the pinned revisions into a
temporary workspace, runs only the allowlisted baselines and a base-to-head
RepoPilot review, and records statuses, output hashes, and stable evidence
hashes. Baseline failures are retained as observations; they do not become
RepoPilot findings. The collected artifact still needs dual-label adjudication.

`validate-result` checks the artifact against the current manifest, including
the manifest hash, immutable PR revisions, scanner provenance, baseline command
allowlist, and one review observation per case.

Collection schema 2 also records the stable `change_proof.contract_deltas`
family/change IDs and a hash over those IDs. The collector preserves all
currently emitted contract identities, while the first independent labeling
metric is intentionally limited to the security and test IDs. Paths, evidence
prose, and runtime semantics are not treated as independently validated by
this family/change measurement.

When only one expert is available for the contract surface, use the separate
exploratory pilot. It is blinded and hash-pinned, but it does not weaken the
dual-review protocol or its metrics:

```bash
python3 scripts/real_history.py contract-pilot-template \
  --artifact real-history-run.json --pilot-reviewer expert \
  --output contract-pilot.toml
# Fill contract_label, expected_contract_ids, and rationale from the pinned diff.
python3 scripts/real_history.py validate-contract-pilot \
  --artifact real-history-run.json --pilot contract-pilot.toml
python3 scripts/real_history.py contract-pilot-metrics \
  --artifact real-history-run.json --pilot contract-pilot.toml \
  --output contract-pilot-metrics.json
python3 scripts/real_history.py validate-contract-pilot-metrics \
  --artifact real-history-run.json --pilot contract-pilot.toml \
  --metrics contract-pilot-metrics.json
```

The pilot reports case outcomes and per-ID confusion counts with Wilson
intervals. The final validation command recomputes the score from the pinned
inputs and rejects edited or stale metrics. Its scope is explicitly
single-expert exploratory evidence over the measured security/test subset; it
is not independent validation, a production estimate, or evidence for the
unmeasured contract families.

The differential pilot follows the same integrity rule. Run
`pilot-validate-metrics` before circulating `pilot-metrics.json`; it rejects
edited counts, stale hashes, missing fields, and extra fields. The
`pilot-metrics-report` output keeps unavailable measurements visible, so missing
event instrumentation cannot become an invented latency or duplicate-work
result.

When only one expert is available, `pilot-template` creates a blinded
`single-expert-pilot-v1` worksheet. Fill one label and rationale per case, then
run `pilot-validate` and `pilot-score`. The resulting report is explicitly
exploratory: it can show case-level outcomes, determinism, and captured timing
or resource summaries, but it is not independent validation and cannot support
a production or language-wide claim. The pilot does not modify the
dual-review protocol or its metrics.

`template` creates one deterministic worksheet per independent reviewer. It
copies only pinned case identity and baseline statuses from the collection
artifact. The worksheet is explicitly blinded: RepoPilot findings stay in the
collection artifact and are not shown to the labeler. Labels and rationales
stay empty. Complete both worksheets from the diff and repository evidence, then run
`validate-annotation` before creating the `adjudication-template`. The latter
copies both independent labels and leaves the adjudicated label and rationale
empty for an explicit third decision. These commands create evidence packets;
`validate-adjudication` requires that decision and its rationale to be filled
and verifies that the copied independent labels still match both worksheets.
`metrics` is then allowed to calculate corpus-only case-level TP/FN/TN/FP,
recall, specificity, and precision with Wilson 95% intervals. Its output pins
all three input hashes and states that the result is descriptive evidence for
this holdout, not a production or language-wide estimate.

`validate-metrics` recomputes that score from the pinned inputs and rejects a
hand-edited or stale JSON artifact. `metrics-report` renders a deterministic
Markdown summary for a validated dual-review or exploratory pilot metrics file;
the report repeats the scope and limitation so it cannot be mistaken for a
broader quality claim.

`label-coverage` audits the evidence packet before scoring. With no worksheet
arguments it reports the collected holdout as pending; with `--pilot` it marks
the packet as single-expert exploratory; with `--annotation`, `--annotation-a`,
and `--annotation-b` it validates the dual-adjudicated packet. The output lists
unreviewed measured observations and emitted contract IDs outside the measured
registry. `coverage-report` renders the same gaps as deterministic Markdown.
Run `validate-label-coverage` before circulating the JSON or Markdown packet;
it recomputes the complete audit and rejects edits or stale label inputs. This
is a completeness audit, not a recall or precision estimate.

The same metrics artifact now includes per-ID contract confusion counts and
Wilson intervals for `security-boundary/*` and `test-coverage/*`. A reviewer
labels `expected_contract_ids` from the diff and repository evidence; the
machine observation remains in the collection artifact and is not copied into
the blinded worksheet.
