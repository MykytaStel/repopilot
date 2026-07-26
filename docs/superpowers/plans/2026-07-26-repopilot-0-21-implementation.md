# RepoPilot 0.21 Risk Memory and Merge Readiness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a local-first RepoPilot 0.21 contract that reports compatible
risk deltas, ownership-aware merge readiness, trustworthy overlay behavior, and
deeper framework coverage in the existing language frontends.

**Architecture:** Introduce pure canonical history, ownership, and readiness
records below the output layer. Keep persistence and projection as adapters so
analysis remains deterministic and side-effect free until explicit history
recording. Extend language-owned tables with differential safe/unsafe tests,
without adding engine-level language dispatch.

**Tech Stack:** Rust 2024, serde/serde_json, clap, sha2, globset, tree-sitter,
existing RepoPilot scan/review/MCP/Action contracts, Python release scripts.

## Global Constraints

- Analysis remains local-only, deterministic, and network-free.
- No new top-level CLI command.
- History is opt-in and creates no files while disabled.
- JSON, MCP, Action, and AI-context changes are additive.
- Files stay at or below 300 lines and functions at or below 50 lines.
- Finding history uses occurrence identity; baseline matching stays on stable
  finding IDs.
- Default-profile precision and strict-mode recall are preserved.
- No commit, push, tag, or publication without explicit user authorization.

---

### Task 1: Fix Overlay Tracking and Cache Invalidation

**Files:**
- Modify: `.gitignore`
- Modify: `src/commands/mcp/scan_cache.rs`
- Modify: `src/commands/mcp/scan_cache/git.rs`
- Modify: `src/commands/mcp/scan_cache/tests/invalidation.rs`
- Modify: `tests/cli_stabilization.rs`

**Interfaces:**
- Produces: `git::is_generated_repopilot_path(rel: &str) -> bool`
- Produces: cache-key control input for `.repopilot/overlay.toml`
- Preserves: generated `.repopilot/cache/` and `.repopilot/history/` exclusion

- [x] **Step 1: Add failing cache-key tests**

```rust
#[test]
fn overlay_changes_invalidate_but_generated_state_does_not() {
    let (_dir, root) = init_repo();
    let overlay = root.join(".repopilot/overlay.toml");
    std::fs::create_dir_all(overlay.parent().unwrap()).unwrap();
    std::fs::write(&overlay, "[[overlay]]\nrule = \"a\"\n").unwrap();
    let first = cache_key(&root, &args(&root)).unwrap();

    std::fs::write(&overlay, "[[overlay]]\nrule = \"b\"\n").unwrap();
    let second = cache_key(&root, &args(&root)).unwrap();
    assert_ne!(first, second);

    std::fs::create_dir_all(root.join(".repopilot/history")).unwrap();
    std::fs::write(root.join(".repopilot/history/runs.jsonl"), "{}\n").unwrap();
    assert_eq!(second, cache_key(&root, &args(&root)).unwrap());
}
```

- [x] **Step 2: Run the focused test and confirm RED**

Run:
`cargo test --lib commands::mcp::scan_cache::tests::invalidation::overlay_changes_invalidate_but_generated_state_does_not`

Expected: overlay edit leaves the key unchanged with the current broad
`.repopilot` exclusion.

- [x] **Step 3: Narrow generated-state filtering and hash overlay content**

Implement exact generated path matching for `.repopilot/cache` and
`.repopilot/history`. Add `.repopilot/overlay.toml` beside feedback and ignore
controls in the cache-key material.

- [x] **Step 4: Make the overlay trackable**

Add `!/.repopilot/overlay.toml` after the broad `/.repopilot/*` rule. Keep
history and cache ignored.

- [x] **Step 5: Run targeted checks**

Run:
`cargo test --lib commands::mcp::scan_cache`

Run:
`cargo test --test cli_stabilization`

Expected: both pass and overlay/history behavior is pinned.

---

### Task 2: Replace the Prototype History Model with a Compatible Receipt Contract

**Files:**
- Rewrite: `src/history/model.rs`
- Rewrite: `src/history/delta.rs`
- Modify: `src/history/mod.rs`
- Test: `src/history/delta.rs`
- Test: `tests/history_contract.rs`

**Interfaces:**
- Produces:
  `RunReceiptV1 { schema_version, comparison, recorded_at, revision, findings }`
- Produces:
  `ComparisonIdentity { workspace, target, scope, base, head, profile, config_fingerprint, overlay_fingerprint, analysis_schema }`
- Produces:
  `RiskDelta { comparison, new, persisting, resolved, severity_shifts }`
- Produces:
  `compare(current: &RunReceiptV1, prior: &RunReceiptV1) -> ComparisonResult`

- [x] **Step 1: Add failing occurrence-collision and compatibility tests**

```rust
#[test]
fn distinct_occurrences_with_one_baseline_id_are_not_merged() {
    let prior = receipt("full", vec![finding("same-id", "occ-a", Medium)]);
    let current = receipt(
        "full",
        vec![
            finding("same-id", "occ-a", Medium),
            finding("same-id", "occ-b", High),
        ],
    );
    let delta = compare(&current, &prior).compatible().unwrap();
    assert_eq!(delta.persisting.len(), 1);
    assert_eq!(delta.new.len(), 1);
}

#[test]
fn changed_scope_never_resolves_full_scope_occurrences() {
    let full = receipt("full", vec![finding("a", "occ-a", High)]);
    let changed = receipt("changed", Vec::new());
    assert!(matches!(
        compare(&changed, &full),
        ComparisonResult::Unavailable(ComparisonUnavailable::ScopeMismatch)
    ));
}
```

- [x] **Step 2: Run the history unit tests and confirm RED**

Run: `cargo test --lib history::delta`

Expected: the prototype uses `stable_finding_key`, has no compatibility result,
and cannot represent persisting occurrences.

- [x] **Step 3: Implement versioned pure data records**

Use owned, serializable enums and structs. Sort occurrence records by
occurrence key, then rule/path, before storing or comparing. Keep timestamp out
of `ComparisonIdentity`.

- [x] **Step 4: Implement compatibility-first delta computation**

Return an explicit unavailable reason before classifying findings. Compare
occurrence-key maps without overwriting duplicate baseline IDs.

- [x] **Step 5: Verify the model**

Run: `cargo test --lib history`

Run: `cargo test --test history_contract`

Expected: collision, ordering, severity-shift, and mismatch tests pass.

---

### Task 3: Add Safe, Bounded History Storage

**Files:**
- Rewrite: `src/history/storage.rs`
- Create: `src/history/diagnostic.rs`
- Modify: `src/history/mod.rs`
- Modify: `src/config/model.rs`
- Modify: `src/config/template.rs`
- Test: `src/history/storage.rs`
- Test: `tests/config_loader.rs`

**Interfaces:**
- Produces:
  `HistoryStore::new(workspace_root: &Path, limits: HistoryLimits) -> Self`
- Produces:
  `HistoryStore::load() -> HistoryLoad`
- Produces:
  `HistoryStore::newest_compatible(&ComparisonIdentity) -> CompatibleRun`
- Produces:
  `HistoryStore::record(&RunReceiptV1) -> Result<(), HistoryWriteError>`
- Produces:
  `HistoryLimits { max_runs: usize, max_bytes: u64 }`
- Produces: structured `HistoryDiagnostic`

- [x] **Step 1: Add failing storage tests**

Cover:

```rust
#[test]
fn disabled_history_configuration_is_the_default() {
    assert!(!HistorySection::default().enabled);
}

#[test]
fn pruning_keeps_complete_json_lines_within_both_limits() {
    // Record three receipts under max_runs=2 and a small byte cap.
    // Assert two parseable newest receipts remain and file size is bounded.
}

#[test]
fn truncated_tail_is_reported_without_losing_valid_receipts() {
    // Write one valid line and one truncated line.
    // Assert one receipt plus HistoryDiagnostic::TruncatedRecord.
}
```

- [x] **Step 2: Run focused tests and confirm RED**

Run: `cargo test --lib history::storage`

Run: `cargo test --test config_loader history`

- [x] **Step 3: Implement atomic bounded replacement**

Read valid receipts, append in memory, prune oldest by count and serialized byte
size, write a sibling staged file, flush, then rename. Reuse the repository's
recoverable staged-replacement pattern where possible.

- [x] **Step 4: Surface corruption and write diagnostics**

Do not silently discard parsing or IO failures. Return valid records with
diagnostics; a recording failure does not change analysis success.

- [x] **Step 5: Verify storage and config**

Run: `cargo test --lib history`

Run: `cargo test --test config_loader`

Expected: storage, zero/one limits, corruption, and default-off behavior pass.

---

### Task 4: Integrate Explicit History Recording into Scan and Review

**Files:**
- Modify: `src/cli/options/scan.rs`
- Modify: `src/cli/options/review.rs`
- Modify: `src/commands/product_scan.rs`
- Modify: `src/commands/scan.rs`
- Modify: `src/commands/review.rs`
- Create: `src/history/context.rs`
- Test: `tests/history_cli.rs`
- Modify: `tests/cli_stabilization.rs`

**Interfaces:**
- Produces: `--record-history` on `scan` and `review`
- Produces:
  `build_scan_receipt(session, mode, findings) -> RunReceiptV1`
- Produces:
  `build_review_receipt(session, review_input, profile, findings) -> RunReceiptV1`
- Removes: history side effects from `run_product_scan`

- [x] **Step 1: Add failing CLI integration tests**

```rust
#[test]
fn scan_does_not_create_history_without_opt_in() {
    let repo = fixture_repo();
    repopilot().args(["scan", repo.path_str()]).assert().success();
    assert!(!repo.path().join(".repopilot/history").exists());
}

#[test]
fn compatible_recorded_scans_show_a_delta() {
    let repo = fixture_repo();
    run_scan(&repo, &["--record-history"]);
    introduce_finding(&repo);
    let output = run_scan(&repo, &["--record-history", "--format", "json"]);
    assert_eq!(output["risk_delta"]["comparison"]["status"], "compatible");
}
```

- [x] **Step 2: Confirm RED**

Run: `cargo test --test history_cli`

- [x] **Step 3: Remove prototype recording from `run_product_scan`**

Keep product scan deterministic. Commands decide whether to load/record history
after filtering and after review scope/base/head are known.

- [x] **Step 4: Add explicit recording and diagnostics**

Enable recording when `--record-history` or `[history].enabled = true`. Use
`session.workspace_root()` for storage and the analysis path only as receipt
target metadata.

- [x] **Step 5: Verify CLI behavior**

Run: `cargo test --test history_cli`

Run: `cargo test --test cli_stabilization`

Run: `cargo test --test scan_command_cli`

Expected: default scans are side-effect free and compatible runs expose deltas.

---

### Task 5: Build Deterministic CODEOWNERS and Boundary Ownership

**Files:**
- Create: `src/review/ownership/mod.rs`
- Create: `src/review/ownership/codeowners.rs`
- Create: `src/review/ownership/model.rs`
- Modify: `src/review/mod.rs`
- Test: `src/review/ownership/codeowners.rs`
- Test: `tests/review_ownership.rs`

**Interfaces:**
- Produces:
  `OwnershipIndex::discover(root: &Path) -> OwnershipDiscovery`
- Produces:
  `OwnershipIndex::owners_for(path: &Path) -> Vec<Owner>`
- Produces:
  `OwnershipSummary::for_impact(changed: &[ChangedFile], impact: &ImpactPaths, index: &OwnershipIndex) -> Self`

- [x] **Step 1: Add failing CODEOWNERS precedence tests**

```rust
#[test]
fn last_matching_rule_wins_and_owners_are_deduplicated() {
    let index = parse("* @all\n/src/ @backend\n/src/auth/ @security @backend\n");
    assert_eq!(
        index.owners_for(Path::new("src/auth/session.rs")),
        vec![owner("@security"), owner("@backend")]
    );
}
```

Also cover discovery precedence `.github/CODEOWNERS`, root `CODEOWNERS`,
`docs/CODEOWNERS`, comments, escaped spaces, deleted paths, and no-file
fallback boundaries.

- [x] **Step 2: Confirm RED**

Run: `cargo test --lib review::ownership`

- [x] **Step 3: Implement parser and matcher**

Keep parsing deterministic and GitHub-compatible for supported syntax. Emit an
explicit limitation diagnostic for unsupported constructs instead of silently
guessing.

- [x] **Step 4: Implement package/directory fallback**

Use existing workspace/package facts when available; otherwise return the
first stable top-level directory boundary. Never invent a username or team.

- [x] **Step 5: Verify ownership**

Run: `cargo test --lib review::ownership`

Run: `cargo test --test review_ownership`

Expected: precedence, discovery, fallback, and deterministic ordering pass.

---

### Task 6: Derive the Canonical Merge Readiness Record

**Files:**
- Create: `src/review/readiness/mod.rs`
- Create: `src/review/readiness/model.rs`
- Create: `src/review/readiness/derive.rs`
- Modify: `src/review/model.rs`
- Modify: `src/review/report.rs`
- Modify: `src/output/decision_summary.rs`
- Test: `src/review/readiness/derive.rs`
- Test: `tests/review_readiness.rs`

**Interfaces:**
- Produces:
  `MergeReadinessRecord { verdict, reasons, impact, ownership, verification, limitations, risk_delta }`
- Produces: `ReadinessVerdict::{Ready, Review, Blocked}`
- Produces: stable `ReadinessReasonCode`
- Produces:
  `derive_readiness(report, gates, ownership, delta) -> MergeReadinessRecord`

- [x] **Step 1: Add failing verdict tests**

```rust
#[test]
fn failed_gate_is_blocked_with_stable_reason_code() {
    let readiness = derive_readiness(&fixture_with_failed_gate());
    assert_eq!(readiness.verdict, ReadinessVerdict::Blocked);
    assert_eq!(readiness.reasons[0].code, ReadinessReasonCode::FindingGateFailed);
}

#[test]
fn unowned_impacted_surface_requires_review() {
    let readiness = derive_readiness(&fixture_with_unowned_impact());
    assert_eq!(readiness.verdict, ReadinessVerdict::Review);
}
```

- [x] **Step 2: Confirm RED**

Run: `cargo test --lib review::readiness`

- [x] **Step 3: Implement pure deterministic derivation**

Precedence is `blocked > review > ready`. Sort reason codes, paths, owners, and
verification steps deterministically. Use shared redaction for any
human-readable evidence.

- [x] **Step 4: Make the legacy decision summary a projection**

Map `Ready/Review/Blocked` to existing `PASS/REVIEW/BLOCK` labels so console
compatibility and exit behavior remain stable.

- [x] **Step 5: Verify readiness**

Run: `cargo test --lib review::readiness`

Run: `cargo test --test review_readiness`

Run: `cargo test --test review_command`

Expected: verdict precedence, ownership reasons, verification, and legacy
summary compatibility pass.

---

### Task 7: Project Readiness and Risk Delta Across Product Surfaces

**Files:**
- Modify: `src/report/schema/review.rs`
- Modify: `src/review/render/console.rs`
- Modify: `src/review/render/markdown.rs`
- Modify: `src/review/render/json.rs`
- Modify: `src/review/render/sarif.rs`
- Modify: `src/commands/mcp/review_change.rs`
- Modify: `src/commands/mcp/context.rs`
- Modify: `src/output/ai_context/json.rs`
- Modify: `scripts/review_delta.py`
- Modify: `scripts/repopilot-action-review.sh`
- Test: `tests/review_readiness.rs`
- Test: `tests/mcp_cli.rs`
- Test: `tests/action_delta.rs`
- Test: `tests/ai_context_golden.rs`
- Test: `tests/report_schema_contract.rs`

**Interfaces:**
- Consumes: `MergeReadinessRecord` and `RiskDelta`
- Produces: additive `merge_readiness` and optional `risk_delta` fields
- Preserves: existing schema readers, exit codes, and Action artifacts

- [ ] **Step 1: Add failing projection parity tests**

Assert the same fixture has identical verdict, reason-code set, owner set, and
verification count in JSON, MCP compact/full, Action delta metadata, and AI
context.

- [ ] **Step 2: Confirm RED**

Run:
`cargo test --test review_readiness --test mcp_cli --test action_delta --test ai_context_golden`

- [ ] **Step 3: Add shared-record projections**

Render human labels from stable reason codes. Keep SARIF additions under result
properties or invocation metadata without changing existing result identity.

- [ ] **Step 4: Update additive schema compatibility fixtures**

Advance the report schema only if required by repository policy, retain all
accepted older versions, and prove strict older readers ignore new fields.

- [ ] **Step 5: Verify all projections**

Run:
`cargo test --test review_readiness --test mcp_cli --test action_delta --test ai_context_golden --test report_schema_contract`

Expected: parity and compatibility tests pass.

---

### Task 8: Deepen Managed-Language Framework Coverage

**Files:**
- Modify: `src/languages/java/review.rs`
- Modify: `src/languages/kotlin/review.rs`
- Modify: `src/languages/csharp/review.rs`
- Test: `src/review/signals/taint/tests.rs`
- Create: managed-language cases under `tests/fixtures/review-zoo/taint/`
- Modify: `tests/review_zoo.rs`

**Interfaces:**
- Extends: language-owned `TaintTables`
- Adds: Spring MVC/Boot, Ktor/Spring, and ASP.NET Core source/sink idioms
- Preserves: language-neutral taint engine

- [ ] **Step 1: Add one failing unsafe and one passing safe case per framework**

Unsafe cases pass request-derived values to raw SQL, process, or file sinks.
Safe cases use constants, parameterized APIs, or a supported sanitizer.

- [ ] **Step 2: Confirm RED only for unsafe cases**

Run: `cargo test --lib review::signals::taint`

Run: `cargo test --test review_zoo managed`

- [ ] **Step 3: Add minimal high-specificity frontend patterns**

Do not add global method-name matches. Bind source and sink idioms to the owning
frontend grammar and framework vocabulary.

- [ ] **Step 4: Verify managed languages**

Run: `cargo test --lib review::signals::taint`

Run: `cargo test --test review_zoo`

Expected: unsafe cases emit expected signals and safe cases remain empty.

---

### Task 9: Deepen TypeScript/JavaScript, Python, and Go Coverage

**Files:**
- Modify: `src/languages/javascript/review.rs`
- Modify: `src/languages/python/review.rs`
- Modify: `src/languages/go/review.rs`
- Test: `src/review/signals/taint/tests.rs`
- Create: framework cases under `tests/fixtures/review-zoo/taint/`
- Modify: `tests/review_zoo.rs`

**Interfaces:**
- Adds: Next.js App Router, Express, Fastify/Hono, FastAPI/Django/Flask,
  `net/http`/Gin/Echo/Fiber idioms

- [ ] **Step 1: Add differential fixture tests**

Cover at least one request-to-database/process/file flow and one near-identical
safe flow for every newly claimed framework family.

- [ ] **Step 2: Confirm RED**

Run: `cargo test --lib review::signals::taint`

- [ ] **Step 3: Extend frontend-owned tables conservatively**

Prefer qualified call shapes and framework-specific access paths. Do not treat
generic identifiers such as `query`, `body`, `exec`, or `write` as sufficient
on their own.

- [ ] **Step 4: Verify dynamic and Go frontends**

Run: `cargo test --lib review::signals::taint`

Run: `cargo test --test review_zoo`

Expected: every unsafe/safe pair behaves as specified.

---

### Task 10: Improve Rust Web Boundaries and Panic-Risk Precision

**Files:**
- Modify: `src/languages/rust/review.rs`
- Modify focused modules under: `src/audits/code_quality/rust_panic_risk/`
- Test: `src/review/signals/tests.rs`
- Test focused modules under: `src/audits/code_quality/rust_panic_risk/`
- Create: Rust cases under `tests/fixtures/review-zoo/boundary/`

**Interfaces:**
- Adds: Axum, Actix, and Rocket boundary evidence
- Preserves: dedicated Rust panic-risk capability and no generic taint claim

- [ ] **Step 1: Add failing web-boundary cases and panic-risk FP guards**

Unsafe/review-worthy cases change handler/auth boundaries. Safe cases change
ordinary internal functions. Panic-risk guards cover idiomatic infallible or
test-only contexts currently at risk of noise.

- [ ] **Step 2: Confirm RED**

Run: `cargo test --lib languages::rust`

Run: `cargo test --lib rust_panic_risk`

- [ ] **Step 3: Add focused patterns and contextual guards**

Keep Rust panic-risk in its dedicated audit. Extend frontend boundary metadata
without creating a taint capability.

- [ ] **Step 4: Verify Rust quality**

Run: `cargo test --lib languages::rust`

Run: `cargo test --lib rust_panic_risk`

Run: `cargo test --test review_zoo`

Expected: new boundary true positives and panic-risk false-positive guards pass.

---

### Task 11: Synchronize Product, Engineering, and Release Documentation

**Files:**
- Modify: `docs/roadmap.md`
- Modify: `docs/roadmap/v0.20.md`
- Modify: `docs/engineering/analysis-platform-state.md`
- Modify: `docs/engineering/language-surface-inventory.md`
- Modify generated: `docs/language-support.md`
- Modify: `docs/configuration.md`
- Modify: `docs/commands.md`
- Modify: `CHANGELOG.md` only within `[Unreleased]`
- Create or modify: `docs/releases/v0.21.0.md`
- Modify: `specs/001-021-risk-memory-merge-readiness/spec.md`

**Interfaces:**
- Produces: one exact release promise and status across all documentation
- Marks: v0.20 release preparation done
- Marks: 0.21 spec status `Implemented` only after verified implementation

- [ ] **Step 1: Add or update documentation contract tests**

Pin history defaults/options, overlay tracking, readiness fields, language
matrix generation, and v0.20 completion status in existing doc/CLI contract
tests.

- [ ] **Step 2: Confirm documentation drift is detected**

Run:
`cargo test --test cli_stabilization --test language_support_doc`

Run:
`python3 scripts/release-contract.py check`

- [ ] **Step 3: Update current-state and user workflow docs**

Lead with the user outcome: record risk, review readiness, inspect owners and
verification, then use Action/MCP projections. Remove stale graph/language
checklist claims.

- [ ] **Step 4: Update release notes and changelog**

Describe user-visible outcomes, compatibility, opt-in history, language depth,
and limitations. Do not claim zoo measurements unless freshly verified.

- [ ] **Step 5: Regenerate and verify docs**

Run:
`REPOPILOT_BLESS=1 cargo test --test language_support_doc`

Run:
`cargo test --test cli_stabilization`

Run:
`python3 scripts/release-contract.py check`

Expected: generated and curated docs agree.

---

### Task 12: Full 0.21 Verification and Spec Closure

**Files:**
- Modify only if evidence requires:
  `specs/001-021-risk-memory-merge-readiness/spec.md`
- Modify only if evidence requires:
  `docs/releases/v0.21.0.md`

**Interfaces:**
- Consumes: all tasks above
- Produces: release-gate evidence without publishing

- [ ] **Step 1: Run formatting and lint gates**

Run: `cargo fmt --all -- --check`

Run: `cargo clippy --all-targets --all-features -- -D warnings`

- [ ] **Step 2: Run the complete test and contract gates**

Run: `cargo test --all`

Run: `python3 scripts/release-contract.py check`

Run: `npm run test:release-contract`

- [ ] **Step 3: Run performance and self-scan gates**

Run: `npm run review:performance`

Run: `cargo run -- scan . --fail-on-priority p1`

- [ ] **Step 4: Run zoo evidence when available**

Run: `python3 scripts/zoo.py scan`

If `.zoo/` is unavailable, record that fact and omit measured zoo claims.

- [ ] **Step 5: Run release verification**

Run: `bash scripts/verify-release.sh`

Do not bump, tag, publish, commit, or push.

- [ ] **Step 6: Close the spec from evidence**

Change the tracked spec status from `Approved` to `Implemented` only when every
acceptance scenario and applicable success criterion has a passing test or
documented gate result.
