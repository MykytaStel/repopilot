# Single-Expert Differential Pilot Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a validated single-expert exploratory scoring workflow over the existing baseline-aware differential artifact without weakening the preregistered dual-review protocol.

**Architecture:** Add one focused Python module for pilot worksheet rendering/validation and one for scoring. Extend the existing differential CLI with three pilot subcommands. Reuse the existing manifest and differential artifact validators, and keep all outputs explicitly exploratory and hash-pinned.

**Tech Stack:** Python 3.11 standard library (`tomllib`, `json`, `hashlib`, `statistics`, `unittest`), existing RepoPilot benchmark scripts, TOML/JSON artifacts, Markdown documentation.

**Spec:** `docs/superpowers/specs/2026-09-06-single-expert-pilot-design.md`

## Global Constraints

- Preserve `dual-independent-adjudication-v1` unchanged.
- Do not infer labels from RepoPilot findings, baseline failures, or merge status.
- Keep outputs deterministic and hash-pinned.
- Treat `single-expert-exploratory` results as descriptive evidence only.
- Keep source files below the repository's approximately 300-line limit.
- Do not commit generated `.repopilot/evidence` artifacts.

---

### Task 1: Define pilot worksheet contract

**Files:**
- Create: `scripts/differential_pilot.py`
- Test: `scripts/tests/test_differential_pilot.py`

**Interfaces:**
- Consumes: `validate_artifact`, `validate_manifest`, and a differential artifact path.
- Produces: `render_pilot_template(...) -> str`, `validate_pilot(...) -> dict[str, object]`, and `load_pilot(...) -> tuple[dict[str, object], dict[str, dict[str, object]]]`.

- [ ] **Step 1: Write the failing tests**

  Add tests that assert a rendered worksheet declares `protocol = "single-expert-pilot-v1"`, `blinded = true`, and contains no `in_diff_rule_ids`, `in_diff_evidence_keys`, or `novel_in_diff_evidence_keys`; assert that a completed label with a rationale validates; assert that an unknown rule, missing rationale, hash drift, or missing case raises `HoldoutManifestError`.

- [ ] **Step 2: Run the focused tests and verify RED**

  Run:

  ```bash
  python3 -m unittest scripts.tests.test_differential_pilot
  ```

  Expected result: import failure because `differential_pilot` does not yet exist.

- [ ] **Step 3: Implement the worksheet contract**

  Define `PILOT_SCHEMA_VERSION = 1`, `PILOT_PROTOCOL = "single-expert-pilot-v1"`, and allowed labels from `real_history_contract`. Load the differential artifact through `validate_artifact`, derive aggregate baseline statuses from the first baseline repetition, and render only pinned case identity plus blank label fields. Validate top-level hashes, `assessment_mode`, `blinded`, exact case identity, allowed labels, known rule IDs, and non-empty rationales.

- [ ] **Step 4: Run the focused tests and verify GREEN**

  Run the same unittest command and require all pilot contract tests to pass.

- [ ] **Step 5: Commit the contract slice**

  ```bash
  git add scripts/differential_pilot.py scripts/tests/test_differential_pilot.py
  git commit -m "feat: add single-expert pilot worksheet contract"
  ```

### Task 2: Add exploratory scoring

**Files:**
- Create: `scripts/differential_pilot_metrics.py`
- Modify: `scripts/tests/test_differential_pilot.py`

**Interfaces:**
- Consumes: validated pilot worksheet and validated differential artifact.
- Produces: `build_pilot_metrics(...) -> dict[str, object]` and `write_pilot_metrics(...) -> dict[str, object]`.

- [ ] **Step 1: Write failing scoring tests**

  Add a fixture with one no-defect case containing novel evidence, one no-defect case with none, and one uncertain case. Assert outcomes `fp`, `tn`, and `excluded`; assert recall is `None` when there are no positive cases; assert the output contains scope, limitation, input hashes, Wilson intervals, deterministic case count, and median timing fields.

- [ ] **Step 2: Run scoring tests and verify RED**

  Run:

  ```bash
  python3 -m unittest scripts.tests.test_differential_pilot.DifferentialPilotTests.test_scores_exploratory_outcomes
  ```

  Expected result: import failure for `differential_pilot_metrics`.

- [ ] **Step 3: Implement scoring**

  Convert each `novel_in_diff_evidence_keys` entry to its rule ID using the existing novelty identity format. Compute case outcomes using exact rule intersection, exclude `uncertain`, calculate TP/FN/TN/FP and Wilson intervals, and summarize only measurements present in the artifact: deterministic review cases, median baseline wall time, median review wall time, and median child RSS when available. Emit `scope = "single-expert exploratory pilot"` and a limitation that forbids independent or general claims.

- [ ] **Step 4: Run scoring tests and full script tests**

  ```bash
  python3 -m unittest scripts.tests.test_differential_pilot
  python3 -m unittest discover -s scripts/tests -p 'test_*.py'
  ```

- [ ] **Step 5: Commit scoring**

  ```bash
  git add scripts/differential_pilot_metrics.py scripts/tests/test_differential_pilot.py
  git commit -m "feat: score single-expert differential pilot"
  ```

### Task 3: Expose the pilot workflow through the differential CLI

**Files:**
- Modify: `scripts/differential.py`
- Modify: `scripts/tests/test_differential.py`

**Interfaces:**
- Consumes: `render_pilot_template`, `validate_pilot`, `write_pilot_metrics`.
- Produces: `pilot-template`, `pilot-validate`, and `pilot-score` commands with stable text/JSON summaries and exit codes.

- [ ] **Step 1: Add failing CLI tests**

  Assert missing required arguments return exit code 2, a valid template command writes a worksheet, and pilot-score writes a JSON metrics artifact.

- [ ] **Step 2: Run CLI tests and verify RED**

  ```bash
  python3 -m unittest scripts.tests.test_differential.DifferentialContractTests.test_pilot_commands
  ```

  Expected result: argparse rejects `pilot-template` because the command is not yet registered.

- [ ] **Step 3: Implement CLI branches**

  Add the three command choices and arguments `--pilot`, `--reviewer`, and `--output`. Validate the differential artifact before each branch, catch `HoldoutManifestError` and `DifferentialManifestError`, create output parent directories, and print a compact status in text mode or the returned summary in JSON mode.

- [ ] **Step 4: Run CLI and script tests**

  ```bash
  python3 -m unittest scripts.tests.test_differential scripts.tests.test_differential_pilot
  python3 scripts/differential.py check --format json
  ```

- [ ] **Step 5: Commit CLI integration**

  ```bash
  git add scripts/differential.py scripts/tests/test_differential.py
  git commit -m "feat: expose single-expert pilot commands"
  ```

### Task 4: Run the pilot on the collected holdout

**Files:**
- Generate ignored artifacts under: `.repopilot/evidence/v0.23/`

- [ ] **Step 1: Generate the blinded worksheet**

  ```bash
  python3 scripts/differential.py pilot-template \
    --artifact .repopilot/evidence/v0.23/differential-run-v2.json \
    --reviewer model-assisted \
    --output .repopilot/evidence/v0.23/model-pilot.toml
  ```

- [ ] **Step 2: Fill the model-assisted assessment explicitly**

  Record the provisional `no-defect` labels and rationales from the existing assessment file, while retaining `assessment_mode = "single-expert-exploratory"`.

- [ ] **Step 3: Validate and score the pilot**

  ```bash
  python3 scripts/differential.py pilot-validate \
    --artifact .repopilot/evidence/v0.23/differential-run-v2.json \
    --pilot .repopilot/evidence/v0.23/model-pilot.toml
  python3 scripts/differential.py pilot-score \
    --artifact .repopilot/evidence/v0.23/differential-run-v2.json \
    --pilot .repopilot/evidence/v0.23/model-pilot.toml \
    --output .repopilot/evidence/v0.23/model-pilot-metrics.json
  ```

- [ ] **Step 4: Inspect the result**

  Confirm that the report says exploratory, has zero positive cases, and does not claim recall or utility beyond the two-case holdout.

### Task 5: Document the larger evidence workflow

**Files:**
- Modify: `tests/benchmarks/README.md`
- Modify: `docs/engineering/v0.23-evidence-ledger.md`
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Add usage documentation**

  Document the three pilot commands, the blank-field workflow, and the exact limitation that one reviewer does not establish independent validation.

- [ ] **Step 2: Update the evidence ledger**

  Record the pilot as exploratory evidence, keep RP23-014 open, and state that the differential utility claim still requires a larger labeled corpus and independent review.

- [ ] **Step 3: Run documentation and release checks**

  ```bash
  python3 scripts/release-contract.py check
  git diff --check
  ```

- [ ] **Step 4: Commit documentation**

  ```bash
  git add tests/benchmarks/README.md docs/engineering/v0.23-evidence-ledger.md CHANGELOG.md
  git commit -m "docs: describe single-expert pilot evidence boundary"
  ```

### Task 6: Full verification and draft PR

**Files:**
- No additional source changes expected.

- [ ] **Step 1: Run the complete local gate**

  ```bash
  python3 -m unittest discover -s scripts/tests -p 'test_*.py'
  python3 -m compileall -q scripts
  python3 scripts/release-contract.py check
  cargo fmt --all -- --check
  cargo clippy --all-targets --all-features -- -D warnings
  cargo run --quiet -- scan . --fail-on-priority p1
  npm run review:performance
  git diff --check
  ```

- [ ] **Step 2: Inspect the diff and generated report boundary**

  Confirm generated artifacts remain ignored, no dual-review validator changed, and every metric carries the exploratory limitation.

- [ ] **Step 3: Push the feature branch**

  ```bash
  git push -u origin feat/single-expert-pilot
  ```

- [ ] **Step 4: Open one draft PR**

  ```bash
  gh pr create --draft --base main --head feat/single-expert-pilot \
    --title "feat: add single-expert differential pilot" \
    --body-file /tmp/repopilot-single-expert-pilot-pr.md
  ```
