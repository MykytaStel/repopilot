# RepoPilot quality expansion: implementation scope

**Status:** first implementation slice and verification complete  
**Baseline:** `origin/main` at `05015bf5c84eaabe49e866b30e1761bfdb8a45dc`

## Goal

Improve the first-use experience and close concrete MCP/plugin trust gaps while
using repository and PR evidence to choose future detector work. Keep MCP's
machine-readable report stable while making its ordinary text block useful to a
person reading it in an agent client.

## Included work

1. Render MCP scan, change-review, and explain `content[].text` as concise
   Markdown. Keep machine-readable reports in `structuredContent`, and keep the
   full JSON report in RepoPilot's analysis store for pagination and explain
   tools. Bound incoming MCP request lines to 1 MiB.
2. Confine `intent_path` to the configured MCP root, like other caller-supplied
   paths.
3. Reject symlinked `.repopilot` directories and snapshot files before Claude
   Code or Cursor session-start hooks invalidate or write session state; protect
   direct `repopilot snapshot` calls too. Gemini's documented hook setup reuses
   the protected Claude scripts; Copilot uses the protected CLI command.
4. Label AI Context's parser coverage and broader fact inventory as separate
   scopes so their file counts cannot be mistaken for the same denominator.
5. Record the evidence-driven detector backlog: test assertion trivialization
   is a candidate with a concrete missed case, but requires a real-diff fixture
   and justified rewrite controls before implementation. Keep Promise `.catch()`
   gated on independent labels and a repository-held-out evaluation.
6. Trace weakened matchers in modified, named-export JS/TS helpers to unchanged
   tests that directly import and call those helpers. Resolve the exact module
   path and imported symbol; keep re-exports, default/namespace/dynamic imports,
   indirect calls, and multi-hop flows outside this slice.
7. Attribute clean, uniquely matched error handlers moved between changed files
   to their source file. For Rust, also match a moved `match Result` handler to
   `?` propagation when the same Result-producing operation remains. Changed
   handlers and ambiguous destinations remain visible.

## Acceptance checks

- MCP scan/review/explain text starts with a readable summary; tests prove
  `structuredContent` still contains the machine-readable report and analysis
  handles still replay the full report. Oversized input is rejected and the
  server continues processing subsequent requests.
- `intent_path` escaping the configured MCP root is rejected before reading.
- Claude Code and Cursor session-start and direct snapshot calls do not write
  through symlinked state paths. Gemini reuses the protected Claude scripts.
- AI Context names analyzed-file counts and inventoried-file counts explicitly.
- No detector accuracy claim or Promise `.catch()` rule is added from the
  exploratory, single-reviewer corpus.
- CLI and MCP both suppress the exact cross-file handler move, while a changed
  handler body, a different Rust Result-producing operation, and duplicate
  destination handlers leave the removal visible. Re-reviewing the historical
  MCP refactor diff no longer emits the publication or transport move signals.

## Evidence limits and next evaluation

The 0.25.0 holdout has 98 PRs but only two reconciled weakened-change labels,
and one reviewer; it is exploratory rather than an independent accuracy
benchmark. Its strongest concrete follow-up is `ih-083`, where an assertion
after a component unmount cannot observe the behavior, paired with the existing
positive guard `ih-084` and justified rewrite controls such as `ih-003` and
`ih-082`. Build a pinned changed-line fixture set from those diffs before
changing the detector.

The Promise discovery pool is not labeled. Do not promote its hunk-match counts
to candidate or accuracy counts. The `.catch()` pilot proceeds only after
independent harm/control adjudication, a frozen repository split, and the
minimum held-out denominators already specified in
[`promise-catch-pilot-design.md`](promise-catch-pilot-design.md).

This slice improves deterministic checks and their evaluation evidence. It does
not train an LLM or claim that the current corpora are suitable training data.

## Verification record

The full Rust suite and 405-test Python scripts suite passed. After the final
small MCP module and schema-builder refactors, the relevant binary tests,
`mcp_cli`, and MCP parity tests were rerun and passed; `cargo clippy
--all-targets --all-features -- -D warnings`, formatting, and the 0.25.0
release-contract check also passed. The release-contract check ran the local
zoo scan.

The latest `review --since-snapshot --detail full` reported zero in-diff
findings across 27 analyzed files, with 13 test/fixture/generated files skipped
by policy. Its verdict remains `REVIEW`: no proof policy or formal verification
checks were selected. It also raised two gate-eligible
`behavioral.error-handling-removed` signals at the old MCP transport and
publication files. The follow-up matcher now accounts for exact handler moves
and Rust `match Result` to `?` extraction when the Result-producing call is
preserved. Re-reviewing the historical refactor diff removes those two signals;
an unrelated `integrity/gates/ci.rs` signal remains. This is a narrow
file-local mitigation, not a semantic proof or a precision/recall estimate:
identical code copied into another changed file can resemble a move. Ambiguous
destinations and changed operations remain reportable. Other report noise in
the historical diff includes imports of existing dependencies in split modules
and a test-only file write.

The weakened-helper and moved-handler follow-ups are covered by targeted
fixtures and CLI/MCP parity tests. The helper implementation considers direct
named value exports and imports in one hop, ignores fixture/test-data callers,
and attributes only unchanged tests with a direct helper call. This establishes
the supported mechanism and its boundaries; it is not a production precision
or recall estimate. The full Rust suite, Clippy, formatting, and hosted Rust,
CodeQL, security/maintenance, and Linux/macOS/Windows checks passed on the
follow-up PR.
