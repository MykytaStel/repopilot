# Independent real-history holdout

## Controlled ChangeProof Benchmark v1

`changeproof.toml` is a small committed corpus over existing review-zoo
safe/unsafe fixtures. It evaluates the canonical `change_proof`, not merely
rule IDs. Every case is confined under `tests/fixtures/review-zoo`, identifies
its expected decision and any independently known dimensions, and hashes both
the materialized before/after source tree and the fixture's non-empty mutation
rationale. Fixture paths must be relative regular files beneath the committed
root: symlinks and `.git` metadata are rejected. `unknown`,
`unsupported`, and `unavailable` are not empty positives and never become a
precision or recall denominator.

```bash
python3 scripts/changeproof_benchmark.py check
python3 scripts/changeproof_benchmark.py collect --scanner target/release/repopilot \
  --output /tmp/changeproof-benchmark.json
python3 scripts/changeproof_benchmark.py validate-result \
  --artifact /tmp/changeproof-benchmark.json
python3 scripts/changeproof_benchmark.py report \
  --artifact /tmp/changeproof-benchmark.json --output /tmp/changeproof-benchmark.md
```

`--scanner` is required: collection never builds a binary and therefore never
downloads dependencies. Collection runs one cold and two warm reviews of every
fixture. It retains only
the canonical proof projection, semantic SHA-256, bounded timing/RSS metadata,
and oracle hash; raw review output and command logs are discarded. Validation
uses an exact schema, recomputes all hashes and dimensions, and rejects a
nondeterministic case from every measured denominator. Claims remain
unavailable in v1 because RepoPilot has no canonical observed-claims projection;
an expected empty claim set is not fabricated as a pass. The first eight cases
cover authorization, behavioural network-call, and SQL-taint safe/unsafe
changes. Delivery `LIMITED`, verification stale/unavailable, and no-op cases
need a confined policy-aware adapter and remain explicit gaps, not green
coverage claims. The pinned real-history corpus below remains a separate,
pending independent-adjudication protocol.

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
python3 scripts/differential.py budget-check \
  --artifact differential-run.json
python3 scripts/differential.py coverage-audit \
  --artifact differential-run.json --output coverage-audit.md
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

The release binary's synthetic scan resource matrix uses the separate
`scan-rss-v1` policy:

```bash
npm run scan:resource
```

It runs full and changed scans in cold and warm phases, writes the host profile
and RSS samples to `/tmp/repopilot-scan-resource-matrix.json`, and fails when a
required sample is unavailable or over its ceiling.

The differential collection artifact is schema 2. Each baseline, base scan, and
review run has validated monotonic start/finish telemetry. Review runs may also
record evidence-ready and decision-ready phase events from RepoPilot's internal
timings. The first baseline adapters normalize `python.compile` diagnostics as
`path:line:error-kind` identities and `python.tests` failures as stable pytest
node identities. A clean successful baseline is measured with an empty evidence
set; conftest import failures are normalized to their collection path; an
unrecognized failure stays explicitly unavailable. `validate-result` reports
measured, unavailable, and untracked coverage separately. Command success or
output hashes alone are not treated as evidence overlap. A baseline diagnostic
ID is not automatically comparable to a RepoPilot finding ID: static
duplicate-work requires an explicit `review-exact-v1` comparison mapping,
otherwise that measurement remains unavailable.

## Local real-project sandbox

The B5 runner is intentionally separate from the differential and real-history
collectors. It consumes a pinned TOML manifest and writes only to the ignored
`.zoo/repopilot-validation/` directory:

```bash
python3 scripts/sandbox.py check \
  --manifest .zoo/repopilot-validation/manifest.toml
python3 scripts/sandbox.py run \
  --manifest .zoo/repopilot-validation/manifest.toml \
  --case ripgrep-control \
  --source .zoo/ripgrep \
  --scanner target/release/repopilot \
  --output .zoo/repopilot-validation/runs/ripgrep-control.json
python3 scripts/sandbox.py validate-artifact \
  --manifest .zoo/repopilot-validation/manifest.toml \
  --artifact .zoo/repopilot-validation/runs/ripgrep-control.json
python3 scripts/sandbox.py pilot \
  --manifest .zoo/repopilot-validation/manifest-pilot.toml \
  --source-root .zoo \
  --scanner target/release/repopilot \
  --repeats 3 \
  --output .zoo/repopilot-validation/runs/technical-pilot-summary.json
python3 scripts/sandbox.py validate-pilot \
  --manifest .zoo/repopilot-validation/manifest-pilot.toml \
  --artifact .zoo/repopilot-validation/runs/technical-pilot-summary.json
python3 scripts/sandbox.py mutation \
  --manifest .zoo/repopilot-validation/manifest-mutation.toml \
  --source-root .zoo \
  --scanner target/release/repopilot \
  --output .zoo/repopilot-validation/runs/express-mutation-summary.json
python3 scripts/sandbox.py validate-mutation \
  --manifest .zoo/repopilot-validation/manifest-mutation.toml \
  --artifact .zoo/repopilot-validation/runs/express-mutation-summary.json
python3 scripts/sandbox.py report \
  --manifest .zoo/repopilot-validation/manifest-pilot.toml \
  --artifact .zoo/repopilot-validation/runs/technical-pilot-summary.json \
  --format markdown
python3 scripts/sandbox.py report \
  --manifest .zoo/repopilot-validation/manifest-mutation.toml \
  --artifact .zoo/repopilot-validation/runs/express-mutation-summary.json \
  --format markdown --output .zoo/repopilot-validation/runs/express-mutation-report.md
python3 scripts/sandbox.py metrics \
  --manifest .zoo/repopilot-validation/manifest-mutation.toml \
  --artifact .zoo/repopilot-validation/runs/express-mutation-summary.json \
  --output .zoo/repopilot-validation/runs/express-mutation-metrics.json
python3 scripts/sandbox.py validate-metrics \
  --manifest .zoo/repopilot-validation/manifest-mutation.toml \
  --artifact .zoo/repopilot-validation/runs/express-mutation-summary.json \
  --metrics .zoo/repopilot-validation/runs/express-mutation-metrics.json
python3 scripts/sandbox.py metrics-report \
  --manifest .zoo/repopilot-validation/manifest-mutation.toml \
  --artifact .zoo/repopilot-validation/runs/express-mutation-summary.json \
  --metrics .zoo/repopilot-validation/runs/express-mutation-metrics.json \
  --output .zoo/repopilot-validation/runs/express-mutation-metrics.md
```

The manifest requires a full source SHA, a content-addressed image, an
allowlisted argument-vector command, and `network = "none"` for measured
commands. The runner copies a verified source into a run-owned case directory;
it never mutates an existing zoo clone. Build/test oracles run only through the
Docker adapter (`--pull=never`, one workspace mount, 2 CPU, 4 GiB, bounded
timeouts). Scanner JSON is written to a run-owned report file and normalized
before the smaller stdout/stderr log bound is applied; reports above the
8 MiB normalization limit remain explicitly unavailable. If Docker or the
scanner is unavailable, the result remains `unavailable` and retains a cleanup
receipt. Command output is hashed and
bounded; raw stdout/stderr and finding snippets are not persisted. On Linux and
macOS, command artifacts now record positive peak RSS through the shared
`posix-time-v1` `/usr/bin/time` sampler. Unsupported platforms, malformed
sampler output, and timeouts remain `unavailable`; a missing sample never
becomes a zero-memory claim.

`pilot` writes one artifact per case and repetition plus a summary. A case is
`passed` only when every oracle passes and every normalized scan hash is stable;
otherwise the summary reports `drift` or `unavailable`. This is a technical
reproducibility result, separate from mutation and human-usability metrics.

`mutation` consumes cases with `mutation_kind = "violation"` or
`"negative-control"`. A violation may expect an oracle failure; that is a
passing mutation case only when baseline/setup and reverse patch pass as well.
The summary keeps the independent oracle state visible and remains separate
from production recall or precision.

Mutation cases may declare `expected_rule_ids`. For a `violation`, every
declared rule must be observed in the mutated scan; for a `negative-control`,
declared rules must not be introduced by the patch. The runner therefore makes
a separate baseline scan and compares stable normalized rule/path/evidence
identities. Evidence uses a digest of the reported snippet and the finding ID,
with a line fallback when those fields are unavailable, so line shifts and
pre-existing findings do not invalidate a clean negative control. The receipt
records expected, observed-new, and all observed rule IDs, and metrics expose
exact violation and negative-control rates. Cases without this field retain
lifecycle-only semantics and cannot support an exact rule-signal claim.
Mutation metrics also keep `tuning` and `evaluation` cases separate; tuning
results must not be presented as held-out evaluation.

Cases may set `analysis_mode = "changed"` when the mutation is intended to
exercise changed-scan semantics; the default is `"default"`. The selected mode
is recorded in the artifact and report so a full scan cannot be mistaken for a
change-review measurement.

`report` validates a pilot or mutation summary against its manifest before
rendering a deterministic human-readable Markdown or text report. It shows the
overall status, per-case status, oracle states, normalized finding counts and
hashes when available, the mutation lifecycle, limits, and a next action. An
expected oracle failure for a `violation` is explained as a successful mutation
case; `unavailable` remains an explicit missing-evidence state. The report does
not include raw command output or finding snippets. It includes a resource
evidence table per case with median analyzed-command peak RSS, the available
sample count, and sampler source. Unavailable samples remain visible and are
never treated as zero memory use. Without `--output` it is printed to the
terminal; with `--output` it is saved under the ignored sandbox directory.

`metrics` recomputes a JSON artifact from the validated summary and its child
artifacts. It records numerator/denominator pairs, 95% Wilson intervals,
coverage, lifecycle, determinism or mutation scan observations, analyze wall
time, and peak RSS availability. `validate-metrics` rejects edited or stale
metrics by recomputing the artifact. The current mutation manifest does not
declare exact expected rule IDs or a baseline scan, so TP/FN/TN/FP and exact
additional value remain `unavailable`; this is a protocol boundary, not a
zero-quality result. `metrics-report` validates before rendering the bounded
Markdown report.

`coverage-audit` renders the same validated denominators by baseline ID. It
keeps unavailable reasons visible and points to the next adapter work without
scoring precision, recall, utility, or overlap.

Resource samples follow the same boundary: cumulative child RSS is measured
only when a positive per-command delta is available. A zero or unsupported
delta is reported as unavailable, never as proof of zero memory use.
On Linux and macOS the collector uses `/usr/bin/time` and records the sampler
source; Windows and systems without a supported time format stay unavailable.
Repeated runs are labeled `cold` for the first execution and `warm` for later
executions, and pilot reports preserve those RSS medians separately.

The manifest also carries `differential-rss-v1`, a review-workload budget of
65,536 KiB for cold runs and 49,152 KiB for warm runs. `budget-check` requires
the `posix-time-v1` source and both phases for every case. These ceilings are a
regression gate for the pinned packet-v9 host and workload; changing either
requires a new measurement and an explicit policy change. An unavailable
sample fails the check.

For `python.tests`, a failed pytest node remains baseline-only in the default
static review run. A path or changed test name cannot prove the same failure
identity. To collect an explicit verification comparison, provide a temporary
config that defines the exact check and pin it in the artifact:

```toml
[[verification.checks]]
id = "python.tests"
role = "test"
program = "python3"
args = ["-m", "pytest", "-q"]
```

```bash
python3 scripts/differential.py collect \
  --scanner target/release/repopilot \
  --review-config /tmp/repopilot-python-tests.toml \
  --review-verify-python-tests \
  --output differential-run.json
```

Only complete, revision-compatible output from that explicit check is recorded
as `review-verification-v1` exact node evidence. The artifact stores the config
hash and the coverage audit reports it separately. Any resulting overlap is
evidence about executed verification work; it is not static-review detection
and cannot be used to claim precision or recall.

Resource budgets are workload-bound. The committed `differential-rss-v1`
policy declares `verification_checks = []`, so it applies to static review
only. A packet collected with explicit verification must use a separately
reviewed policy file whose `verification_checks` exactly match the artifact;
otherwise `budget-check` fails with `workload_mismatch` instead of comparing
incompatible RSS samples:

```toml
[resource_policy]
schema_version = 1
policy_id = "differential-rss-python-tests-v1"
workload = "v0.23-real-history-expanded-review-python-tests"
source = "posix-time-v1"
unit = "KiB"
statistic = "median"
required_phases = ["cold", "warm"]
ceiling_kb_by_phase = { cold = 65536, warm = 49152 }
unavailable = "fail"
verification_checks = ["python.tests"]
```

Pass a temporary policy with `--resource-policy /tmp/python-tests-policy.toml`;
do not reuse the static policy or treat one local packet as a universal memory
claim.

When one reviewer is available, `pilot-template` creates an exploratory
worksheet over the same pinned differential artifact. `pilot-score` reports
only captured novel-evidence, timing, determinism, and resource fields;
decision latency and time to first useful evidence remain unavailable until the
collector records the required events. Duplicate work additionally requires
an explicit `review-exact-v1` or `review-verification-v1` identity mapping.
Validate the metrics artifact before rendering or circulating its report.

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
metric is intentionally limited to the measured delivery action-reference,
security-boundary, and test-coverage IDs. Paths, evidence prose, and runtime
semantics are not treated as independently validated by this family/change
measurement.

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
single-expert exploratory evidence over the measured delivery action-reference,
security-boundary, and test-coverage subset; it is not independent validation,
a production estimate, or evidence for the unmeasured contract families.

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
