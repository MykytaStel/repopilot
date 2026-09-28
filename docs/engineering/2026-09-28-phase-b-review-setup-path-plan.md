# Phase B Review Setup Path Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make reviews with missing verification setup show one valid path from generated suggestions to an explicitly selected check, and document how to reach a verified review.

**Architecture:** Keep `next_action_for` as the canonical source for the setup action so all report projections inherit the same text. Preserve proof and verdict rules; the user reviews suggestions, copies accepted checks into active config, and explicitly runs one check by its ID.

**Tech Stack:** Rust 2024, Clap CLI, TOML configuration, Markdown docs, Cargo integration tests.

**Spec:** `docs/engineering/v0.24-phase-b-review-setup-path-spec.md`

**Product context:** P01 in the [Q4 product plan](../roadmap/product-plan-2026-q4.uk.md).
The [sandbox baseline](2026-09-28-sandbox-product-baseline.md) identifies the
existing validation assets for subsequent slices. This implementation plan
continues to cover the bounded setup path.

## Global Constraints

- Do not run suggested commands, install dependencies, or change active configuration automatically.
- Do not change verification policy semantics, verdict derivation, or the priority of review findings and sensitive signals.
- Do not add a CLI command or implement new stack detectors.
- Keep generated suggestions separate from `repopilot.toml`; only user-selected entries become active checks.
- Keep the first-screen action concise and share it across existing review projections.

## Review Focus

- Unavailable required checks use the exact suggestions-file path and an explicit `--verify CHECK_ID` next step; pin in `next_action_names_unavailable_required_checks` and `review_first_screen`.
- A missing proof policy with no applicable checks produces the same setup action; pin in `next_action_explains_missing_policy_when_no_checks_apply`.
- A failed required check continues to outrank setup guidance; preserve `next_action_names_failed_required_checks`.
- P0/P1 or sensitive human-review evidence continues to outrank setup guidance; preserve `high_priority_evidence_outranks_unavailable_checks_in_next_action` and `human_review_evidence_outranks_policy_setup_in_next_action`.
- Suggestions stay inactive until copied, and a configured check proves nothing until it passes; cover the separate output path in `init_exports_reviewable_toml_without_applying_it_to_config` and the pass transition in `configured_passing_check_reaches_verified`.

---

### Task 1: Canonical setup action and projection coverage

**Files:**
- Modify: `src/review/proof/next_action.rs`
- Test: `src/review/proof_tests.rs`
- Test: `tests/review_first_screen.rs`
- Test: `tests/review_projection_parity.rs`
- Test: `tests/review_html.rs`
- Test: `tests/review_readiness.rs`
- Test: `tests/mcp_change_proof.rs`
- Test: `tests/action_delta.rs`

**Interfaces:**
- Consumes: existing `next_action_for(&ChangeProof) -> &'static str` and canonical `decision.next_action` projection.
- Produces: one shared action string for unavailable-check and missing-policy setup cases; no public API changes.

- [x] **Step 1: Pin both setup cases to one exact action.** Update the existing unavailable-check and missing-policy unit tests to expect: `Run repopilot init --suggestions-output .repopilot/init-suggestions.toml, review its suggestions, copy accepted checks into repopilot.toml, then run repopilot review . --verify CHECK_ID (replace CHECK_ID with an accepted ID).` Also assert both cases return the same string. Use the plain CHECK_ID placeholder so the instruction remains visible in Markdown and HTML; the docs give a runnable concrete ID.
- [x] **Step 2: Run the focused unit tests and confirm they fail on the old generic command.** Run: `cargo test next_action_names_unavailable_required_checks` and `cargo test next_action_explains_missing_policy_when_no_checks_apply`. Expected: both fail because the output uses `repopilot-suggestions.toml` and omits the accepted check ID.
- [x] **Step 3: Implement the shared action in `src/review/proof/next_action.rs`.** Reuse one constant from both setup branches; leave failed-check, high-priority evidence, stale-check, unselected-check, and human-review ordering unchanged.
- [x] **Step 4: Pin first-screen and shared projection text.** Extend the existing first-screen and projection tests to assert the suggestions path, config-copy instruction, and `--verify CHECK_ID` appear in the canonical decision and human projections. Update hard-coded HTML/readiness expectations; assert MCP and Action summaries preserve the canonical `decision.next_action` when it is present.
- [x] **Step 5: Run the focused projection tests.** Run: `cargo test --test review_first_screen`, `cargo test --test review_projection_parity`, `cargo test --test review_html`, `cargo test --test review_readiness`, `cargo test --test mcp_change_proof`, and `cargo test --test action_delta`. Expected: all pass and the rendered next action matches the canonical decision text.

### Task 2: First-review walkthrough and release evidence

**Files:**
- Modify: `docs/configuration.md`
- Modify: `CHANGELOG.md` under `[Unreleased]`
- Modify: `docs/engineering/v0.24-evidence-ledger.md` in `RP24-008`
- Modify: `docs/engineering/v0.24-phase-b-review-setup-path-spec.md`
- Test: `tests/init_suggestions.rs` (extend the existing non-mutation case if needed)
- Test: `tests/review_first_screen.rs` (confirm the accepted check ID is explicit)

**Interfaces:**
- Consumes: the action from Task 1 and existing `repopilot init --suggestions-output` behavior.
- Produces: a local walkthrough that ends with an explicit check ID and explains that `VERIFIED` requires a passing selected check.

- [x] **Step 1: Add the short walkthrough to `docs/configuration.md`.** Show `repopilot review .`, generate `.repopilot/init-suggestions.toml`, inspect it, copy an accepted `[[verification.checks]]` entry into `repopilot.toml`, then run `repopilot review . --verify rust.test` for the Rust example or replace `rust.test` with the accepted entry's ID. State that an unpassed or unavailable check leaves the review unverified and that other proof limits may still require action.
- [x] **Step 2: Strengthen the output/config regression if needed.** Make `tests/init_suggestions.rs::init_exports_reviewable_toml_without_applying_it_to_config` use `.repopilot/init-suggestions.toml` and prove the active config remains separate from the suggestions file.
- [x] **Step 3: Record the user-facing change and evidence.** Add one `[Unreleased]` changelog bullet; update RP24-008 with the setup-path behavior and actual focused-test evidence; set the spec status to `in-progress` while implementing and `done` only after acceptance checks pass.
- [x] **Step 4: Run the setup and documentation-adjacent tests.** Run: `cargo test --test init_suggestions` and `cargo test --test review_first_screen`. Expected: both pass; generated suggestions remain separate and explicit verification is required.

### Task 3: Handoff verification

**Files:**
- Inspect: `git diff --check`, `git status`, and the completed diff.
- Verify: focused tests from Tasks 1–2 and the RepoPilot local pre-handoff gate from `source-command-rp-gate`.

- [x] **Step 1: Review the diff against the spec.** Confirm no policy/verdict behavior changed and failed checks/high-priority evidence retain precedence.
- [x] **Step 2: Run the RepoPilot local pre-handoff gate.** Report local gate results separately from hosted CI and zoo evidence; `.zoo/` availability must be checked before making any zoo claim.
- [ ] **Step 3: Commit the verified feature-branch diff, then run `cargo run --release -- review . --base origin/main`.** Ref-based dogfood ignores the uncommitted working tree, so run it after the patch is committed. Read the first screen; confirm the command uses the exact suggestions path when setup guidance applies and never promises `VERIFIED` before a selected check passes.
