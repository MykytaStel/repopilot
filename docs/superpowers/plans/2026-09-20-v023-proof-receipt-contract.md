# v0.23 Proof Receipt Contract Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an additive, deterministic Proof Receipt to every review machine projection, with explicit replay compatibility states, while preserving all 0.22 fields and exit codes.

**Architecture:** `ChangeProof` remains the only verdict engine. A focused `ProofReceipt` projection in `src/review/proof/receipt.rs` will capture the proof, evidence class, coverage, obligations, reason codes, next action, provenance hashes, and replay state. JSON and SARIF will publish the same serialized receipt; the existing human renderers will consume the same receipt in the following slice, so no renderer can invent a verdict.

**Tech Stack:** Rust 2024, serde/serde_json, SHA-256 via the existing `sha2` dependency, Cargo integration tests, existing review fixtures and MCP projection tests.

**Spec:** `docs/superpowers/specs/2026-09-20-v023-proof-receipts-design.md`

## Global Constraints

- No version bump, `v0.23.0` tag, package publication, or release workflow run during the feature phase.
- No new top-level CLI command.
- Existing `ChangeProof`, readiness fields, JSON fields, and exit codes remain backward compatible.
- A receipt mismatch or unsupported schema must never become `VERIFIED`.
- Receipt content is local-only, root-confined, deterministic, bounded, escaped, and free of raw source uploads.
- `Finding::id` is not an occurrence identity; receipt tests must keep distinct evidence locations distinct.
- All production code follows red → green → refactor; each behavior has a failing test before implementation.

## Review Focus

- A receipt generated from the same proof and inputs must serialize byte-for-byte identically; the receipt hash test owns this case.
- A changed workspace revision or configuration must return `stale`, never `matched`; replay tests own both mismatches.
- An unknown receipt schema and malformed JSON must return structured `unsupported` or `invalid`, without panicking; replay tests own both cases.
- Empty, partially covered, and unavailable reviews must carry their existing `NOT ASSESSED`/`REVIEW` proof and explicit receipt state; receipt-construction tests own these cases.
- Two findings with the same stable ID but different paths/lines must remain two evidence entries; projection parity tests own this case.

---

### Task 1: Define the receipt and replay compatibility model

**Files:**
- Create: `src/review/proof/receipt.rs`
- Modify: `src/review/proof.rs:8-18` to export the receipt types and builder
- Test: `src/review/proof/receipt_tests.rs`

**Interfaces:**
- Consumes: `ChangeProof`, `EvidenceSummary`, `ReviewReport`, the existing `canonical_json_hash`, `REPOPILOT_VERSION`, and `SCAN_REPORT_SCHEMA_VERSION`.
- Produces: `ProofReceipt`, `ReceiptReplayState`, `ReceiptReplayContext`, `build_proof_receipt`, and `replay_receipt` for the projection and schema layers.

- [ ] **Step 1: Write failing receipt model tests**

Add tests in `src/review/proof/receipt_tests.rs` for the public behavior:

```rust
#[test]
fn receipt_keeps_verdict_scope_reasons_obligations_and_next_action() {
    let (report, proof) = review_and_proof(ChangeProofVerdict::Review);
    let receipt = build_proof_receipt(&report, &proof);

    assert_eq!(receipt.proof.verdict, ChangeProofVerdict::Review);
    assert_eq!(receipt.proof.coverage, proof.coverage);
    assert_eq!(receipt.proof.obligations, proof.obligations);
    assert_eq!(receipt.next_action, "Review the listed evidence, close the proof limits, or run the required checks.");
    assert!(receipt.projection_hash.starts_with("sha256:"));
}

#[test]
fn receipt_hash_is_stable_when_collection_inputs_are_reordered() {
    let (mut first_report, proof) = review_and_proof(ChangeProofVerdict::Verified);
    let first = build_proof_receipt(&first_report, &proof);
    first_report.changed_files.reverse();
    let second = build_proof_receipt(&first_report, &proof);

    assert_eq!(first.projection_hash, second.projection_hash);
}
```

Use the existing `ReviewReport` fixture pattern from `src/review/proof/evidence_tests.rs`; do not add a second production verdict helper.

- [ ] **Step 2: Run the focused test and verify the expected failure**

Run:

```bash
cargo test --lib review::proof::receipt_tests -- --nocapture
```

Expected: compilation fails because `ProofReceipt`, `build_proof_receipt`, and the replay types do not exist yet.

- [ ] **Step 3: Implement the minimal receipt model**

Define the following serializable types in `src/review/proof/receipt.rs`:

```rust
pub const PROOF_RECEIPT_SCHEMA_VERSION: &str = "0.1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReceiptReplayState { Matched, Stale, Unsupported, Invalid, Unavailable }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptReplayContext {
    pub workspace_revision: Option<String>,
    pub configuration_hash: Option<String>,
    pub analyzer_version: Option<String>,
    pub report_schema: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProofReceipt {
    pub schema_version: &'static str,
    pub analyzer_version: String,
    pub report_schema: String,
    pub workspace_revision: String,
    pub configuration_hash: String,
    pub replay_state: ReceiptReplayState,
    pub projection_hash: String,
    pub proof: ChangeProof,
    pub evidence: EvidenceSummary,
    pub reason_codes: Vec<String>,
    pub next_action: String,
    pub unavailable_inputs: Vec<String>,
}
```

`build_proof_receipt` must:

1. derive `EvidenceSummary` from the existing proof;
2. capture the local workspace revision with `WorkspaceRevision::capture(&report.repo_root).id()`;
3. compute `configuration_hash` from a canonical object containing review mode, visibility profile, base ref, configured/selected verification IDs, and the current report schema;
4. derive a deterministic `projection_hash` from the receipt payload excluding `replay_state` and `projection_hash` itself;
5. sort/deduplicate reason-code strings and unavailable inputs;
6. set `replay_state` to `Matched` for a freshly built receipt.

`replay_receipt` must compare all four context fields. Return:

- `Matched` when every recorded field is present and equal;
- `Unavailable` when the caller cannot supply a required context field;
- `Unsupported` when schema or analyzer version is not accepted;
- `Stale` when revision, configuration, or report schema differs;
- `Invalid` when the projection hash does not match a recomputation.

Use a fixed accepted schema list containing `PROOF_RECEIPT_SCHEMA_VERSION`; do not silently accept future schemas.

- [ ] **Step 4: Run the focused tests and refactor only after green**

Run:

```bash
cargo test --lib review::proof::receipt_tests -- --nocapture
cargo fmt --all -- --check
```

Expected: all receipt tests pass and formatting is clean. Refactor only naming or duplication while keeping the same assertions, then rerun the focused tests.

- [ ] **Step 5: Commit the model**

```bash
git add src/review/proof.rs src/review/proof/receipt.rs src/review/proof/receipt_tests.rs
git commit -m "feat: add replayable proof receipt model"
```

### Task 2: Publish one receipt in JSON and SARIF

**Files:**
- Modify: `src/report/schema/review.rs:1-150` to add the receipt field from the same proof instance
- Modify: `src/review/render/sarif.rs:75-96` to attach the same serialized receipt
- Test: `tests/review_proof_receipt.rs`
- Modify: `tests/review_projection_parity.rs:1-110` to assert receipt parity

**Interfaces:**
- Consumes: `build_proof_receipt`, the existing `ReviewJsonReport::from_report`, and SARIF run properties.
- Produces: additive JSON field `proof_receipt` and SARIF run property `proofReceipt`, with identical JSON values.

- [ ] **Step 1: Write failing projection tests**

Add an end-to-end fixture that creates a temporary Git repository, runs `repopilot review --format json --sarif-output`, and asserts:

```rust
assert_eq!(json["proof_receipt"], sarif["runs"][0]["properties"]["proofReceipt"]);
assert_eq!(json["proof_receipt"]["proof"], json["change_proof"]);
assert_eq!(json["proof_receipt"]["replay_state"], "matched");
assert!(json["proof_receipt"]["unavailable_inputs"].is_array());
```

Also add two findings with the same stable ID at different evidence locations to the fixture and assert the receipt retains both evidence entries in the detailed proof payload.

- [ ] **Step 2: Run the test and verify it fails**

Run:

```bash
cargo test --test review_proof_receipt -- --nocapture
```

Expected: the test fails because `proof_receipt` and `proofReceipt` are absent.

- [ ] **Step 3: Add the additive JSON/SARIF fields**

In `ReviewJsonReport`, compute `proof` once, then build `proof_receipt` from that exact proof. Keep `change_proof` and `evidence` unchanged. In SARIF, attach `serde_json::to_value(receipt)?` under `run.properties.proofReceipt` while retaining the existing `changeProof` and `evidence` properties.

- [ ] **Step 4: Run targeted projection and MCP parity tests**

Run:

```bash
cargo test --test review_proof_receipt -- --nocapture
cargo test --test review_projection_parity -- --nocapture
cargo test --test mcp_change_proof -- --nocapture
```

Expected: all pass; JSON, SARIF, and stored MCP projections retain the existing fields and agree on the new receipt.

- [ ] **Step 5: Commit the projections**

```bash
git add src/report/schema/review.rs src/review/render/sarif.rs tests/review_proof_receipt.rs tests/review_projection_parity.rs
git commit -m "feat: publish proof receipts in review projections"
```

### Task 3: Add bounded replay validation tests and public API documentation

**Files:**
- Modify: `src/review/proof/receipt.rs` to expose a strict replay validator and bounded error code
- Test: `src/review/proof/receipt_tests.rs`
- Modify: `docs/engineering/v0.23-evidence-ledger.md` with the receipt schema and replay contract
- Modify: `CHANGELOG.md` under `[Unreleased]`

**Interfaces:**
- Consumes: the receipt serialized by Task 2.
- Produces: deterministic validation behavior for `matched`, `stale`, `unsupported`, `invalid`, and `unavailable`.

- [ ] **Step 1: Write failing replay tests**

Add tests for each state:

```rust
#[test]
fn replay_rejects_revision_change_as_stale() { /* change workspace_revision */ }
#[test]
fn replay_rejects_configuration_change_as_stale() { /* change configuration_hash */ }
#[test]
fn replay_rejects_future_schema_as_unsupported() { /* schema_version = "9.0" */ }
#[test]
fn replay_rejects_tampered_payload_as_invalid() { /* change next_action */ }
#[test]
fn replay_reports_missing_context_as_unavailable() { /* context fields are None */ }
```

Each test must assert the exact enum state and verify that no code path changes the embedded proof verdict.

- [ ] **Step 2: Run the replay tests and verify the expected failures**

Run:

```bash
cargo test --lib review::proof::receipt_tests::replay -- --nocapture
```

Expected: the new tests fail before the validator is complete.

- [ ] **Step 3: Implement strict validation and bounded errors**

Keep validation pure and local. Do not execute commands, read arbitrary paths, or accept a future schema. Return the enum state plus a bounded reason string for diagnostics; cap any rendered diagnostic at 256 bytes and escape it in human output.

- [ ] **Step 4: Update docs and changelog**

Document the fields, hash input boundary, replay states, and explicit limitations. State that the receipt is evidence of the recorded analysis inputs, not proof of runtime behavior or human usefulness.

- [ ] **Step 5: Run the focused gate and commit**

```bash
cargo test --lib review::proof::receipt_tests -- --nocapture
cargo test --test review_proof_receipt -- --nocapture
python3 scripts/release-contract.py check

git add src/review/proof/receipt.rs src/review/proof/receipt_tests.rs docs/engineering/v0.23-evidence-ledger.md CHANGELOG.md
git commit -m "docs: specify proof receipt replay contract"
```

### Task 4: Whole-slice verification and handoff evidence

**Files:**
- Modify: `tests/review_projection_parity.rs` only if a regression assertion is needed
- Create: `docs/superpowers/evidence/2026-09-20-v023-proof-receipt-slice.md`

**Interfaces:**
- Consumes: the completed receipt model and projection tests.
- Produces: a bounded local evidence record; it does not claim release readiness or human usefulness.

- [ ] **Step 1: Run the relevant Rust checks**

```bash
cargo test --lib review::proof::receipt_tests -- --nocapture
cargo test --test review_proof_receipt -- --nocapture
cargo test --test review_projection_parity -- --nocapture
cargo test --test mcp_change_proof -- --nocapture
cargo test --test review_readiness -- --nocapture
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

- [ ] **Step 2: Run the existing self-scan smoke check**

```bash
cargo run --quiet -- scan . --fail-on-priority p1 --format json --output /tmp/repopilot-v023-proof-receipt-p1.json --no-progress
```

Record the exit status, finding count, and any unresolved-import diagnostic without converting it into a release claim.

- [ ] **Step 3: Write the evidence record**

Record commit SHAs, commands, pass/fail results, receipt schema version, parity checks, known unavailable inputs, and the fact that package version/tag/release were unchanged.

- [ ] **Step 4: Commit the evidence record**

```bash
git add docs/superpowers/evidence/2026-09-20-v023-proof-receipt-slice.md
git commit -m "docs: record proof receipt slice evidence"
```

## Subsequent plans required before 0.23 candidate review

This slice does not close the complete 0.23 risk ledger. After it passes, create and review separate plans for:

1. human Proof Card parity across console/Markdown/HTML/MCP/Action/AI context;
2. contract packs and the 6/54 rule-evidence gap;
3. sandbox runner, mutation oracles, unavailable test identity, and independent labels;
4. advisory/opt-in CI rollout with rollback evidence;
5. platform install, dependency advisory resolution, publication recovery, and any operator study.

The version remains `0.22.0` until those plans produce sufficient evidence and a separate release decision is approved.
