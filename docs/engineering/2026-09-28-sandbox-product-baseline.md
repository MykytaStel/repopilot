# Sandbox and product validation baseline — 2026-09-28

Status: inspected; existing artifact validation only, no fresh workload execution.
Code baseline: `origin/main` and the planning branch both resolve to `5ff7cf2a`.

This record inventories existing engineering capability, reusable local
assets, and measurements still needed to establish everyday product usefulness.

## Asset locations

The primary checkout contains `.zoo/repopilot-validation/` and
`.repopilot/evidence/v0.23/`. The active planning worktree contains neither
directory. Earlier worktree-specific statements that `.zoo/` was unavailable
do not establish that the assets are absent from the machine.

The local asset directories are ignored by Git. Their presence is not a
portable test setup. Future work must record source and scanner identities,
resolve the correct asset root, and write new runs into separate directories.
Existing artifacts and manifests must not be overwritten to make them agree.

## What is already implemented

| Layer | Existing assets | What it can establish |
| --- | --- | --- |
| Real-project sandbox | `scripts/sandbox.py`, `sandbox_runner.py`, `sandbox_process.py`, local pinned manifests | Controlled before/mutation/oracle/revert lifecycle; normalized scan observations; repeatability and available timing/resource samples |
| Exact rule observation | Mutation cases with `expected_rule_ids`, baseline analysis, `tuning` and `evaluation` splits | Whether a specified supported rule appears on the violation and stays absent on its matched negative control |
| Controlled ChangeProof benchmark | `scripts/changeproof_benchmark.py`, `tests/benchmarks/changeproof.toml` | Canonical review-proof observations on committed safe/unsafe fixtures, with explicit unavailable dimensions |
| Real-history collection | `scripts/real_history.py`, `tests/benchmarks/manifest.toml` | Observations on fixed revisions; independent labeling and adjudication remain a separate step |
| Differential utility | `scripts/differential.py`, `tests/benchmarks/differential.toml` | Existing repository checks versus RepoPilot on six fixed cases, repeated three times, with evidence identity and comparability boundaries |
| Evidence audit | `scripts/phase0_evidence.py`, rule scorecard and zoo expectations | Which protocols and artifacts are valid and which labels or comparable measurements are missing |
| Human use | No completed usability study established by this inspection | First-use friction, comprehension, decision changes, and voluntary repeat use require observation |

The sandbox currently invokes `scan` through `_run_static_analysis`. It does
not, by itself, exercise the complete interactive `review` setup path or prove
all ChangeProof decisions. The controlled ChangeProof benchmark and the
real-history/differential collectors already provide complementary review
coverage; extend or run the relevant existing layer rather than creating a
parallel benchmark framework.

## Revalidated and inspected local artifacts

Paths below are relative to the primary checkout. `valid` means the validator
accepted the artifact contract; `passed`/`unavailable` are recorded experiment
outcomes. Neither means the current product has just passed a fresh scan.

| Artifact | Result of this inspection | Boundary / next action |
| --- | --- | --- |
| `.zoo/repopilot-validation/manifest.toml` | `sandbox.py check`: valid, one `ripgrep-control` case | The separate pilot manifest declares four projects/cases |
| `runs/technical-pilot-summary.json` under the sandbox directory | Recorded `unavailable`; current pilot manifest mismatch; no matching top-level local manifest hash | Historical packet; retain it with its original identity, do not silently relabel it |
| `runs/technical-pilot-summary-after-images.json` | `validate-pilot`: valid; recorded `passed` for ripgrep, Cobra, Express, FastAPI | A usable historical technical pilot; refresh against a deliberately selected current binary |
| `runs/express-mutation-summary.json` | `validate-mutation`: valid; recorded four passed lifecycle cases | Its saved metrics report lacks expected rule identities and a baseline scan; TP/FP/FN and additional value are unavailable |
| `experiments/unresolved-local-import-packet/runs/summary.json` | `validate-mutation`: rejected because the summary and current manifest do not match; stored status is `failed` | Recover the matching immutable pair or recollect; not current detector evidence |
| `experiments/cobra-unresolved-local-import-packet/runs/summary.json` | `validate-mutation`: valid; recorded `unavailable` for all four cases | Rule expectations and tuning/evaluation splits exist; inspect missing prerequisites before recollection |

The old Express metrics report records `lifecycle_pass_rate = 4/4` and
`violation_signal_rate = 0/2`, while exact rule metrics are unavailable. This
illustrates why successful execution cannot be substituted for successful
problem detection. It is a historical scan packet, not a current review
regression result or an independently labeled false-negative rate.

## Fresh audit of existing evidence

Running the current `phase0_evidence.py` against the primary checkout's local
assets returned:

- decision: `blocked`; closure: `open`;
- three protocols valid, three tracks pending labels, zero invalid tracks;
- real-history: six cases, 12 baseline observations, six review observations;
- differential: six cases, 36 baseline observations, 18 review observations;
- rule quality: six of 54 registered rules have default-profile evidence;
  25 labeled default findings across 11 snapshot repositories;
- independent real-history worksheets plus adjudication are missing;
- the paired differential pilot worksheet and metrics are not complete for
  the current collection.

The six-case `annotation-a-current.toml`, `annotation-b-current.toml`, and
`differential-pilot-current.toml` have empty case labels. A separate old
`model-pilot.toml` contains two `no-defect` labels and has a metrics artifact;
it is model-assisted exploratory work on that packet. It neither completes
the six-case collection nor supplies independent human assessments.

`blocked` here is the evidence report's closure decision. Product development
can proceed on setup, usability, offline behavior, and a clearly labeled
single-expert exploratory path. Broad independent quality claims remain
unavailable until the corresponding protocol is satisfied.

## Exact offline boundary

The sandbox manifest requires `network = "none"`. Docker commands include
`--network none` and `--pull=never`, and use bounded resources and a dedicated
workspace. This is an existing useful isolation mechanism for build/test
oracles.

The analyzer runs separately through a host command. Source preparation can
clone the pinned revision when no local source is supplied. Therefore these
packets do not prove that the complete analyzer process tree was tested under
a network deny policy, nor that a fresh machine can install all prerequisites
offline. The product plan adds those acceptance cases to existing validation
surfaces. Docker remains an engineering harness dependency, not a requirement
for ordinary CLI users.

## Reproduction pointers

Use the current scripts with an explicit asset checkout. These commands only
inspect manifests and saved artifacts; they do not rerun project workloads.
For an asset checkout that also has current scripts:

```bash
python3 scripts/sandbox.py check \
  --manifest .zoo/repopilot-validation/manifest.toml
python3 scripts/sandbox.py validate-pilot \
  --manifest .zoo/repopilot-validation/manifest-pilot.toml \
  --artifact .zoo/repopilot-validation/runs/technical-pilot-summary-after-images.json
python3 scripts/sandbox.py validate-mutation \
  --manifest .zoo/repopilot-validation/manifest-mutation.toml \
  --artifact .zoo/repopilot-validation/runs/express-mutation-summary.json
python3 scripts/phase0_evidence.py --root . \
  --evidence-dir .repopilot/evidence/v0.23 --format text
```

If scripts and assets are in different checkouts, pass absolute manifest and
artifact paths and set the evidence audit's `--root` to the asset checkout.
Record both code and asset identities. Do not infer success from a filename
such as `current`, from exit code zero alone, or from a schema-valid packet.

## Sources

- [Benchmark and sandbox usage](../../tests/benchmarks/README.md)
- [Sandbox runner](../../scripts/sandbox_runner.py)
- [Docker adapter and resource boundary](../../scripts/sandbox_process.py)
- [Differential preregistration](../../tests/benchmarks/differential.toml)
- [Evidence audit](../../scripts/phase0_evidence.py)
- [Historical evidence ledger](v0.23-evidence-ledger.md)
- [Current release evidence ledger](v0.24-evidence-ledger.md)
